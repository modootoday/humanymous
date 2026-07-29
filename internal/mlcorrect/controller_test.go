package mlcorrect

import "testing"

func TestController_FireThresholdStartsAtTheta0(t *testing.T) {
	c := NewController(0.01, 0.5, 0.02)
	if got := c.FireThreshold(); got != 0.5 {
		t.Fatalf("fresh controller must serve θ0=0.5, got %v", got)
	}
}

func TestController_CalibrationRaisesThresholdOnOverFlaggedHumans(t *testing.T) {
	c := NewController(0.01, 0.5, 0.05)
	start := c.FireThreshold()
	// oracle-confirmed humans the model scores as bots (pBot high) → θ must rise to protect them.
	for i := 0; i < 300; i++ {
		c.ObserveOutcome(OutcomePassSolved, 0.95, false)
	}
	if c.FireThreshold() <= start {
		t.Fatalf("over-flagged humans must raise θ: start=%v now=%v", start, c.FireThreshold())
	}
	snap := c.Snapshot()
	if snap.Calibration.HumansSeen != 300 {
		t.Fatalf("calibration must have counted 300 humans, got %d", snap.Calibration.HumansSeen)
	}
}

func TestController_AmbiguousDoesNotCalibrate(t *testing.T) {
	c := NewController(0.01, 0.5, 0.05)
	for i := 0; i < 100; i++ {
		c.ObserveOutcome(OutcomeUnknown, 0.99, true)
	}
	if c.FireThreshold() != 0.5 {
		t.Fatalf("ambiguous outcomes must not move θ, got %v", c.FireThreshold())
	}
	if s := c.Snapshot(); s.Calibration.HumansSeen != 0 || s.Calibration.BotsSeen != 0 {
		t.Fatalf("ambiguous outcomes must not be counted as labels: %+v", s.Calibration)
	}
}

func TestController_DriftFiresOnStuddPlusCovariate(t *testing.T) {
	c := NewController(0.01, 0.5, 0.02)
	// STUDD: model and frozen engine agree for a while (low mimic-loss)...
	for i := 0; i < 400; i++ {
		c.ObserveOutcome(OutcomeCatalogBot, 0.95, true) // model says bot, engine says bot → agree
	}
	// ...then the model diverges from the engine (mimic-loss rises).
	for i := 0; i < 400; i++ {
		c.ObserveOutcome(OutcomeCatalogBot, 0.02, true) // model says human, engine still says bot → disagree
	}
	// add covariate shift → 2-of-3 gate should fire.
	c.UpdateCovariate(0.9)
	ev := c.Snapshot().Drift
	if !ev.MimicAlarm {
		t.Fatal("sustained student/teacher divergence must raise the STUDD mimic alarm")
	}
	if !ev.CovariateShift {
		t.Fatal("a large PSI must register covariate shift")
	}
	if !ev.Fired {
		t.Fatalf("two agreeing drift arms must fire the gate: %+v", ev)
	}
}

func TestController_PassSolveAnomalySurfaces(t *testing.T) {
	c := NewController(0.01, 0.5, 0.02)
	if c.Snapshot().PassAnomalous {
		t.Fatal("baseline must not be anomalous")
	}
	for i := 0; i < 5000; i++ {
		c.ObservePass(true) // adversaries solving Pass at ~100%
	}
	if !c.Snapshot().PassAnomalous {
		t.Fatal("a sustained Pass solve-rate spike must surface as anomalous (poisoning guard)")
	}
}

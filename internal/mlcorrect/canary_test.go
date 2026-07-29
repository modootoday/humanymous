package mlcorrect

import (
	"sync/atomic"
	"testing"
)

func TestCanaryGuard_TripsOnHumanFPBreach(t *testing.T) {
	var rolled atomic.Int32
	var gotReason atomic.Value
	g := NewCanaryGuard(CanaryBudget{MaxHumanFP: 0.05, ProbationHumans: 1000}, func(r string) {
		rolled.Add(1)
		gotReason.Store(r)
	})
	g.Arm(0)
	// below the min-observe window: even a high FP must NOT trip yet.
	g.evaluate(minCanaryObserve-1, 0.9, false, false)
	if rolled.Load() != 0 {
		t.Fatal("must not trip before the min-observe window")
	}
	// past the window with FP over budget → trip exactly once.
	g.evaluate(minCanaryObserve+5, 0.9, false, false)
	g.evaluate(minCanaryObserve+6, 0.9, false, false) // already tripped → no second callback
	if rolled.Load() != 1 {
		t.Fatalf("rollback must fire exactly once, got %d", rolled.Load())
	}
	if s := g.Snapshot(minCanaryObserve + 6); s.State != "tripped" || s.Reason == "" {
		t.Fatalf("state must be tripped with a reason, got %+v", s)
	}
}

func TestCanaryGuard_GraduatesBreachFree(t *testing.T) {
	var rolled atomic.Int32
	g := NewCanaryGuard(CanaryBudget{MaxHumanFP: 0.05, ProbationHumans: 100}, func(string) { rolled.Add(1) })
	g.Arm(0)
	// healthy FP for the whole probation window → graduate, never roll back.
	for i := uint64(1); i <= 100; i++ {
		g.evaluate(i, 0.0, false, false)
	}
	if rolled.Load() != 0 {
		t.Fatalf("a healthy model must never roll back, got %d", rolled.Load())
	}
	if s := g.Snapshot(100); s.State != "graduated" {
		t.Fatalf("must graduate after a breach-free probation, got %q", s.State)
	}
	// post-graduation observations are inert.
	g.evaluate(200, 0.9, true, true)
	if rolled.Load() != 0 {
		t.Fatal("a graduated guard must not trip")
	}
}

func TestCanaryGuard_TripsOnDriftAndPoisoning(t *testing.T) {
	var rolled atomic.Int32
	g := NewCanaryGuard(CanaryBudget{MaxHumanFP: 0.9, ProbationHumans: 1000, RollbackOnDrift: true}, func(string) { rolled.Add(1) })
	g.Arm(0)
	g.evaluate(5, 0.0, true, false) // drift fired → trip even inside the min-observe window
	if rolled.Load() != 1 {
		t.Fatalf("drift must trip the canary, got %d", rolled.Load())
	}

	var rolled2 atomic.Int32
	g2 := NewCanaryGuard(CanaryBudget{MaxHumanFP: 0.9, ProbationHumans: 1000, RollbackOnPassAnomaly: true}, func(string) { rolled2.Add(1) })
	g2.Arm(0)
	g2.evaluate(5, 0.0, false, true) // pass anomaly → trip
	if rolled2.Load() != 1 {
		t.Fatalf("pass anomaly must trip the canary, got %d", rolled2.Load())
	}
}

func TestController_ArmCanary_RollsBackOnOverFlagging(t *testing.T) {
	var rolled atomic.Int32
	c := NewController(0.005, 0.5, 0.0) // gamma 0 → θ frozen at 0.5 so the model keeps flagging
	c.ArmCanary(CanaryBudget{MaxHumanFP: 0.10, ProbationHumans: 100000}, func(string) { rolled.Add(1) })
	// feed confirmed humans the model scores as bots (pBot=0.9 ≥ θ) → realized human-FP climbs past 0.10.
	for i := 0; i < 400; i++ {
		c.ObserveOutcome(OutcomePassSolved, 0.9, false)
	}
	if rolled.Load() == 0 {
		t.Fatalf("sustained human over-flagging must trip the armed canary (fp est=%.3f)", c.Snapshot().Calibration.HumanFPEst)
	}
	if s := c.Snapshot().Canary; s.State != "tripped" {
		t.Fatalf("controller snapshot must report the canary tripped, got %q", s.State)
	}
}

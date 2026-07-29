package mlcorrect

import "testing"

func TestPageHinkley_DetectsRise(t *testing.T) {
	ph := NewPageHinkley(0.01, 3)
	// stable low-error stream: no alarm.
	for i := 0; i < 500; i++ {
		if ph.Update(0.02) {
			t.Fatalf("false alarm on stable stream at %d", i)
		}
	}
	// concept drift: error jumps up and stays.
	fired := false
	for i := 0; i < 500; i++ {
		if ph.Update(0.5) {
			fired = true
			break
		}
	}
	if !fired {
		t.Fatal("Page-Hinkley failed to detect a sustained rising shift")
	}
}

func TestPSI_StableVsShifted(t *testing.T) {
	ref := []float64{0.25, 0.25, 0.25, 0.25}
	if p := PSI(ref, ref); p > 1e-9 {
		t.Fatalf("identical distributions must have PSI ~0, got %v", p)
	}
	shifted := []float64{0.7, 0.2, 0.07, 0.03}
	if p := PSI(ref, shifted); p < 0.25 {
		t.Fatalf("a large shift must exceed the 0.25 significant-shift threshold, got %v", p)
	}
}

func TestDriftMonitor_2of3Gate(t *testing.T) {
	d := NewDriftMonitor()
	// Only covariate shift (1 of 3) → must NOT fire.
	d.UpdatePSI(0.9)
	if d.Gate().Fired {
		t.Fatal("a single alarm (covariate only) must not fire the 2-of-3 gate")
	}
	// Add a sustained mimic-loss rise (2 of 3) → must fire.
	for i := 0; i < 1000; i++ {
		d.ObserveMimic(0.02)
	}
	for i := 0; i < 1000; i++ {
		d.ObserveMimic(0.8)
	}
	ev := d.Gate()
	if !ev.MimicAlarm {
		t.Fatal("mimic detector should have alarmed on the sustained rise")
	}
	if !ev.Fired {
		t.Fatalf("two agreeing alarms (mimic + covariate) must fire the gate: %+v", ev)
	}
}

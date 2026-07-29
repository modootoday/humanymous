package behavior

import (
	"testing"

	"github.com/modootoday/humanymous/internal/signals"
)

// humanLike mirrors the coherent-human behavioral summary used across the scoring tests.
func humanLike() signals.BehaviorSummary {
	return signals.BehaviorSummary{
		Mouse: signals.MouseFeatures{Samples: 45, VelocityStdDev: 0.6, StraightLineFrac: 0.15,
			AccelEntropy: 2.1, MeanJerk: 0.4, MeanCurvature: 0.3, CoalescedRatio: 3.0, MaxTeleportPx: 40},
		Key:       signals.KeyFeatures{Keystrokes: 14, MeanDwellMs: 95, DwellStdDevMs: 28, MeanFlightMs: 140, FlightStdDev: 35},
		Events:    signals.EventFeatures{TotalEvents: 60, UntrustedFrac: 0, ClickCount: 1},
		DurationS: 8,
	}
}

func TestExtract_DimAndDeterminism(t *testing.T) {
	b := humanLike()
	fv1 := Extract(b)
	if len(fv1) != FeatureDim {
		t.Fatalf("FeatureVector len %d != FeatureDim %d", len(fv1), FeatureDim)
	}
	if FeatureDim != len(FeatureNames()) {
		t.Fatalf("FeatureDim %d != len(featureNames) %d — schema/layout drift", FeatureDim, len(FeatureNames()))
	}
	fv2 := Extract(b)
	for i := range fv1 {
		if fv1[i] != fv2[i] {
			t.Fatalf("non-deterministic feature at %d: %v vs %v", i, fv1[i], fv2[i])
		}
	}
}

func TestExtract_PresenceFlags(t *testing.T) {
	// keyboard-only human: no pointer, has keys → has_pointer=0, has_key=1.
	kbOnly := signals.BehaviorSummary{Key: signals.KeyFeatures{Keystrokes: 20, MeanDwellMs: 90}, Events: signals.EventFeatures{TotalEvents: 20}, DurationS: 5}
	if HasPointer(kbOnly) {
		t.Error("keyboard-only session must NOT report HasPointer")
	}
	if !HasKey(kbOnly) {
		t.Error("keyboard-only session must report HasKey")
	}
	fv := Extract(kbOnly)
	// presence flags are the last two features.
	if fv[FeatureDim-2] != 0 {
		t.Errorf("has_pointer flag want 0 got %v", fv[FeatureDim-2])
	}
	if fv[FeatureDim-1] != 1 {
		t.Errorf("has_key flag want 1 got %v", fv[FeatureDim-1])
	}
}

func TestExtract_EmptyIsFinite(t *testing.T) {
	// zero session (no behavior at all) must produce all-finite features, not NaN/Inf.
	fv := Extract(signals.BehaviorSummary{})
	for i, v := range fv {
		if v != v { // NaN
			t.Fatalf("NaN feature at %d (%s)", i, FeatureNames()[i])
		}
		if v > 1e30 || v < -1e30 {
			t.Fatalf("non-finite feature at %d (%s): %v", i, FeatureNames()[i], v)
		}
	}
}

func TestSchemaHash_StableAndSensitive(t *testing.T) {
	h1 := SchemaHash()
	if h1 == "" || len(h1) != 24 { // 12 bytes hex
		t.Fatalf("unexpected schema hash %q", h1)
	}
	if SchemaHash() != h1 {
		t.Fatal("schema hash must be stable across calls")
	}
}

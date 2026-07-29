package mlserve

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/modootoday/humanymous/internal/behavior"
)

// validBundle builds a well-formed bundle with the given single-hidden-unit weights over the real
// feature dimension, mean=0 / std=1 (identity standardization), identity calibration.
func validBundle(w1 []float32, w2, b2 float32) MLPBundle {
	d := behavior.FeatureDim
	mean := make([]float32, d)
	std := make([]float32, d)
	for i := range std {
		std[i] = 1
	}
	return MLPBundle{
		Version: "test", SchemaHash: behavior.SchemaHash(), FeatureDim: d, Hidden: 1,
		Mean: mean, Std: std, W1: [][]float32{w1}, B1: []float32{0}, W2: []float32{w2}, B2: b2,
		CalA: 1, CalB: 0,
	}
}

func TestMLP_ForwardMath(t *testing.T) {
	d := behavior.FeatureDim
	w1 := make([]float32, d)
	w1[0] = 2 // hidden = relu(2 * x0)
	m := &MLP{b: validBundle(w1, 1, 0)}   // logit = relu(2*x0)
	fv := make(behavior.FeatureVector, d) // all zero
	if got := m.Predict(fv); got.Abstain || abs(got.PBot-0.5) > 1e-4 {
		t.Fatalf("zero input → logit 0 → p 0.5, got %+v", got)
	}
	fv[0] = 3 // relu(6)=6 → sigmoid(6) ≈ 0.9975
	if got := m.Predict(fv); got.Abstain || got.PBot < 0.99 {
		t.Fatalf("x0=3 → p≈0.9975, got %+v", got)
	}
}

func TestMLP_Validate_Rejects(t *testing.T) {
	good := validBundle(make([]float32, behavior.FeatureDim), 1, 0)
	// wrong featureDim
	bad := good
	bad.FeatureDim = behavior.FeatureDim + 1
	if err := bad.validate(); err == nil {
		t.Error("featureDim mismatch must be rejected")
	}
	// wrong W1 width
	bad2 := good
	bad2.W1 = [][]float32{make([]float32, behavior.FeatureDim-1)}
	if err := bad2.validate(); err == nil {
		t.Error("W1 width mismatch must be rejected")
	}
	// missing schema hash
	bad3 := good
	bad3.SchemaHash = ""
	if err := bad3.validate(); err == nil {
		t.Error("missing schemaHash must be rejected")
	}
}

func TestMLP_LoadRoundTrip(t *testing.T) {
	b := validBundle(make([]float32, behavior.FeatureDim), 1, 0)
	dir := t.TempDir()
	path := filepath.Join(dir, "m.json")
	raw, _ := json.Marshal(b)
	if err := os.WriteFile(path, raw, 0o644); err != nil {
		t.Fatal(err)
	}
	m, err := LoadMLP(path)
	if err != nil {
		t.Fatalf("LoadMLP: %v", err)
	}
	if m.SchemaHash() != behavior.SchemaHash() {
		t.Error("loaded schema hash mismatch")
	}
	// a bundle that loads must plug into the seam and score without abstaining (schema matches).
	Set(m)
	defer Set(nil)
	if got := Score(make(behavior.FeatureVector, behavior.FeatureDim)); got.Abstain {
		t.Fatal("loaded matching-schema bundle must score, not abstain")
	}
}

func abs(f float32) float32 {
	if f < 0 {
		return -f
	}
	return f
}

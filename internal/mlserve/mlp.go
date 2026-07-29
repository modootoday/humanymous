package mlserve

import (
	"encoding/json"
	"fmt"
	"math"
	"os"

	"github.com/modootoday/humanymous/internal/behavior"
)

// mlp.go is the FIRST behavioral model implementation (SoT-42 Pillar A): a compact multilayer
// perceptron served entirely in pure Go — no cgo, no ONNX, no Python runtime at the edge. It fits
// the "small, policy-specific model, not a giant LLM" goal: a one-hidden-layer net over the ~27-dim
// aggregate behavioral feature vector, trained offline by cmd/ml-train into a JSON bundle. When the
// aggregate model plateaus, a sequence model (TCN/ONNX) can slot in behind the same Predictor
// interface without touching the scoring core.

// MLPBundle is the on-disk model artifact: standardization stats + one hidden layer + a Platt
// calibration on the logit, plus the provenance that pins it to a feature schema. All slices are
// row-major.
type MLPBundle struct {
	Version    string      `json:"version"`    // semver-ish bundle id, e.g. "mlp-20260729-1"
	SchemaHash string      `json:"schemaHash"` // behavior.SchemaHash() at train time (stale-model guard)
	FeatureDim int         `json:"featureDim"`
	Hidden     int         `json:"hidden"`
	Mean       []float32   `json:"mean"` // per-feature standardization
	Std        []float32   `json:"std"`
	W1         [][]float32 `json:"w1"` // [Hidden][FeatureDim]
	B1         []float32   `json:"b1"` // [Hidden]
	W2         []float32   `json:"w2"` // [Hidden]
	B2         float32     `json:"b2"` // scalar
	CalA       float32     `json:"calA"` // Platt: p = sigmoid(calA*logit + calB)
	CalB       float32     `json:"calB"`
}

// MLP is a loaded, validated, immutable predictor.
type MLP struct{ b MLPBundle }

// LoadMLP reads and validates a bundle file. It fails (rather than silently mis-predicting) on any
// shape inconsistency so a corrupt/foreign bundle can never feed garbage into the score.
func LoadMLP(path string) (*MLP, error) {
	raw, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var b MLPBundle
	if err := json.Unmarshal(raw, &b); err != nil {
		return nil, fmt.Errorf("mlserve: bundle parse: %w", err)
	}
	if err := b.validate(); err != nil {
		return nil, err
	}
	return &MLP{b: b}, nil
}

func (b MLPBundle) validate() error {
	if b.FeatureDim != behavior.FeatureDim {
		return fmt.Errorf("mlserve: bundle featureDim %d != engine %d", b.FeatureDim, behavior.FeatureDim)
	}
	if b.Hidden <= 0 || len(b.W1) != b.Hidden || len(b.B1) != b.Hidden || len(b.W2) != b.Hidden {
		return fmt.Errorf("mlserve: bundle hidden-layer shape invalid")
	}
	if len(b.Mean) != b.FeatureDim || len(b.Std) != b.FeatureDim {
		return fmt.Errorf("mlserve: bundle standardization shape invalid")
	}
	for i, row := range b.W1 {
		if len(row) != b.FeatureDim {
			return fmt.Errorf("mlserve: bundle W1 row %d width %d != %d", i, len(row), b.FeatureDim)
		}
	}
	if b.SchemaHash == "" {
		return fmt.Errorf("mlserve: bundle missing schemaHash")
	}
	return nil
}

func sigmoid(x float32) float32 { return float32(1.0 / (1.0 + math.Exp(float64(-x)))) }

// Predict runs the forward pass. Any shape mismatch or non-finite result abstains rather than
// emitting a bogus probability.
func (m *MLP) Predict(fv behavior.FeatureVector) Prediction {
	b := m.b
	if len(fv) != b.FeatureDim {
		return Abstained()
	}
	// standardize
	x := make([]float32, b.FeatureDim)
	for i := range fv {
		s := b.Std[i]
		if s == 0 {
			s = 1
		}
		x[i] = (fv[i] - b.Mean[i]) / s
	}
	// hidden = relu(W1 x + b1)
	var logit float32 = b.B2
	for h := 0; h < b.Hidden; h++ {
		acc := b.B1[h]
		row := b.W1[h]
		for i := 0; i < b.FeatureDim; i++ {
			acc += row[i] * x[i]
		}
		if acc < 0 {
			acc = 0 // ReLU
		}
		logit += b.W2[h] * acc
	}
	p := sigmoid(b.CalA*logit + b.CalB)
	if p != p || p < 0 || p > 1 { // NaN / out of range guard
		return Abstained()
	}
	return Prediction{PBot: p}
}

func (m *MLP) SchemaHash() string    { return m.b.SchemaHash }
func (m *MLP) BundleVersion() string { return m.b.Version }

// Package mlserve is the inference SEAM for the behavioral model (SoT-42 Pillar A). It is the
// single quarantine boundary between the pure-Go scoring core and any native ML runtime (ONNX/cgo):
// internal/signals and internal/scoring depend only on this interface, never on a model runtime,
// so the scoring core stays pure and testable.
//
// The DEFAULT predictor ABSTAINS — it returns a neutral result and the engine scores exactly as it
// does today. A real model (small, policy-specific, ONNX) is loaded behind this interface in a
// later increment; if it is absent, mis-versioned, or times out, the seam falls back to Abstain so
// the behavioral ML can NEVER block a request by its mere presence (fail-open on latency/
// availability). Enforcement authority is separate and operator-gated in the scoring layer.
package mlserve

import (
	"sync/atomic"

	"github.com/modootoday/humanymous/internal/behavior"
)

// Prediction is the model output. PBot is a CALIBRATED probability in [0,1] that the session is
// automated; Abstain=true means "no usable signal" (no model loaded, schema mismatch, timeout, or
// insufficient behavioral evidence) — the scoring layer treats Abstain as neutral (zero score).
type Prediction struct {
	PBot    float32
	Abstain bool
}

// Abstained is the neutral result.
func Abstained() Prediction { return Prediction{Abstain: true} }

// Predictor scores a behavioral FeatureVector. Implementations must be safe for concurrent use and
// must never panic on a malformed/empty vector (return Abstained instead).
type Predictor interface {
	// Predict returns the calibrated p(bot) for the feature vector, or Abstained.
	Predict(fv behavior.FeatureVector) Prediction
	// SchemaHash is the behavior feature schema this predictor was built against. The caller
	// checks it against behavior.SchemaHash() and abstains on mismatch (stale-model guard).
	SchemaHash() string
	// BundleVersion identifies the loaded model bundle for audit/versioning ("none" when abstaining).
	BundleVersion() string
}

// AbstainPredictor is the default: it never predicts. Shipping this means the l4.ml.behavioral
// signal is present in the pipeline but contributes nothing — proving the seam is freeze-safe
// before any model exists.
type AbstainPredictor struct{}

func (AbstainPredictor) Predict(behavior.FeatureVector) Prediction { return Abstained() }
func (AbstainPredictor) SchemaHash() string                        { return "" }
func (AbstainPredictor) BundleVersion() string                     { return "none" }

// current holds the active predictor. It defaults to AbstainPredictor so the engine has a live,
// safe hook with zero behavior change until a bundle is installed via Set.
var current atomic.Pointer[Predictor]

func init() {
	var p Predictor = AbstainPredictor{}
	current.Store(&p)
}

// Set atomically swaps the active predictor (used by the bundle loader / hot-swap path). A nil
// predictor resets to Abstain.
func Set(p Predictor) {
	if p == nil {
		p = AbstainPredictor{}
	}
	current.Store(&p)
}

// Default returns the active predictor (never nil).
func Default() Predictor { return *current.Load() }

// ThresholdProvider yields the p(bot) cut-point at which the behavioral residual "fires" (annotates
// the session as model-suspicious). It is a dependency-inversion seam: the pure scoring core reads
// FireThreshold() and never imports the self-calibration package. The default is a static 0.5; a
// self-calibrating implementation (internal/mlcorrect, ACI) can be installed at startup so the
// residual fires at a threshold that holds the human false-positive budget over time. Because the
// l4.ml.behavioral signal is weight-0 / score-exempt, moving this cut-point changes only the audit
// annotation, never a verdict — so calibration is monitor-first and freeze-safe.
type ThresholdProvider interface{ FireThreshold() float32 }

var threshold atomic.Pointer[ThresholdProvider]

// SetThresholdProvider installs the fire-threshold source (nil resets to the static default).
func SetThresholdProvider(p ThresholdProvider) {
	if p == nil {
		threshold.Store(nil)
		return
	}
	threshold.Store(&p)
}

// FireThreshold returns the current residual fire cut-point (0.5 when none is installed).
func FireThreshold() float32 {
	if p := threshold.Load(); p != nil {
		return (*p).FireThreshold()
	}
	return 0.5
}

// Score is the convenience entry the scoring layer calls. It enforces the stale-model guard: if
// the active predictor's schema hash does not match the current feature schema, it abstains rather
// than feed a model features it was not trained on. An empty predictor hash (AbstainPredictor)
// short-circuits to Abstain.
func Score(fv behavior.FeatureVector) Prediction {
	p := Default()
	sh := p.SchemaHash()
	if sh == "" {
		return Abstained()
	}
	if sh != behavior.SchemaHash() {
		return Abstained() // stale/mismatched model bundle — fail closed to heuristics
	}
	return p.Predict(fv)
}

// Package behavior is the SHARED, pure-Go behavioral feature transform (SoT-42 Pillar A).
//
// It turns the already-collected behavioral summary (internal/signals.BehaviorSummary — the
// engineered mouse/keystroke/event aggregates the WASM collector produces) into a fixed, ordered,
// deterministically-scaled feature vector. The SAME transform runs at TRAIN time (dataset export)
// and at SERVE time (edge inference), so there is exactly one code path and no train/serve skew —
// mirroring the WASM/server shared-types discipline used elsewhere in the project.
//
// This package has NO ML, NO cgo, NO platform deps. It is the seam a small, policy-specific model
// (internal/mlserve) consumes. It never emits a verdict; the schema hash is what pins a model
// bundle to the exact features it was trained on (a mismatch fails closed to heuristics).
package behavior

import (
	"crypto/sha256"
	"encoding/hex"
	"math"
	"strings"

	"github.com/modootoday/humanymous/internal/signals"
)

// FeatureSchemaVersion identifies the ordered feature layout below. Bump it (and the model
// bundle's feature_schema_hash) whenever the feature set or ordering changes — a bundle trained
// on an older schema must refuse to run against a newer one.
const FeatureSchemaVersion = "bv1"

// featureNames is the CANONICAL ORDERED feature layout. The index of each name is its position in
// the FeatureVector; the list (plus the version) is what SchemaHash pins. Do not reorder — append
// only, and bump FeatureSchemaVersion when you do.
var featureNames = []string{
	// --- pointer / mouse (log1p on heavy-tailed counts/px; fracs/entropy pass through) ---
	"mouse.samples.log", "mouse.mean_curvature", "mouse.velocity_std", "mouse.accel_entropy",
	"mouse.straight_frac", "mouse.pause_count.log", "mouse.max_teleport.log", "mouse.mean_jerk",
	"mouse.coalesced_ratio",
	// --- keystroke ---
	"key.keystrokes.log", "key.mean_dwell_ms.log", "key.dwell_std_ms.log", "key.mean_flight_ms.log",
	"key.flight_std_ms.log", "key.paste_events.log", "key.zero_dwell_frac",
	// --- events / cadence ---
	"event.total.log", "event.untrusted_frac", "event.no_move_clicks.log", "event.synthetic_flags.log",
	"event.click_count.log", "event.long_gap_count.log", "event.burst_var_ms.log", "event.min_reaction_ms.log",
	// --- window + presence flags (let a model condition on which channels are present) ---
	"window.duration_s.log", "presence.has_pointer", "presence.has_key",
}

// FeatureDim is the length of every FeatureVector.
var FeatureDim = len(featureNames)

// FeatureVector is one session's fixed-length, ordered, scaled behavioral feature tensor.
type FeatureVector []float32

// FeatureNames returns a copy of the canonical ordered feature names (for dataset export headers).
func FeatureNames() []string {
	out := make([]string, len(featureNames))
	copy(out, featureNames)
	return out
}

// log1p is a stable, monotone squash for heavy-tailed non-negative counts / durations / pixels /
// milliseconds. Deterministic and part of the schema, so train and serve agree byte-for-byte.
func log1p(v float64) float32 {
	if v < 0 {
		v = 0
	}
	return float32(math.Log1p(v))
}

func b2f(b bool) float32 {
	if b {
		return 1
	}
	return 0
}

// HasPointer reports whether the session carried a usable pointer stream. A keyboard-only /
// no-mouse human is a first-class case: the model must ABSTAIN on pointer features rather than
// read their absence as "bot" (accessibility ship-blocker).
func HasPointer(b signals.BehaviorSummary) bool { return b.Mouse.Samples > 0 }

// HasKey reports whether the session carried keystrokes.
func HasKey(b signals.BehaviorSummary) bool { return b.Key.Keystrokes > 0 }

// Extract computes the canonical FeatureVector from a behavioral summary. Deterministic and
// side-effect free — the single shared train/serve transform.
func Extract(b signals.BehaviorSummary) FeatureVector {
	m, k, e := b.Mouse, b.Key, b.Events
	fv := FeatureVector{
		// mouse
		log1p(float64(m.Samples)), float32(m.MeanCurvature), float32(m.VelocityStdDev),
		float32(m.AccelEntropy), float32(m.StraightLineFrac), log1p(float64(m.PauseCount)),
		log1p(m.MaxTeleportPx), float32(m.MeanJerk), float32(m.CoalescedRatio),
		// key
		log1p(float64(k.Keystrokes)), log1p(k.MeanDwellMs), log1p(k.DwellStdDevMs),
		log1p(k.MeanFlightMs), log1p(k.FlightStdDev), log1p(float64(k.PasteEvents)),
		float32(k.ZeroDwellFrac),
		// events
		log1p(float64(e.TotalEvents)), float32(e.UntrustedFrac), log1p(float64(e.NoMoveClicks)),
		log1p(float64(e.SyntheticFlags)), log1p(float64(e.ClickCount)), log1p(float64(e.LongGapCount)),
		log1p(e.BurstVarMs), log1p(e.MinReactionMs),
		// window + presence
		log1p(b.DurationS), b2f(HasPointer(b)), b2f(HasKey(b)),
	}
	return fv
}

// SchemaHash is a stable digest of the ordered feature layout + version. A model bundle records
// this at train time; the edge refuses to run a bundle whose hash does not match, so a feature
// change can never silently feed a stale model.
func SchemaHash() string {
	h := sha256.New()
	h.Write([]byte(FeatureSchemaVersion))
	h.Write([]byte{0})
	h.Write([]byte(strings.Join(featureNames, ",")))
	return hex.EncodeToString(h.Sum(nil)[:12])
}

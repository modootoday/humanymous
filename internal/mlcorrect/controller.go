package mlcorrect

import (
	"math"

	"github.com/modootoday/humanymous/internal/behavior"
	"github.com/modootoday/humanymous/internal/mlserve"
)

// Controller must satisfy the mlserve seams (dependency inversion: scoring/mlserve read the
// interfaces, this package implements them) — the fire-threshold source, the covariate observer, and
// the shadow comparator.
var (
	_ mlserve.ThresholdProvider = (*Controller)(nil)
	_ mlserve.FeatureObserver   = (*Controller)(nil)
	_ mlserve.ShadowObserver    = (*Controller)(nil)
)

// controller.go fuses the self-correcting sensors into one control plane and exposes it as the
// behavioral residual's live fire-threshold source (SoT-42 Pillar A, SELF-CORRECTION.md step ①
// MONITOR). It implements the mlserve.ThresholdProvider seam structurally — FireThreshold() — so the
// pure scoring core reads a self-calibrated cut-point without importing this package.
//
// It is monitor-first and freeze-safe: the threshold it serves starts at θ0 (0.5 by default,
// identical to the prior static cut), and only the weight-0 / score-exempt l4.ml.behavioral
// annotation moves as θ self-calibrates — never a verdict. The afferent side (ObserveOutcome /
// ObservePass / UpdateCovariate) is fed from the independent-oracle outcome path; until those events
// flow, θ holds at θ0 and the residual annotates exactly as before.
type Controller struct {
	thr    *ConformalThreshold
	drift  *DriftMonitor
	guard  *PassGuard
	feat   *FeatureMonitor
	shadow *ShadowComparator
}

// NewController builds the control plane. budget is the tolerated human false-positive rate the ACI
// threshold maintains; theta0 the starting fire threshold; gamma the adaptation rate.
func NewController(budget, theta0, gamma float32) *Controller {
	return &Controller{
		thr:    NewConformalThreshold(budget, theta0, gamma),
		drift:  NewDriftMonitor(),
		guard:  NewPassGuard(0.4, 0.6),
		feat:   NewFeatureMonitor(0),
		shadow: &ShadowComparator{},
	}
}

// FireThreshold implements mlserve.ThresholdProvider: the current self-calibrated θ.
func (c *Controller) FireThreshold() float32 { return c.thr.Threshold() }

// ObserveOutcome feeds ONE adjudicated session into calibration + drift. pBot is the model's score
// for that session; engineBot is the FROZEN engine's own verdict (the STUDD teacher); o is the
// INDEPENDENT oracle outcome (solved Pass ⇒ human, catalog/failed-CHALLENGE ⇒ bot). It returns the
// current drift gate so a caller can open a retrain ticket. Ambiguous outcomes update only the STUDD
// mimic stream (which needs no label), never the human-FP calibration — no self-labeling.
func (c *Controller) ObserveOutcome(o Outcome, pBot float32, engineBot bool) DriftEvent {
	// STUDD: student(model p(bot)) vs teacher(frozen engine verdict). Label-free.
	var teacher float32
	if engineBot {
		teacher = 1
	}
	c.drift.ObserveMimic(math.Abs(float64(pBot - teacher)))

	if label, auto := Confirm(o); auto && label != LabelAmbiguous {
		isHuman := label == LabelHuman
		c.thr.Observe(isHuman, pBot) // ACI: only human-FP moves θ
		// oracle-error stream: the model is "wrong" when its fire decision disagrees with the label.
		flagged := pBot >= c.thr.Threshold()
		isBot := label == LabelBot
		errInd := 0.0
		if flagged != isBot {
			errInd = 1
		}
		c.drift.ObserveOracleError(errInd)
	}
	return c.drift.Gate()
}

// ObservePass records a Pass attempt outcome for the poisoning-guard solve-rate monitor. An abnormal
// solve-rate spike (adversaries solving Pass at scale) surfaces via Snapshot as a drift signal that
// STOPS auto-ingesting Pass-human labels offline.
func (c *Controller) ObservePass(solved bool) { c.guard.ObservePassAttempt(solved) }

// UpdateCovariate feeds the max per-feature PSI vs the frozen reference (from a batch feature
// monitor) into the covariate-shift arm of the 2-of-3 drift gate.
func (c *Controller) UpdateCovariate(maxFeaturePSI float64) { c.drift.UpdatePSI(maxFeaturePSI) }

// ObserveFeatures implements mlserve.FeatureObserver: it taps every scored feature vector into the
// covariate monitor and, when a window closes, pushes the fresh max-PSI into the drift gate's
// covariate arm. This runs on the hot path (only when a model is live) — the per-call work is a
// handful of bucket comparisons; PSI is computed once per closed window, not per call.
func (c *Controller) ObserveFeatures(fv behavior.FeatureVector) {
	if maxPSI, closed := c.feat.Observe(fv); closed {
		c.drift.UpdatePSI(maxPSI)
	}
}

// ObserveShadow implements mlserve.ShadowObserver: it compares a staged candidate's prediction to the
// active model's, at the live fire threshold θ, so an operator can see how a promotion would change
// behavior before committing. Monitor-only — the shadow output is never served.
func (c *Controller) ObserveShadow(active, shadow mlserve.Prediction) {
	c.shadow.observe(active.PBot, shadow.PBot, active.Abstain, shadow.Abstain, c.thr.Threshold())
}

// ControllerSnapshot is the no-PII observability view for the Ledger / admin console / canary guard.
type ControllerSnapshot struct {
	Calibration   Stats       // θ, budget, realized human-FP EMA, counts
	Drift         DriftEvent  // current 2-of-3 gate + component alarms
	PassSolveRate float32
	PassAnomalous bool        // Pass solve-rate abnormal ⇒ suspected oracle poisoning
	Shadow        ShadowStats // active-vs-candidate divergence (zero-valued when no shadow installed)
}

// Snapshot returns the current control-plane state.
func (c *Controller) Snapshot() ControllerSnapshot {
	return ControllerSnapshot{
		Calibration:   c.thr.Snapshot(),
		Drift:         c.drift.Gate(),
		PassSolveRate: c.guard.SolveRate(),
		PassAnomalous: c.guard.SolveRateAnomalous(),
		Shadow:        c.shadow.Snapshot(),
	}
}

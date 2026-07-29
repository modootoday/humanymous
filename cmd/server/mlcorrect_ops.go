package main

import (
	"net/http"

	"github.com/modootoday/humanymous/internal/behavior"
	"github.com/modootoday/humanymous/internal/mlcorrect"
	"github.com/modootoday/humanymous/internal/mlserve"
	"github.com/modootoday/humanymous/internal/mltrain"
	"github.com/modootoday/humanymous/internal/scoring"
	"github.com/modootoday/humanymous/internal/signals"
)

// mlcorrect_ops.go connects the SoT-42 Pillar A self-correcting control plane to the request path:
// the afferent feed (Pass outcomes → calibration/drift) and the read-only observability endpoint.
// Everything here is monitor-first and nil-safe: with no -ml-bundle, a.ctrl is nil and every path
// is a no-op, so the engine and its verdicts are byte-identical to a build without this file.

// handleMLCorrect exposes the control-plane snapshot (θ, realized human-FP EMA, drift gate, Pass
// anomaly) for operators. Operator-token gated, read-only, no PII. Returns a disabled marker when no
// model/controller is loaded rather than 404, so an operator can tell "off" from "mis-authed".
func (a *app) handleMLCorrect(w http.ResponseWriter, r *http.Request) {
	if !a.operatorAuthorized(w, r) {
		return
	}
	if a.ctrl == nil {
		writeJSON(w, map[string]any{"enabled": false, "reason": "no -ml-bundle loaded; l4.ml.behavioral abstains"})
		return
	}
	snap := a.ctrl.Snapshot()
	writeJSON(w, map[string]any{
		"enabled":       true,
		"fireThreshold": a.ctrl.FireThreshold(),
		"calibration":   snap.Calibration,
		"drift":         snap.Drift,
		"passSolveRate": snap.PassSolveRate,
		"passAnomalous": snap.PassAnomalous,
		"shadow":        snap.Shadow, // active-vs-candidate divergence (zero when no shadow bundle)
	})
}

// feedPassOutcome feeds a COMPLETED Pass into the self-correcting control plane (monitor-first).
//
// A SOLVED Pass is a strong, INDEPENDENT human-oracle signal — exactly what the ACI calibration
// needs (θ moves only on confirmed humans). A FAILED Pass is deliberately NOT treated as a confirmed
// bot: humans fumble the puzzle (ACC-1), and mislabeling them would poison the oracle-error/STUDD
// streams. So a failure feeds only the solve-rate guard, never a bot label. The model p(bot) is
// recomputed from the session's stored behavioral summary (it is not persisted at score time), and
// the frozen engine's own verdict is the STUDD teacher. No-op when the control plane is absent or
// the model abstains — never self-labels, never touches a verdict.
func (a *app) feedPassOutcome(rep signals.SessionReport, solved bool) {
	if solved {
		// A solved Pass is a confirmed human: persist the labeled trace for offline retraining if the
		// operator enabled collection. Fail-safe + nil-safe — never blocks or fails the request.
		if err := a.traceSink.Append(mltrain.Record{
			Label:    0, // human
			Behavior: rep.Client.Behavior,
			TS:       rep.Timestamp.Unix(),
			Cohort:   cohortOf(rep.Client.Behavior),
			Source:   "pass",
		}); err != nil {
			a.log.Warn("ml oracle-trace append failed", "err", err)
		}
	}
	if a.ctrl == nil {
		return
	}
	a.ctrl.ObservePass(solved)
	if !solved {
		return // failures inform only the poisoning-guard solve-rate, never the human/bot oracle
	}
	pred := mlserve.Score(behavior.Extract(rep.Client.Behavior))
	if pred.Abstain {
		return // no usable model signal for this session — nothing to calibrate against
	}
	engineBot := rep.Scoring.Verdict == scoring.VerdictDeny // STUDD teacher: engine's own automation call
	a.ctrl.ObserveOutcome(mlcorrect.OutcomePassSolved, pred.PBot, engineBot)
}

// cohortOf classifies a session's input MODALITY into an accessibility-relevant cohort, so collected
// traces let the offline retrain gate enforce human-FP≈0 PER modality. This matters because a
// motor-behavior model is most likely to over-flag exactly the users who generate little pointer
// microstructure — keyboard-only, switch-access, and other assistive-tech users — and an aggregate
// gate would hide that regression behind the pointer-using majority. It is a coarse, honest proxy
// from the aggregate summary (the threshold mirrors l4.event.no_interaction's <4 mouse samples):
//   - typed but little/no pointer motion ⇒ "keyboard" (the AT-adjacent lane),
//   - real pointer motion               ⇒ "pointer",
//   - neither                           ⇒ "default".
func cohortOf(b signals.BehaviorSummary) string {
	if b.Mouse.Samples < 4 {
		if b.Key.Keystrokes > 0 {
			return "keyboard"
		}
		return "default"
	}
	return "pointer"
}

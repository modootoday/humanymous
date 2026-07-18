package scoring

import "github.com/modootoday/humanymous/internal/signals"

// behavior.go turns a reduced behavioral feature vector (L4) into signals using
// explainable heuristic thresholds (SoT-03 §5). Thresholds are conservative to
// keep FPR low; short sessions produce low confidence.

// behaviorThresholds groups the tunable cut-points (SoT-03).
type behaviorThresholds struct {
	minMouseSamples int     // below this, low confidence
	minKeystrokes   int     // below this, low confidence
	straightnessHi  float64 // straightness ratio <= this near-1.0 => bot-ish
	velocityStdLo   float64 // px/ms std below => too regular
	dwellStdLo      float64 // ms std below => too regular
	flightStdLo     float64 // ms std below => too regular
	teleportPx      float64 // instantaneous jump above => teleport
}

func defaultBehaviorThresholds() behaviorThresholds {
	return behaviorThresholds{
		minMouseSamples: 12,
		minKeystrokes:   6,
		straightnessHi:  1.03,
		velocityStdLo:   0.02,
		dwellStdLo:      3.0,
		flightStdLo:     5.0,
		teleportPx:      400,
	}
}

// BehaviorSignals derives L4 signals from a BehaviorSummary. Confidence scales
// with sample count so sparse sessions can't drive a hard verdict.
func BehaviorSignals(b signals.BehaviorSummary) []signals.Signal {
	t := defaultBehaviorThresholds()
	var out []signals.Signal
	add := func(id string, val any, v signals.Verdict, conf float64, notes string) {
		out = append(out, signals.New(id, val, v, conf, signals.SourceWASM, notes))
	}

	// --- No meaningful interaction: a session observed for a real window that
	// produced no mouse trajectory and no keystrokes is bot-like (headful
	// nodriver-class). A real human generates dozens of pointermove samples; a
	// couple of stray load-time events do not count. Low confidence to protect
	// passive human readers (SoT-04 frontier note). ---
	if b.Mouse.Samples < 4 && b.Key.Keystrokes == 0 && b.DurationS > 1.5 {
		add("l4.event.no_interaction", b.Mouse.Samples, signals.VerdictSuspicious, 0.6,
			"no meaningful human input over observation window")
	}

	// --- Events: isTrusted is the strongest, highest-confidence tell. ---
	if b.Events.TotalEvents > 0 {
		if b.Events.UntrustedFrac > 0 {
			add("l4.event.untrusted", b.Events.UntrustedFrac, signals.VerdictBot, 1.0,
				"synthetic dispatchEvent detected")
		} else {
			add("l4.event.untrusted", 0, signals.VerdictOK, 1.0, "")
		}
		if b.Events.NoMoveClicks > 0 {
			add("l4.event.no_move_click", b.Events.NoMoveClicks, signals.VerdictSuspicious, 0.7,
				"clicks without preceding movement")
		}
		if b.Events.SyntheticFlags > 0 {
			add("l4.event.perfect_timing", b.Events.SyntheticFlags, signals.VerdictSuspicious, 0.6,
				"zero-jitter timing artifacts")
		}
	}

	// --- AI-agent behavioral tells (SoT-16) ---
	// LLM inference-loop cadence: >=2 long "thinking" pauses, low intra-burst
	// variance, and instant resumption (no human reaction-time floor ~200-300ms).
	// A reading human also pauses, but resumes with a reaction floor + variance.
	if b.Events.LongGapCount >= 2 && b.Events.TotalEvents >= 8 &&
		b.Events.MinReactionMs > 0 && b.Events.MinReactionMs < 150 && b.Events.BurstVarMs < 2500 {
		add("l4.agent.burst_silence",
			map[string]any{"gaps": b.Events.LongGapCount, "react": b.Events.MinReactionMs},
			signals.VerdictBot, 0.6, "LLM inference-loop cadence (pause->instant burst)")
	}
	// Mouse events only at click time (FP-Agent's top feature): clicks present
	// but almost no approach trajectory.
	if b.Events.ClickCount >= 2 && b.Mouse.Samples < b.Events.ClickCount*3 {
		add("l4.mouse.click_no_trajectory", b.Mouse.Samples, signals.VerdictBot, 0.6,
			"clicks without an approach trajectory (agent teleport-click)")
	}
	// Machine-speed typing: mean flight < 25ms (humans ~80-150ms with high
	// variance; AI agents type at single-digit-to-low-double-digit ms).
	if b.Key.Keystrokes >= 4 && b.Key.MeanFlightMs > 0 && b.Key.MeanFlightMs < 25 {
		add("l4.key.machine_speed", b.Key.MeanFlightMs, signals.VerdictBot, 0.7,
			"machine-speed inter-key latency")
	}

	// --- Mouse kinematics ---
	if b.Mouse.Samples >= t.minMouseSamples {
		conf := sampleConfidence(b.Mouse.Samples, t.minMouseSamples)
		if b.Mouse.StraightLineFrac > 0.8 || (b.Mouse.MeanCurvature > 0 && b.Mouse.MeanCurvature < 0.001) {
			add("l4.mouse.straightness", b.Mouse.StraightLineFrac, signals.VerdictSuspicious, conf,
				"near-straight-line trajectories")
		}
		if b.Mouse.VelocityStdDev < t.velocityStdLo {
			add("l4.mouse.velocity_std", b.Mouse.VelocityStdDev, signals.VerdictSuspicious, conf,
				"near-zero velocity variance (too regular)")
		}
		if b.Mouse.MaxTeleportPx > t.teleportPx {
			add("l4.mouse.max_teleport", b.Mouse.MaxTeleportPx, signals.VerdictSuspicious, conf,
				"large instantaneous cursor jump")
		}
		// Coalesced-events tell (SoT-14): real pointing hardware batches many
		// high-frequency sub-samples per pointermove; CDP/OS-injected input
		// produces ~1. Catches synthetic input that isTrusted cannot (isTrusted
		// is true for CDP injection). Moderate confidence: some low-rate devices
		// legitimately coalesce little.
		if b.Mouse.CoalescedRatio > 0 && b.Mouse.CoalescedRatio <= 1.05 {
			add("l4.mouse.coalesced_synthetic", b.Mouse.CoalescedRatio, signals.VerdictSuspicious, 0.5,
				"pointermoves carry no coalesced sub-events (synthetic input)")
		}
	} else if b.Mouse.Samples == 0 && b.Events.NoMoveClicks > 0 {
		add("l4.mouse.samples", 0, signals.VerdictSuspicious, 0.6, "interaction without any mouse movement")
	}

	// --- Keystroke dynamics ---
	if b.Key.Keystrokes >= t.minKeystrokes {
		conf := sampleConfidence(b.Key.Keystrokes, t.minKeystrokes)
		if b.Key.DwellStdDevMs < t.dwellStdLo {
			add("l4.key.dwell_std", b.Key.DwellStdDevMs, signals.VerdictSuspicious, conf,
				"near-zero dwell variance (fixed-delay typing)")
		}
		if b.Key.FlightStdDev < t.flightStdLo {
			add("l4.key.flight_std", b.Key.FlightStdDev, signals.VerdictSuspicious, conf,
				"near-zero flight variance")
		}
		if b.Key.ZeroDwellFrac > 0.5 {
			add("l4.key.zero_dwell_frac", b.Key.ZeroDwellFrac, signals.VerdictBot, conf,
				"majority synthetic zero-dwell keys")
		}
	}
	if b.Key.PasteEvents > 0 && b.Key.Keystrokes == 0 {
		add("l4.key.paste", b.Key.PasteEvents, signals.VerdictSuspicious, 0.6,
			"field populated by paste with no keystrokes")
	}

	return out
}

// sampleConfidence ramps from 0.5 at the minimum sample count toward 1.0 as
// samples grow, so richer sessions yield more confident behavioral verdicts.
func sampleConfidence(n, min int) float64 {
	if n <= min {
		return 0.5
	}
	c := 0.5 + 0.5*float64(n-min)/float64(min*4)
	if c > 1.0 {
		return 1.0
	}
	return c
}

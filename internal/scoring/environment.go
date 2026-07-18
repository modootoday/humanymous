package scoring

import "github.com/modootoday/humanymous/internal/signals"

// environment.go derives L2 environment signals from the ad-block / tracking
// protection probe (SoT-09). Design invariant: privacy tooling is a HUMAN
// trait, so adblock/tracking-protection never raise the bot score on their own.
// They (a) tag the session as privacy-conscious for FP mitigation, and (b) feed
// consistency and liveness checks.

// EnvironmentResult reports derived signals plus a privacy flag consumed by
// FP mitigation.
type EnvironmentResult struct {
	Signals []signals.Signal
	Privacy bool // session shows genuine privacy tooling -> damp fingerprint noise
}

// EnvironmentSignals evaluates the probe. hasClient indicates whether any
// client report was received at all (to distinguish "blocked" from "no JS").
func EnvironmentSignals(e signals.Environment, hasClient bool) EnvironmentResult {
	var out []signals.Signal
	add := func(id string, val any, v signals.Verdict, conf float64, notes string) {
		out = append(out, signals.New(id, val, v, conf, signals.SourceJS, notes))
	}

	// Liveness: if the client is present but the probe never executed, that is
	// a weak anomaly (a real browser running our JS would complete the probe).
	if hasClient && !e.Probed {
		add("l2.env.probe_dead", false, signals.VerdictSuspicious, 0.5,
			"client present but environment probe did not run")
		return EnvironmentResult{Signals: out}
	}
	if !e.Probed {
		return EnvironmentResult{Signals: out} // no client -> handled by HR-10 elsewhere
	}

	privacy := false

	// Ad-block present: neutral-to-human. Record as OK (0 weight) + set privacy.
	if e.AdBlock {
		add("l2.env.adblock", true, signals.VerdictOK, 1.0, "ad-block active (privacy user)")
		privacy = true
	}
	if e.TrackingProtection || e.GPC || e.DoNotTrack == "1" || e.BraveShields {
		add("l2.env.tracking_protection", true, signals.VerdictOK, 1.0, "tracking protection active")
		privacy = true
	}

	// Consistency: a claimed strong-privacy posture (GPC/DNT/Brave) that blocks
	// NOTHING is inconsistent -- a spoofer set the flags without the behavior.
	claimsPrivacy := e.GPC || e.DoNotTrack == "1" || e.BraveShields
	blocksSomething := e.AdBlock || e.TrackingProtection
	if claimsPrivacy && !blocksSomething {
		add("l2.env.privacy_inconsistent", true, signals.VerdictSuspicious, 0.6,
			"privacy flags set but no blocking behavior observed")
	}

	return EnvironmentResult{Signals: out, Privacy: privacy}
}

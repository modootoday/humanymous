package main

import (
	"net/http"
	"time"

	"github.com/modootoday/humanymous/internal/signals"
)

// pass_scoring.go is the engine-fusion side of humanymous Pass (PLAN-07 R12: split
// out of pass_handler.go by concern). It is where a Pass attempt is JUDGED: the
// axis-③ velocity governor, the SOFT behavioral model, the blue difficulty
// controller, and the KPI labelling. Every path here folds risk into the shared
// engine WITHOUT any lockout/delay — the accessible lane never gates on motor
// richness or speed (SoT-36 §5). No behavior change from the split.

// passFP is the STABLE per-fingerprint key (JA4 engine + /24 subnet) — the identity
// that a cookie-rotating bot cannot shed but a residential-proxy pool shares. Both
// the velocity governor (axis ③) and the attestation issuer (axis ①) key on it.
func (a *app) passFP(r *http.Request) string {
	return ja4Stable(a.reg.Hello(r.RemoteAddr)) + "|" + clientSubnet(r)
}

// passSolvedSignal is the OK signal that grants the Pass trust upgrade (SoT-36 §3).
func passSolvedSignal() signals.Signal {
	return signals.New("l7.pass.solved", true, signals.VerdictOK, 1.0, signals.SourceServer, "humanymous Pass cleared")
}

// applyPassVelocity is the axis-③ engine fusion (SoT-36 §7): it meters how fast a
// session (and its JA4|subnet fingerprint) drives the Pass endpoints and, past a
// threshold, folds an l7.pass.velocity/flood signal into the session's engine score.
// This does TWO honest things WITHOUT any lockout/delay:
//   - raises the session's risk, so a flooding bot that later SOLVES the puzzle is
//     STILL flagged BOT by the engine verdict (solving Pass never launders automation),
//   - lets the caller tax the PoW *cost* (crypto axis), never the puzzle difficulty
//     (accessibility: cost shifts to crypto as suspicion grows, SoT-36 §2).
//
// The window is short (30s) and self-clearing — it never stalls the red/blue wargame.
// Returns (combined level, session-only level): the combined level drives the risk
// signal + PoW cost; the session level drives the attestation requirement, keeping the
// shared-fingerprint wargame's fresh-session strategies clean. In production the
// attestation trigger extends to the fp level (already metered), catching a
// cookie-rotating flood from one JA4|subnet — not exercisable in a shared-fp harness.
func (a *app) applyPassVelocity(sid string, r *http.Request, now time.Time) (int, int) {
	sLvl := a.passVel.Level(a.passVel.Observe("s|"+sid, now))         // per-session cadence
	fLvl := a.passVel.Level(a.passVel.Observe("f|"+a.passFP(r), now)) // cross-session (proxy pool)
	lvl := sLvl
	if fLvl > lvl {
		lvl = fLvl
	}
	if lvl == 0 {
		return 0, sLvl
	}
	ps := a.pass.get(sid)
	a.pass.mu.Lock()
	prev := ps.velLevel
	if lvl > prev {
		ps.velLevel = lvl
	}
	a.pass.mu.Unlock()
	if lvl <= prev {
		return lvl, sLvl // already flagged at this level — don't append a duplicate signal
	}
	id, v := "l7.pass.velocity", signals.VerdictSuspicious
	if lvl == 2 {
		id, v = "l7.pass.flood", signals.VerdictBot
	}
	rep, _ := a.store.Get(sid)
	rep.Network.Signals = append(rep.Network.Signals,
		signals.New(id, nil, v, 1.0, signals.SourceServer, "Pass velocity (automation cadence)"))
	a.engine.Score(&rep)
	a.store.StoreScored(sid, rep, now)
	return lvl, sLvl
}

// applyPassBehavior is the deeper real-event model (SoT-36 §5), fused SOFT. It derives
// behavioral tells from the interaction proof and folds the matching (already-weighted)
// l4.* signals into the session score. It NEVER blocks — the accessible lane forbids
// gating on motor richness or speed (some AT/switch devices inject fast, regular input)
// — so a flagged trace still clears Pass; the engine merely holds it at elevated risk
// (verdict ≠ ALLOW). Real browser input (fractional performance.now deltas, human-scale
// variance) trips none of these. This softly reaches the fresh-identity single forgery
// that velocity (axis ③) and attestation (axis ①) leave to the last line.
func (a *app) applyPassBehavior(sid string, pr passProof, now time.Time) {
	var sig []signals.Signal
	seen := map[string]bool{}
	add := func(id string, v signals.Verdict, conf float64, note string) {
		if seen[id] {
			return
		}
		seen[id] = true
		sig = append(sig, signals.New(id, nil, v, conf, signals.SourceServer, note))
	}
	if len(pr.KeyDurs) >= 3 {
		if meanFloats(pr.KeyDurs) < 15 { // no human presses arrow keys at <15ms flight
			add("l4.key.machine_speed", signals.VerdictBot, 0.7, "machine-speed inter-key latency (<15ms)")
		}
		if stddev(pr.KeyDurs) < 6 { // real keystroke flight varies by tens of ms
			add("l4.key.flight_std", signals.VerdictSuspicious, 0.5, "near-zero keystroke flight variance")
		}
		if allIntegerMs(pr.KeyDurs) { // real timings carry sub-ms jitter
			add("l4.event.perfect_timing", signals.VerdictSuspicious, 0.6, "zero sub-ms jitter (synthetic timing)")
		}
	}
	if len(pr.Durations) >= 5 && allIntegerMs(pr.Durations) {
		add("l4.event.perfect_timing", signals.VerdictSuspicious, 0.6, "zero sub-ms jitter (synthetic pointer timing)")
	}
	// Mobile sensor consistency (SoT-36 §5): a touch/mobile-claiming session with flat
	// or absent device-motion is inconsistent — a real phone always carries hand-tremor
	// micro-jitter. SOFT: a mounted phone is low-motion too, so this raises risk, never
	// blocks. Desktop mouse/keyboard sessions make no mobile claim and are never checked.
	if pr.PointerType == "touch" || len(pr.Pressures) > 0 {
		if len(pr.Motion) < 3 || stddev(pr.Motion) < 0.01 {
			add("l2.adv.mobile_inconsistent", signals.VerdictSuspicious, 0.5, "touch/mobile claim without device-motion micro-jitter")
		}
	}
	if len(sig) == 0 {
		return
	}
	rep, _ := a.store.Get(sid)
	rep.Network.Signals = append(rep.Network.Signals, sig...)
	a.engine.Score(&rep)
	a.store.StoreScored(sid, rep, now)
}

// passDifficulty is the blue controller's knob (SoT-36 §8): harder challenges for
// suspicious sessions, trivial for likely-humans. Difficulty scales with the
// session's current risk; a self-labelled red-team probe always gets the hardest.
func (a *app) passDifficulty(sid string, r *http.Request) int {
	if r.Header.Get("X-HM-Redteam") != "" {
		return 3
	}
	rep, _ := a.store.Get(sid)
	risk := rep.Scoring.RiskScore
	d := int((risk - 20) / 15) // 30->0, 45->1, 60->2, 75->3
	if d < 0 {
		d = 0
	}
	if d > 3 {
		d = 3
	}
	return d
}

// passLabel classifies an attempt for the wargame KPIs (SoT-36 §8). A self-declared
// red-team probe reports its STRATEGY name (ground-truth bot, per-strategy KPI);
// otherwise the session's risk band is a weak label (bot >=70, human <30, else
// unknown). Strategy names are header values with no '|' so the metric key is safe.
func passLabel(risk float64, r *http.Request) string {
	if s := r.Header.Get("X-HM-Redteam"); s != "" {
		return s
	}
	switch {
	case risk >= 70:
		return "bot"
	case risk < 30:
		return "human"
	default:
		return "unknown"
	}
}

// isBotLabel reports whether a KPI label is an adversary (a red-team strategy or a
// high-risk "bot"), as opposed to the human floor / ambiguous "unknown" band.
func isBotLabel(label string) bool { return label != "human" && label != "unknown" }

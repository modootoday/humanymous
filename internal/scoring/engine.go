package scoring

import "github.com/modootoday/humanymous/internal/signals"

// engine.go orchestrates L6 cross-checks and L7 scoring. It is deliberately
// thin: each concern (crosschecks, behavior, environment, combine, hardrules)
// lives in its own file (SRP). Score is a pure function (SoT-05 §8).

// Engine scores sessions under a fixed policy.
type Engine struct {
	Policy Policy
}

// NewEngine returns an Engine with the default policy (policyVersion 1.0.0).
func NewEngine() *Engine { return &Engine{Policy: DefaultPolicy()} }

// Score evaluates a merged SessionReport and fills its CrossChecks + Scoring.
// It mutates r.CrossChecks and returns the ScoreResult (also stored in r).
func (e *Engine) Score(r *signals.SessionReport) signals.ScoreResult {
	contributions, rc := e.assemble(r)
	risk := Combine(contributions, e.Policy)
	rc.combined = risk
	return e.decide(r, contributions, rc, risk)
}

// ScoreWithTrace scores exactly as Score, and additionally returns the additive
// ScoreTrace (SoT-30 §6): per-layer subtotals + LayerCap hits, dedup drops, the
// ordered hard-rule evaluation, and the contribution set. It reuses the same
// dedup/noisy-OR math and rule table, so the ScoreResult it returns is identical
// to Score's (guarded by scoretrace_test.go).
func (e *Engine) ScoreWithTrace(r *signals.SessionReport) (signals.ScoreResult, ScoreTrace) {
	contributions, rc := e.assemble(r)
	risk, layers, drops := combineTrace(contributions, e.Policy)
	rc.combined = risk
	res := e.decide(r, contributions, rc, risk)
	trace := ScoreTrace{
		Contributions: contribTraces(contributions),
		DedupDropped:  drops,
		PerLayer:      layers,
		HardRules:     tracePromotionRules(rc),
		Score:         res.RiskScore,
		Verdict:       res.Verdict,
		HardRuleFired: res.HardRuleFired,
		Policy:        PolicyView{e.Policy.Version, e.Policy.LayerCap, e.Policy.ChallengeAt, e.Policy.DenyAt},
	}
	return res, trace
}

// assemble derives the extra signals, runs the L6 cross-checks (mutating r), and
// builds the scored contribution set + the hard-rule context. Shared by Score
// and ScoreWithTrace so the two never diverge on inputs.
func (e *Engine) assemble(r *signals.SessionReport) (contributions []scored, rc ruleContext) {
	// 1. Derive additional signals the client can't self-score.
	behavior := BehaviorSignals(r.Client.Behavior)
	env := EnvironmentSignals(r.Client.Environment, hasClientReport(r))
	guard := GuardSignals(r.Client.Guard)
	advanced := AdvancedSignals(r.Client.Advanced, r.Client.UserAgent)

	// 2. Cross-checks (L6).
	r.CrossChecks = CrossChecks(r)

	// 3. Assemble the scored contribution set from every source.
	extra := append(append([]signals.Signal{}, env.Signals...), guard...)
	extra = append(extra, advanced...)
	contributions = e.gather(r, behavior, extra)

	// 4. FP mitigation before combination (privacy users, low-spec devices).
	contributions = applyFPMitigation(contributions)
	if env.Privacy {
		contributions = dampPrivacyNoise(contributions)
	}

	// Hard-rule view: every signal source flattened.
	rc = ruleContext{
		sigs:         r.AllSignals(),
		cross:        r.CrossChecks,
		hasClient:    hasClientReport(r),
		browserClaim: isBrowserUA(r.Client.UserAgent),
	}
	rc.sigs = append(rc.sigs, behavior...)
	rc.sigs = append(rc.sigs, env.Signals...)
	rc.sigs = append(rc.sigs, guard...)
	rc.sigs = append(rc.sigs, advanced...)
	return contributions, rc
}

// decide applies the verdict band + hard-rule override + PoW upgrade and writes
// the ScoreResult into r. Shared by Score and ScoreWithTrace.
func (e *Engine) decide(r *signals.SessionReport, contributions []scored, rc ruleContext, risk float64) signals.ScoreResult {
	verdict := e.Policy.verdictFor(risk)
	hard := applyPromotionRules(rc)
	firedRule := ""
	if hard.verdict != "" {
		verdict = hard.verdict
		firedRule = hard.rule
	}

	// PoW trust upgrade (SoT-13): a session that solved a proof-of-work
	// challenge upgrades a SCORE-based CHALLENGE to ALLOW. It never overrides a
	// hard rule (DENY or a hard CHALLENGE) — PoW proves CPU work, not humanity.
	if verdict == VerdictChallenge && firedRule == "" && hasSignal(rc.sigs, "l7.pow.solved") {
		verdict = VerdictAllow
		firedRule = "PoW-upgrade"
	}

	// humanymous Pass trust upgrade (SoT-36 §3): clearing the interactive Pass
	// challenge upgrades a SCORE-based CHALLENGE to ALLOW, exactly like PoW. It
	// NEVER overrides a hard rule — a session already proven a bot by L1–L7 / JA4 /
	// correlation stays DENY even after solving Pass. Solving Pass is one signal
	// among many; it does not launder conclusive independent bot evidence.
	if verdict == VerdictChallenge && firedRule == "" && hasSignal(rc.sigs, "l7.pass.solved") {
		verdict = VerdictAllow
		firedRule = "Pass-upgrade"
	}

	res := signals.ScoreResult{
		RiskScore:       round1(risk),
		Verdict:         verdict,
		HardRuleFired:   firedRule,
		TopContributors: topContributors(contributions, 5),
		PolicyVersion:   e.Policy.Version,
	}
	r.Scoring = res
	return res
}

// gather converts client/network/behavior/env signals + cross-checks into the
// unified scored slice used by Combine.
func (e *Engine) gather(r *signals.SessionReport, behavior, env []signals.Signal) []scored {
	var out []scored
	appendSig := func(s signals.Signal) {
		out = append(out, scored{
			id:    s.ID,
			layer: s.Layer,
			group: signals.GroupOf(s.ID),
			score: s.Score,
		})
	}
	for _, s := range r.Client.Signals {
		appendSig(s)
	}
	for _, s := range r.Network.Signals {
		appendSig(s)
	}
	for _, s := range behavior {
		appendSig(s)
	}
	for _, s := range env {
		appendSig(s)
	}
	// Cross-checks (L6) participate as network-layer contributions.
	for _, x := range r.CrossChecks {
		if x.Score > 0 {
			out = append(out, scored{
				id:    x.ID,
				layer: signals.LayerConsistency,
				group: "", // each cross-check is independent
				score: x.Score,
			})
		}
	}
	return out
}

// dampPrivacyNoise halves fingerprint-noise contributions for sessions that
// show genuine privacy tooling (SoT-05 §4.3 / SoT-09). NOTE: this is FP-mitigation,
// not the hard rule HR-20 (which is the AI-agent DENY rule; see hardrules.go).
func dampPrivacyNoise(in []scored) []scored {
	for i := range in {
		switch in[i].group {
		case "canvas", "webgl", "audio", "font":
			in[i].score *= 0.5
		}
	}
	return in
}

func hasSignal(sigs []signals.Signal, id string) bool {
	for _, s := range sigs {
		if s.ID == id {
			return true
		}
	}
	return false
}

func hasClientReport(r *signals.SessionReport) bool {
	return r.Client.UserAgent != "" || len(r.Client.Signals) > 0 || r.Client.Environment.Probed
}

func round1(f float64) float64 {
	return float64(int(f*10+0.5)) / 10
}

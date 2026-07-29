package mlcorrect

import "sync"

// oracle.go is the anti-model-collapse label source (SoT-42 Pillar A). The research is unambiguous:
// training a model on labels it (or the frozen engine) produced is a self-consuming loop that
// erases the tail — precisely the rare novel bots the residual exists to catch (Shumailov 2024;
// Strong Model Collapse ICLR 2025). So NO label derived from the model's own p(bot) ever enters
// training. Instead, labels come from the engine's INDEPENDENT adjudication:
//
//   - a SOLVED humanymous Pass (SoT-36, LLM-resistant, accessibility-safe)      ⇒ HUMAN
//   - a red-team CATALOG member, or a FAILED/ABANDONED CHALLENGE                 ⇒ BOT
//   - anything else                                                             ⇒ AMBIGUOUS (→ human queue)
//
// This is oracle-by-harder-independent-mechanism, not self-labeling. Even so the Pass channel is a
// poisoning surface (if adversaries reliably solve Pass, bots get labeled human), so PassGuard
// caps the Pass-sourced fraction of any batch and flags solve-rate anomalies as DRIFT, not data.

// Outcome is the engine's independent adjudication of a session, fed to the oracle after the fact.
type Outcome int

const (
	OutcomeUnknown Outcome = iota
	OutcomePassSolved      // interactive Pass solved → strong human evidence
	OutcomeChallengeFailed // CHALLENGE issued, proof failed → bot evidence
	OutcomeChallengeAbandoned
	OutcomeCatalogBot // a measured red-team catalog session → gold bot
)

// Label is a confirmed training label.
type Label int

const (
	LabelAmbiguous Label = iota // route to the human analyst queue; never auto-trained
	LabelHuman
	LabelBot
)

// Confirm maps an adjudication outcome to a training label and whether it is AUTO-labelable (safe
// to enter training without a human). Ambiguous outcomes are never auto-labeled.
func Confirm(o Outcome) (label Label, auto bool) {
	switch o {
	case OutcomePassSolved:
		return LabelHuman, true
	case OutcomeCatalogBot, OutcomeChallengeFailed, OutcomeChallengeAbandoned:
		return LabelBot, true
	default:
		return LabelAmbiguous, false
	}
}

// PassGuard defends the Pass→human channel against oracle poisoning. It (a) caps the fraction of a
// training batch that may be sourced from Pass-human labels (keeping the gold catalog dominant),
// and (b) tracks the Pass solve-rate so an abnormal spike is surfaced as a DRIFT alarm rather than
// ingested as clean data.
type PassGuard struct {
	mu           sync.Mutex
	maxPassFrac  float32 // e.g. 0.4 — at most 40% of a batch from Pass-human labels
	solveEMA     float32 // decaying Pass solve-rate estimate
	solveBaseline float32
	nPass        uint64
	nOther       uint64
}

// NewPassGuard builds a guard. maxPassFrac defaults to 0.4; baseline is the expected Pass solve-rate.
func NewPassGuard(maxPassFrac, solveBaseline float32) *PassGuard {
	if maxPassFrac <= 0 || maxPassFrac > 1 {
		maxPassFrac = 0.4
	}
	if solveBaseline <= 0 || solveBaseline >= 1 {
		solveBaseline = 0.6
	}
	return &PassGuard{maxPassFrac: maxPassFrac, solveBaseline: solveBaseline, solveEMA: solveBaseline}
}

// ObservePassAttempt records one Pass attempt (solved or not) for solve-rate monitoring.
func (g *PassGuard) ObservePassAttempt(solved bool) {
	g.mu.Lock()
	defer g.mu.Unlock()
	var s float32
	if solved {
		s = 1
	}
	const decay = 0.01
	g.solveEMA += decay * (s - g.solveEMA)
}

// SolveRateAnomalous reports whether the Pass solve-rate has climbed abnormally far above baseline
// (a sign adversaries are solving Pass at scale) — the caller treats this as a drift alarm and
// STOPS auto-ingesting Pass-human labels until reviewed.
func (g *PassGuard) SolveRateAnomalous() bool {
	g.mu.Lock()
	defer g.mu.Unlock()
	// abnormal if solve-rate exceeds baseline by a wide, sustained margin.
	return g.solveEMA > g.solveBaseline+0.25
}

// AdmitPassFraction caps how many Pass-human labels may join a batch given the count of other
// (catalog/CHALLENGE) labels, so the gold catalog stays dominant. Returns the max Pass labels to
// admit. Zero if the solve-rate is anomalous (poisoning suspected).
func (g *PassGuard) AdmitPassFraction(otherLabels int) int {
	if g.SolveRateAnomalous() {
		return 0
	}
	g.mu.Lock()
	defer g.mu.Unlock()
	// pass / (pass + other) <= maxPassFrac  →  pass <= other * f/(1-f)
	f := g.maxPassFrac
	if f >= 1 {
		return otherLabels
	}
	maxPass := float32(otherLabels) * f / (1 - f)
	return int(maxPass)
}

// Stats is a poisoning-guard observability snapshot.
func (g *PassGuard) SolveRate() float32 {
	g.mu.Lock()
	defer g.mu.Unlock()
	return g.solveEMA
}

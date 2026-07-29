// Package mlcorrect is the self-correcting / self-calibrating control plane for the behavioral
// model (SoT-42 Pillar A, continuous-learning loop). It is pure Go, no platform deps, sibling to
// internal/signals — it SENSES (drift, calibration) and PROPOSES (retrain candidates, threshold
// nudges), but it never mutates a verdict or the frozen scoring engine directly. Autonomy is
// bounded exactly as the research prescribes: sensing + reversible actions are automatic;
// promoting a new model and confirming ambiguous labels stay human-gated.
//
// calibrate.go is the "자가보정" core: an online, distribution-shift-robust controller that keeps
// the behavioral residual's decision threshold honest against a per-cohort human false-positive
// BUDGET — without retraining the model. It is Adaptive Conformal Inference (Gibbs & Candès, JMLR
// 2024): the threshold self-adjusts from REALIZED outcomes on oracle-confirmed sessions, with a
// long-run coverage guarantee that holds even under adversarial shift (no exchangeability
// assumption). Because the residual can only nudge toward CHALLENGE (never solo-DENY), a
// mis-calibration costs at most an extra beatable Pass — a bounded, accessibility-checked failure.
package mlcorrect

import "sync"

// ConformalThreshold is an ACI controller for one cohort. It holds the probability threshold θ at
// which the behavioral residual is allowed to "fire" (nudge toward CHALLENGE), driving the
// realized false-positive rate on ORACLE-CONFIRMED HUMAN sessions toward Budget.
//
// Update (per confirmed-human observation): θ ← clamp(θ + γ·(flagged − Budget)), where
// flagged = 1{p(bot) ≥ θ}. A human wrongly flagged pushes θ up (become less sensitive); a human
// correctly passed pushes θ down (become more sensitive) — long-run the flag rate on humans
// converges to Budget. Bot observations do not move θ (they are not the controlled quantity).
type ConformalThreshold struct {
	mu      sync.Mutex
	theta   float32 // current firing threshold in [minTheta, 1]
	budget  float32 // target human false-positive rate (e.g. 0.005)
	gamma   float32 // learning rate
	minθ    float32 // never drop below this (a floor keeps the residual from firing on everyone)
	fpEMA   float32 // decaying estimate of the realized human-FP rate (for observability)
	nHuman  uint64
	nBot    uint64
}

// NewConformalThreshold builds a controller. budget is the tolerated human-FP rate; theta0 is the
// starting threshold; gamma the adaptation rate. Sensible defaults are applied to out-of-range args.
func NewConformalThreshold(budget, theta0, gamma float32) *ConformalThreshold {
	if budget <= 0 || budget >= 1 {
		budget = 0.005
	}
	if theta0 <= 0 || theta0 >= 1 {
		theta0 = 0.5
	}
	if gamma <= 0 || gamma > 0.5 {
		gamma = 0.01
	}
	return &ConformalThreshold{theta: theta0, budget: budget, gamma: gamma, minθ: 0.5, fpEMA: budget}
}

// Threshold returns the current firing threshold θ.
func (c *ConformalThreshold) Threshold() float32 {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.theta
}

// Fires reports whether a p(bot) is at/above the current threshold (i.e. the residual should nudge).
func (c *ConformalThreshold) Fires(pBot float32) bool {
	return pBot >= c.Threshold()
}

// Observe updates the controller from ONE oracle-confirmed session. isHuman must come from an
// independent adjudicator (a solved Pass ⇒ human; a catalog/failed-CHALLENGE ⇒ bot) — NEVER from
// the model's own p(bot), which would create the self-consuming loop the research warns against.
func (c *ConformalThreshold) Observe(isHuman bool, pBot float32) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if !isHuman {
		c.nBot++
		return // bots are not the controlled quantity; only human-FP drives θ
	}
	c.nHuman++
	var flagged float32
	if pBot >= c.theta {
		flagged = 1
	}
	// ACI update: move θ so the human flag-rate tracks the budget.
	c.theta += c.gamma * (flagged - c.budget)
	if c.theta < c.minθ {
		c.theta = c.minθ
	}
	if c.theta > 1 {
		c.theta = 1
	}
	// decaying estimate of realized human-FP, for the operator dashboard / canary guard.
	const decay = 0.02
	c.fpEMA += decay * (flagged - c.fpEMA)
}

// Stats is an observability snapshot (no PII).
type Stats struct {
	Theta        float32
	Budget       float32
	HumanFPEst   float32 // EMA of realized human false-positive rate
	HumansSeen   uint64
	BotsSeen     uint64
}

// Snapshot returns the current controller state for the Ledger / canary guard.
func (c *ConformalThreshold) Snapshot() Stats {
	c.mu.Lock()
	defer c.mu.Unlock()
	return Stats{Theta: c.theta, Budget: c.budget, HumanFPEst: c.fpEMA, HumansSeen: c.nHuman, BotsSeen: c.nBot}
}

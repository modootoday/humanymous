package mlcorrect

import (
	"math/rand"
	"testing"
)

// TestConformal_HoldsBudget: fed a stream of confirmed humans whose p(bot) is uniform in [0,1],
// the controller must drive θ so the long-run human flag-rate tracks the budget. With a uniform
// p, P(flag)=1−θ, so θ should settle near 1−budget.
func TestConformal_HoldsBudget(t *testing.T) {
	c := NewConformalThreshold(0.05, 0.5, 0.02)
	rng := rand.New(rand.NewSource(7))
	for i := 0; i < 40000; i++ {
		c.Observe(true, rng.Float32())
	}
	th := c.Threshold()
	// θ should converge near 1 − budget = 0.95 (so ~5% of uniform humans exceed it).
	if th < 0.90 || th > 0.985 {
		t.Fatalf("θ did not converge near 1−budget: got %.3f (want ~0.95)", th)
	}
	// realized human-FP estimate should be near the budget.
	if fp := c.Snapshot().HumanFPEst; fp < 0.02 || fp > 0.10 {
		t.Fatalf("realized human-FP %.3f not near budget 0.05", fp)
	}
}

// TestConformal_RaisesWhenHumansOverFlagged: if confirmed humans keep scoring high p(bot) (the
// model is over-flagging real users), θ must climb toward its ceiling to protect them.
func TestConformal_RaisesWhenHumansOverFlagged(t *testing.T) {
	c := NewConformalThreshold(0.01, 0.5, 0.02)
	for i := 0; i < 5000; i++ {
		c.Observe(true, 0.9) // every human looks bot-ish → all would be flagged at θ<0.9
	}
	if th := c.Threshold(); th <= 0.9 {
		t.Fatalf("θ must rise above the over-flagged humans' p to protect them, got %.3f", th)
	}
}

// TestConformal_BotsDoNotMoveThreshold: only human-FP is the controlled quantity.
func TestConformal_BotsDoNotMoveThreshold(t *testing.T) {
	c := NewConformalThreshold(0.01, 0.7, 0.02)
	before := c.Threshold()
	for i := 0; i < 10000; i++ {
		c.Observe(false, 0.99) // bots, any p
	}
	if c.Threshold() != before {
		t.Fatalf("bot observations must not move θ: %.3f -> %.3f", before, c.Threshold())
	}
	if c.Snapshot().BotsSeen != 10000 {
		t.Errorf("bot count not tracked: %d", c.Snapshot().BotsSeen)
	}
}

// TestConformal_FloorHolds: θ never drops below the floor even if humans are never flagged.
func TestConformal_FloorHolds(t *testing.T) {
	c := NewConformalThreshold(0.05, 0.6, 0.05)
	for i := 0; i < 5000; i++ {
		c.Observe(true, 0.0) // humans always score p=0 → never flagged → θ drifts down
	}
	if th := c.Threshold(); th < 0.5 {
		t.Fatalf("θ dropped below the 0.5 floor: %.3f", th)
	}
}

package mlcorrect

import (
	"fmt"
	"sync"
)

// canary.go is the probationary AUTO-ROLLBACK watchdog (SELF-CORRECTION.md ⑧ CANARY + ⑨ auto-
// rollback). When an operator arms it, a freshly-served model is on PROBATION: every oracle-confirmed
// outcome re-checks the guard budgets, and the FIRST breach fires a rollback callback (the caller
// reverts to the previous model, or disables the residual to the heuristics-only baseline) with NO
// human in the loop — because rollback is the SAFE direction. If the model survives the probation
// sample breach-free it GRADUATES and the guard disarms.
//
// This is defensible precisely because the residual can only nudge toward CHALLENGE and rollback
// returns it to Abstain (the freeze-safe baseline): an unattended trip can only make detection MORE
// conservative, never break a verdict. Guards fire only on the SAFE side; promotion of a NEW model to
// full traffic still requires human dual-control (⑨).

// minCanaryObserve is how many confirmed humans must accrue since arming before the human-FP budget
// is judged, so the guard does not trip on early-sample noise.
const minCanaryObserve = 20

// CanaryBudget is the probation policy.
type CanaryBudget struct {
	MaxHumanFP            float32 // realized human-FP EMA ceiling during probation (e.g. 4× the ACI budget)
	ProbationHumans       uint64  // graduate after this many confirmed humans with no breach
	RollbackOnDrift       bool    // trip if the 2-of-3 drift gate fires
	RollbackOnPassAnomaly bool    // trip if the Pass solve-rate looks poisoned
}

type canaryState int

const (
	canaryOff canaryState = iota
	canaryArmed
	canaryTripped
	canaryGraduated
)

func (s canaryState) String() string {
	switch s {
	case canaryArmed:
		return "armed"
	case canaryTripped:
		return "tripped"
	case canaryGraduated:
		return "graduated"
	default:
		return "off"
	}
}

// CanaryGuard watches a model on probation and fires a one-shot rollback on the first guard breach.
// Safe for concurrent use.
type CanaryGuard struct {
	mu            sync.Mutex
	budget        CanaryBudget
	state         canaryState
	armedAtHumans uint64
	reason        string
	onRollback    func(reason string)
}

// NewCanaryGuard builds a guard with sane defaults for out-of-range budgets. onRollback is invoked
// exactly once, off the lock, on the first breach.
func NewCanaryGuard(b CanaryBudget, onRollback func(string)) *CanaryGuard {
	if b.MaxHumanFP <= 0 || b.MaxHumanFP >= 1 {
		b.MaxHumanFP = 0.02
	}
	if b.ProbationHumans == 0 {
		b.ProbationHumans = 200
	}
	return &CanaryGuard{budget: b, onRollback: onRollback}
}

// Arm starts probation from the current confirmed-human count.
func (g *CanaryGuard) Arm(humansSeen uint64) {
	g.mu.Lock()
	defer g.mu.Unlock()
	g.state = canaryArmed
	g.armedAtHumans = humansSeen
	g.reason = ""
}

// evaluate is called on each oracle confirmation with the current control-plane metrics. It trips
// (rollback) on the first breach, or graduates once the probation sample is met breach-free. A no-op
// unless armed.
func (g *CanaryGuard) evaluate(humansSeen uint64, humanFP float32, driftFired, passAnomalous bool) {
	g.mu.Lock()
	if g.state != canaryArmed {
		g.mu.Unlock()
		return
	}
	sinceArm := humansSeen - g.armedAtHumans
	var trip string
	switch {
	case sinceArm >= minCanaryObserve && humanFP > g.budget.MaxHumanFP:
		trip = fmt.Sprintf("human-FP %.4f over canary budget %.4f", humanFP, g.budget.MaxHumanFP)
	case g.budget.RollbackOnDrift && driftFired:
		trip = "drift gate fired during probation"
	case g.budget.RollbackOnPassAnomaly && passAnomalous:
		trip = "Pass solve-rate anomaly (suspected poisoning) during probation"
	}
	if trip != "" {
		g.state = canaryTripped
		g.reason = trip
		cb := g.onRollback
		g.mu.Unlock()
		if cb != nil {
			cb(trip) // fire the rollback off the lock
		}
		return
	}
	if sinceArm >= g.budget.ProbationHumans {
		g.state = canaryGraduated
	}
	g.mu.Unlock()
}

// CanaryStats is the observability view (no PII).
type CanaryStats struct {
	State               string `json:"state"`  // off | armed | tripped | graduated
	Reason              string `json:"reason,omitempty"`
	HumansIntoProbation uint64 `json:"humansIntoProbation"`
	ProbationHumans     uint64 `json:"probationHumans"`
}

// Snapshot returns the guard state given the current confirmed-human count.
func (g *CanaryGuard) Snapshot(humansSeen uint64) CanaryStats {
	g.mu.Lock()
	defer g.mu.Unlock()
	var into uint64
	if g.state != canaryOff && humansSeen >= g.armedAtHumans {
		into = humansSeen - g.armedAtHumans
	}
	return CanaryStats{
		State:               g.state.String(),
		Reason:              g.reason,
		HumansIntoProbation: into,
		ProbationHumans:     g.budget.ProbationHumans,
	}
}

package gate

import (
	"strconv"
	"sync"
)

// epoch.go rotates the verdict-token epoch (SoT-28 WS6 / SoT-22 §4). Each token
// carries the epoch it was minted under; the edge accepts only the CURRENT and
// PREVIOUS epoch. Advancing the epoch therefore revokes every token older than
// one rotation — bounding how long a cloned/stolen token can be replayed, and
// giving an emergency "bump" that invalidates all outstanding tokens at once.
// The control plane (issuer) and the edge (verifier) share one manager so they
// stay in lock-step.

// EpochManager holds the current + previous token epoch.
type EpochManager struct {
	mu   sync.RWMutex
	cur  string
	prev string
	n    int
}

// NewEpochManager starts at epoch "e1" with no accepted predecessor.
func NewEpochManager() *EpochManager { return &EpochManager{cur: "e1", n: 1} }

// orDefaultEpochs returns em or a fresh manager.
func orDefaultEpochs(em *EpochManager) *EpochManager {
	if em != nil {
		return em
	}
	return NewEpochManager()
}

// Current returns the epoch new tokens are minted under.
func (e *EpochManager) Current() string {
	e.mu.RLock()
	defer e.mu.RUnlock()
	return e.cur
}

// Accepted returns the epochs a token may present (current + previous).
func (e *EpochManager) Accepted() []string {
	e.mu.RLock()
	defer e.mu.RUnlock()
	if e.prev == "" {
		return []string{e.cur}
	}
	return []string{e.cur, e.prev}
}

// Advance rotates: the current epoch becomes the accepted predecessor and a new
// current is minted. Tokens two epochs old are no longer accepted.
func (e *EpochManager) Advance() {
	e.mu.Lock()
	defer e.mu.Unlock()
	e.prev = e.cur
	e.n++
	e.cur = "e" + strconv.Itoa(e.n)
}

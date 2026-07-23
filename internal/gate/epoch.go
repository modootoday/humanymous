package gate

import (
	"strconv"
	"sync"
	"time"
)

// epoch.go rotates the verdict-token epoch (SoT-28 WS6 / SoT-22 §4). Each token carries the
// epoch it was minted under; the edge accepts only the CURRENT and PREVIOUS epoch, so a token
// is revoked once the epoch advances twice — bounding how long a cloned/stolen token can be
// replayed.
//
// The epoch is derived from a WALL-CLOCK time bucket (deep-review): a per-node counter
// advanced by each node's own 15-minute ticker diverged across a fleet (nodes booted at
// different times held different counters), so a token minted on one node failed the epoch
// check on another — defeating the cross-node fast-path a shared token key exists to enable.
// A wall-clock bucket makes every node compute the SAME current/previous epoch independently,
// exactly like the origin-cloaking epoch (guard.go). A manual `bump` offset remains for an
// emergency fleet-wide revocation (all nodes bump together).

// tokenEpochWindow is the verdict-token epoch bucket (seconds). One bucket + one grace bucket
// bound a token's accepted lifetime to ~2 windows.
const tokenEpochWindow = 15 * 60

// EpochManager derives the token epoch from the wall clock plus an emergency bump offset.
type EpochManager struct {
	mu   sync.RWMutex
	bump int64
}

// NewEpochManager returns a wall-clock-driven epoch manager.
func NewEpochManager() *EpochManager { return &EpochManager{} }

// orDefaultEpochs returns em or a fresh manager.
func orDefaultEpochs(em *EpochManager) *EpochManager {
	if em != nil {
		return em
	}
	return NewEpochManager()
}

// epochAt returns the epoch string for the current wall-clock bucket shifted by `offset`
// buckets and the manual bump.
func (e *EpochManager) epochAt(offset int64) string {
	e.mu.RLock()
	b := e.bump
	e.mu.RUnlock()
	return "e" + strconv.FormatInt(time.Now().Unix()/tokenEpochWindow+b+offset, 10)
}

// Current returns the epoch new tokens are minted under.
func (e *EpochManager) Current() string { return e.epochAt(0) }

// Accepted returns the epochs a token may present (current + previous bucket).
func (e *EpochManager) Accepted() []string { return []string{e.epochAt(0), e.epochAt(-1)} }

// Advance performs a MANUAL, PER-NODE emergency bump: it shifts THIS node's epoch forward one
// step (in-memory, not persisted, not propagated). It is NOT a fleet-wide revocation on its
// own — in a fleet every node would have to bump together, and even then prior-epoch tokens
// survive through the one grace bucket. The genuine fleet-wide "revoke everything now" lever
// is rotating the shared token key (HMN_TOKEN_KEY). Normal rotation is automatic by the wall
// clock and needs no ticker.
func (e *EpochManager) Advance() {
	e.mu.Lock()
	e.bump++
	e.mu.Unlock()
}

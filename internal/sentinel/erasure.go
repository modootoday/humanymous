package sentinel

import (
	"sync"
	"time"
)

// erasure.go implements the cancellable hold-window before a crypto-shred
// commits (SoT-28 §5/WS3). A dual-control-approved erasure is not executed
// immediately: it is SCHEDULED with a grace window during which it can be
// cancelled (e.g. the "erased" subject turns out to be an active-incident
// attacker whose linkage must be kept). Only when the window elapses is the
// linkage key actually destroyed, and a signed erasure certificate is sealed.

// ScheduledShred is an approved erasure awaiting its hold window.
type ScheduledShred struct {
	ID         string
	Subject    string // resolved subject id (never a raw client-supplied value)
	Requester  string
	Approver   string
	LegalBasis string
	ExecuteAt  time.Time
}

// ShredQueue holds scheduled erasures.
type ShredQueue struct {
	mu    sync.Mutex
	items map[string]ScheduledShred
	hold  time.Duration
	nowFn func() time.Time
}

// NewShredQueue builds a queue with a hold (grace) window.
func NewShredQueue(hold time.Duration) *ShredQueue {
	return &ShredQueue{items: map[string]ScheduledShred{}, hold: hold, nowFn: time.Now}
}

// Schedule enqueues an erasure to execute after the hold window.
func (q *ShredQueue) Schedule(subject, requester, approver, legalBasis string) ScheduledShred {
	now := q.nowFn()
	s := ScheduledShred{ID: randHexN(10), Subject: subject, Requester: requester, Approver: approver,
		LegalBasis: legalBasis, ExecuteAt: now.Add(q.hold)}
	q.mu.Lock()
	q.items[s.ID] = s
	q.mu.Unlock()
	return s
}

// Cancel removes a scheduled erasure before it executes.
func (q *ShredQueue) Cancel(id string) (ScheduledShred, bool) {
	q.mu.Lock()
	defer q.mu.Unlock()
	s, ok := q.items[id]
	delete(q.items, id)
	return s, ok
}

// Due returns and REMOVES the erasures whose hold window has elapsed (the caller
// executes them).
func (q *ShredQueue) Due(now time.Time) []ScheduledShred {
	q.mu.Lock()
	defer q.mu.Unlock()
	var out []ScheduledShred
	for id, s := range q.items {
		if !now.Before(s.ExecuteAt) {
			out = append(out, s)
			delete(q.items, id)
		}
	}
	return out
}

// List returns the still-pending scheduled erasures (for the console).
func (q *ShredQueue) List() []ScheduledShred {
	q.mu.Lock()
	defer q.mu.Unlock()
	out := make([]ScheduledShred, 0, len(q.items))
	for _, s := range q.items {
		out = append(out, s)
	}
	return out
}

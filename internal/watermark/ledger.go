package watermark

import (
	"sync"
	"time"
)

// ledger.go maps an issued tagId to its Payload so a recovered carrier can be
// traced back to a session (SoT-08 §4). In-memory with TTL; swap for a database
// in production. It also allocates monotonic tagIds.

// Record is a stored watermark issuance.
type Record struct {
	Payload  Payload
	MIME     string
	Channel  string
	IssuedAt time.Time
	IPHash   string
}

// Ledger stores issuances keyed by tagId.
type Ledger struct {
	mu   sync.Mutex
	m    map[uint32]Record
	next uint32
	ttl  time.Duration
}

// NewLedger returns a ledger with the given TTL.
func NewLedger(ttl time.Duration) *Ledger {
	return &Ledger{m: map[uint32]Record{}, next: 1, ttl: ttl}
}

// Allocate reserves the next tagId.
func (l *Ledger) Allocate() uint32 {
	l.mu.Lock()
	defer l.mu.Unlock()
	id := l.next
	l.next++
	return id
}

// Put records an issuance.
func (l *Ledger) Put(r Record) {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.m[r.Payload.TagID] = r
}

// Get returns the record for a tagId.
func (l *Ledger) Get(tagID uint32) (Record, bool) {
	l.mu.Lock()
	defer l.mu.Unlock()
	r, ok := l.m[tagID]
	return r, ok
}

// GC drops records older than the TTL.
func (l *Ledger) GC(now time.Time) {
	l.mu.Lock()
	defer l.mu.Unlock()
	for id, r := range l.m {
		if now.Sub(r.IssuedAt) > l.ttl {
			delete(l.m, id)
		}
	}
}

// TraceResult is returned when a leaked resource is successfully traced.
type TraceResult struct {
	SessionID string    `json:"sessionId"`
	AssetID   string    `json:"assetId"`
	MIME      string    `json:"mime"`
	Channel   string    `json:"channel"`
	IssuedAt  time.Time `json:"issuedAt"`
	Forged    bool      `json:"forged"`
}

// Trace recovers a carrier from leaked bytes, looks it up, and verifies the HMAC.
func (l *Ledger) Trace(masterKey []byte, mime string, leaked []byte) (*TraceResult, error) {
	carrier, channel, err := Recover(mime, leaked)
	if err != nil {
		return nil, err
	}
	tagID, tagBytes, err := SplitCarrier(carrier)
	if err != nil {
		return nil, err
	}
	rec, ok := l.Get(tagID)
	if !ok {
		return nil, ErrNoFrame
	}
	forged := !Verify(masterKey, rec.Payload, tagBytes)
	return &TraceResult{
		SessionID: rec.Payload.SessionID,
		AssetID:   rec.Payload.AssetID,
		MIME:      rec.MIME,
		Channel:   channel,
		IssuedAt:  rec.IssuedAt,
		Forged:    forged,
	}, nil
}

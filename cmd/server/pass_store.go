package main

import (
	"crypto/rand"
	"encoding/hex"
	"strconv"
	"sync"
	"time"
)

// pass_store.go holds the transient per-session Pass state, the wargame metrics,
// and the anti-replay trace registry (PLAN-07 R12: split out of pass_handler.go by
// concern — state & storage here, HTTP handlers in pass_handler.go, scoring fusion
// in pass_scoring.go, proof verification in pass_proof.go). No behavior change.

const (
	passWindow   = 30 // seconds per bucket (challenge TTL granularity)
	passMaxTries = 4  // hard retry cap per session (SoT-36 §2.3: fail -> new instance)
)

// passSession is the per-session Pass state (in-memory; the challenge is transient).
type passSession struct {
	bucket        uint64
	instance      uint64 // increments on each /new so every puzzle is fresh
	difficulty    int
	issuedAt      time.Time
	tries         int
	solved        bool
	nonce         string // per-instance challenge nonce; binds the crypto proof + the solve
	powDifficulty int    // PoW leading-zero-bit target for this instance
	powOK         bool   // the crypto axis (①) has been satisfied for this instance
	velLevel      int    // highest Pass-velocity level flagged this session (0 ok, 1 elevated, 2 flood)
	attestReq     bool   // this instance additionally requires a rate-limited attestation token
}

// passStore holds transient per-session Pass state + the wargame metrics, guarded
// by a mutex. metrics is the blue team's evidence base (SoT-36 §8): attempts
// counted by label × outcome × difficulty. traces is the anti-replay registry:
// the digest of every accepted motor trace, so a captured human trace can be
// replayed at most once (SoT-36 §5).
type passStore struct {
	mu      sync.Mutex
	m       map[string]*passSession
	metrics map[string]int       // "label|outcome|difficulty" -> count
	traces  map[string]time.Time // motor-trace digest -> first-seen (anti-replay)
}

func newPassStore() *passStore {
	return &passStore{
		m:       map[string]*passSession{},
		metrics: map[string]int{},
		traces:  map[string]time.Time{},
	}
}

// reserveTrace registers a motor-trace digest and reports whether it is FRESH
// (never seen inside the retention window). A replayed capture — identical raw
// timings — collides on the digest and is rejected. Old entries are swept lazily.
func (p *passStore) reserveTrace(digest string, now time.Time) bool {
	p.mu.Lock()
	defer p.mu.Unlock()
	for k, seen := range p.traces {
		if now.Sub(seen) > 10*time.Minute {
			delete(p.traces, k)
		}
	}
	if _, exists := p.traces[digest]; exists {
		return false
	}
	p.traces[digest] = now
	return true
}

// passNonce mints a fresh per-instance challenge nonce.
func passNonce() (string, error) {
	var b [18]byte
	if _, err := rand.Read(b[:]); err != nil {
		return "", err
	}
	return hex.EncodeToString(b[:]), nil
}

// record logs one attempt into the wargame metrics (caller need not hold the lock).
func (p *passStore) record(label string, difficulty int, passed bool) {
	outcome := "fail"
	if passed {
		outcome = "pass"
	}
	key := label + "|" + outcome + "|d" + strconv.Itoa(difficulty) // PLAN-07 R7
	p.mu.Lock()
	p.metrics[key]++
	p.mu.Unlock()
}

// maxPassSessions caps the in-memory per-session Pass map (audit LOW-3: bound the
// resource so a churn of fresh session ids cannot grow it without limit).
const maxPassSessions = 50000

func (p *passStore) get(sid string) *passSession {
	p.mu.Lock()
	defer p.mu.Unlock()
	if len(p.m) > maxPassSessions { // lazy sweep of stale sessions under pressure
		cut := time.Now().Add(-30 * time.Minute)
		for k, s := range p.m {
			if s.issuedAt.Before(cut) {
				delete(p.m, k)
			}
		}
	}
	s := p.m[sid]
	if s == nil {
		s = &passSession{issuedAt: time.Now()} // stamp creation so a fresh session isn't swept
		p.m[sid] = s
	}
	return s
}

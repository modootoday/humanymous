package trafficguard

import (
	"sync"
	"time"
)

// log.go stores per-session traffic records in a bounded ring buffer and
// correlates TLS handshakes (keyed by remote address) with the HTTP requests
// that later reveal the session id (SoT-12 §4 backfill).

const maxPerSession = 64

// Log is the concurrent traffic store.
type Log struct {
	mu       sync.Mutex
	sessions map[string][]TrafficRecord // sessionId -> records
	byAddr   map[string]TrafficRecord   // remoteAddr -> last TLS record (pre-session)
	ttl      time.Duration
	seen     map[string]time.Time
}

// NewLog returns a traffic log with the given per-session TTL.
func NewLog(ttl time.Duration) *Log {
	return &Log{
		sessions: map[string][]TrafficRecord{},
		byAddr:   map[string]TrafficRecord{},
		seen:     map[string]time.Time{},
		ttl:      ttl,
	}
}

// RecordTLS stores a TLS handshake keyed by remote address; the session id is
// unknown at handshake time and is backfilled by the first HTTP request.
func (l *Log) RecordTLS(r TrafficRecord) {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.byAddr[r.RemoteAddr] = r
}

// RecordHTTP appends an HTTP record to its session, attaching the TLS record
// captured for the same remote address.
func (l *Log) RecordHTTP(r TrafficRecord) {
	l.mu.Lock()
	defer l.mu.Unlock()
	if tls, ok := l.byAddr[r.RemoteAddr]; ok {
		r.JA3, r.JA4, r.JA4Engine = tls.JA3, tls.JA4, tls.JA4Engine
		r.ALPN, r.TLSVersion = tls.ALPN, tls.TLSVersion
		r.ExtOrder = tls.ExtOrder
	}
	recs := l.sessions[r.SessionID]
	if len(recs) >= maxPerSession {
		recs = recs[1:]
	}
	l.sessions[r.SessionID] = append(recs, r)
	l.seen[r.SessionID] = r.TS
}

// Session returns a copy of a session's records (oldest first).
func (l *Log) Session(sid string) []TrafficRecord {
	l.mu.Lock()
	defer l.mu.Unlock()
	src := l.sessions[sid]
	return append([]TrafficRecord(nil), src...)
}

// GC evicts sessions and stale address entries older than the TTL.
func (l *Log) GC(now time.Time) {
	l.mu.Lock()
	defer l.mu.Unlock()
	for sid, ts := range l.seen {
		if now.Sub(ts) > l.ttl {
			delete(l.sessions, sid)
			delete(l.seen, sid)
		}
	}
	for addr, r := range l.byAddr {
		if now.Sub(r.TS) > l.ttl {
			delete(l.byAddr, addr)
		}
	}
}

// Reset clears all traffic-log state (SoT-30 §10 run isolation).
func (l *Log) Reset() {
	l.mu.Lock()
	l.sessions = map[string][]TrafficRecord{}
	l.byAddr = map[string]TrafficRecord{}
	l.seen = map[string]time.Time{}
	l.mu.Unlock()
}

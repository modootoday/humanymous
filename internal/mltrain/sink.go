package mltrain

import (
	"bufio"
	"encoding/json"
	"os"
	"path/filepath"
	"sync"
)

// sink.go collects oracle-confirmed labeled traces for offline retraining — closing the
// self-correcting data loop (SoT-42 Pillar A). The live Core appends a Record here every time an
// INDEPENDENT oracle confirms a session (a solved Pass ⇒ human), and cmd/mlcorrect-train later reads
// the file as its -oracle input. It is operator-gated: the Core constructs a sink ONLY when an
// explicit lab/training directory is configured, never by default.
//
// Design constraints: a training-data sink must never block or fail a live request, so callers treat
// Append errors as non-fatal (log-and-continue). It is append-only (no rewrite of past labels) and
// thread-safe. It writes aggregate BehaviorSummary values only — the same biometric aggregates the
// engine already computes — never raw event streams or identifiers.

// TraceSink is a thread-safe, append-only JSONL writer for labeled behavioral traces.
type TraceSink struct {
	mu  sync.Mutex
	f   *os.File
	w   *bufio.Writer
	enc *json.Encoder
	n   uint64
}

// NewTraceSink opens (creating if needed) a JSONL file for appending. The parent directory is
// created with owner-only permissions (0700) because traces are training data, not a public surface.
func NewTraceSink(path string) (*TraceSink, error) {
	if dir := filepath.Dir(path); dir != "" {
		if err := os.MkdirAll(dir, 0o700); err != nil {
			return nil, err
		}
	}
	f, err := os.OpenFile(path, os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0o600)
	if err != nil {
		return nil, err
	}
	w := bufio.NewWriter(f)
	return &TraceSink{f: f, w: w, enc: json.NewEncoder(w)}, nil
}

// Append writes one record as a JSONL line and flushes it. Oracle confirmations (Pass solves) are
// rare, so flushing per record buys durability at negligible cost. A nil sink is a no-op so callers
// need not branch.
func (s *TraceSink) Append(r Record) error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	if err := s.enc.Encode(&r); err != nil {
		return err
	}
	s.n++
	return s.w.Flush()
}

// Count returns how many records this sink has appended (observability).
func (s *TraceSink) Count() uint64 {
	if s == nil {
		return 0
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.n
}

// Close flushes and closes the underlying file. Safe on a nil sink.
func (s *TraceSink) Close() error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.w != nil {
		_ = s.w.Flush()
	}
	if s.f != nil {
		return s.f.Close()
	}
	return nil
}

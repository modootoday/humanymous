package audit

import "fmt"

// sink.go routes every enforcement dispatch through a single typed sink that
// refuses to act without a paired audit record (SoT-18 §7). This makes
// "every enforcement path emits an event" a STRUCTURAL guarantee — you cannot
// call the action without also sealing the record — rather than a CI grep that
// static analysis cannot enforce in Go.

// Action is the side effect an enforcement decision performs (block, challenge,
// pass, ...). It runs ONLY after the record is durably sealed into the chain.
type Action func()

// Sink couples the audit Log to enforcement so no action happens un-audited.
type Sink struct {
	log *Log
}

// NewSink wraps a Log as the enforcement sink.
func NewSink(log *Log) *Sink { return &Sink{log: log} }

// EmitAndAct seals the record FIRST (audit-before-effect, SoT-19 step 13) and
// only then runs the action. A nil record is a programming error and panics —
// that is the point: an enforcement path with no record cannot compile-around
// this sink without deliberately bypassing it.
func (s *Sink) EmitAndAct(r Record, act Action) Record {
	if r.EventType == "" {
		panic(fmt.Sprintf("audit: EmitAndAct called with no event_type (verdict=%q action=%q) — enforcement without audit is forbidden", r.Verdict, r.Action))
	}
	sealed := s.log.Append(r)
	if act != nil {
		act()
	}
	return sealed
}

// Emit seals an informational record with no side effect.
func (s *Sink) Emit(r Record) Record {
	return s.EmitAndAct(r, nil)
}

// Log exposes the underlying chain (for export/verification).
func (s *Sink) Log() *Log { return s.log }

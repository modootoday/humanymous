package audit

import "context"

// sink_record.go defines the pluggable durable/projection destination for sealed
// audit records (SoT-32). The local WAL is the authority on the synchronous seal
// path (its error fails the append closed); Redis/ClickHouse adapters are async,
// replayable projections that must never block or fail the seal.

// RecordSink is a destination for sealed records. Implementations MUST accept
// records in the seq order Append delivers them.
type RecordSink interface {
	// AppendRecord durably accepts one sealed record. For the AUTHORITY sink this
	// returns only after the record is on stable storage (fsync), because its
	// success is what releases the enforcement side effect (EmitAndAct). For a
	// PROJECTION sink it may enqueue and return immediately.
	AppendRecord(ctx context.Context, r Record) error
	// Flush blocks until previously-appended records are durable/shipped.
	Flush(ctx context.Context) error
	// Close flushes and releases resources.
	Close() error
}

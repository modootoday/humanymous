package audit

import (
	"context"
	"testing"
)

// A fixed signing seed simulates the persisted keystore identity (SoT-28 WS8):
// the STH signing key MUST be stable across a restart for replayed checkpoints to
// verify — in production the -keystore provides it.
var testSeed = make([]byte, 32)

func walLog(t *testing.T, dir string, ringCap int) *Log {
	t.Helper()
	w, err := NewWALSink(dir)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = w.Close() }) // release the file so Windows TempDir cleanup can unlink it
	return NewLog(Config{NodeID: "n1", HMACKey: []byte("k"), CheckpointEvery: 4, WAL: w, RingCap: ringCap, SigningSeed: testSeed})
}

// A WAL-backed log persists records; Records() reads them back and the chain
// self-verifies.
func TestWALRoundTripAndVerify(t *testing.T) {
	dir := t.TempDir()
	l := walLog(t, dir, 0)
	for i := 0; i < 10; i++ {
		l.Append(sampleRecord(i))
	}
	recs := l.Records()
	if len(recs) != 10 {
		t.Fatalf("Records() = %d, want 10", len(recs))
	}
	if l.Len() != 10 {
		t.Fatalf("Len() = %d, want 10", l.Len())
	}
	if res := l.SelfVerify(); !res.OK {
		t.Fatalf("SelfVerify failed: %+v", res)
	}
}

// Restart: a fresh Log over the same WAL dir resumes seq/prev_hash and continues
// the chain; the full chain across the restart still verifies.
func TestWALReplayAcrossRestart(t *testing.T) {
	dir := t.TempDir()
	l1 := walLog(t, dir, 0)
	for i := 0; i < 6; i++ {
		l1.Append(sampleRecord(i))
	}
	l1.Checkpoint()
	if err := l1.wal.Close(); err != nil {
		t.Fatal(err)
	}

	// Reopen — simulates a process restart.
	l2 := walLog(t, dir, 0)
	if l2.Len() != 6 {
		t.Fatalf("after replay Len() = %d, want 6", l2.Len())
	}
	// Continue the chain; new records link onto the replayed head.
	for i := 6; i < 12; i++ {
		l2.Append(sampleRecord(i))
	}
	if l2.Len() != 12 {
		t.Fatalf("Len() = %d, want 12", l2.Len())
	}
	if res := l2.SelfVerify(); !res.OK {
		t.Fatalf("post-restart SelfVerify failed at seq %d: %s", res.AtSeq, res.Detail)
	}
	// Seq is contiguous (no gap introduced by replay).
	recs := l2.Records()
	for i := range recs {
		if recs[i].Seq != uint64(i+1) {
			t.Fatalf("seq gap at index %d: %d", i, recs[i].Seq)
		}
	}
}

// Segment rotation: a tiny rotate threshold produces multiple segments that
// ReadAll re-stitches in order.
func TestWALSegmentRotation(t *testing.T) {
	dir := t.TempDir()
	w, err := NewWALSink(dir)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = w.Close() })
	w.rotateBytes = 200 // force frequent rotation
	l := NewLog(Config{NodeID: "n1", HMACKey: []byte("k"), CheckpointEvery: 4, WAL: w, RingCap: 0, SigningSeed: testSeed})
	for i := 0; i < 30; i++ {
		l.Append(sampleRecord(i))
	}
	segs, _ := listSegments(dir)
	if len(segs) < 2 {
		t.Fatalf("expected multiple segments, got %d", len(segs))
	}
	if l.Len() != 30 || len(l.Records()) != 30 {
		t.Fatalf("rotation lost records: Len=%d Records=%d", l.Len(), len(l.Records()))
	}
	if res := l.SelfVerify(); !res.OK {
		t.Fatalf("SelfVerify after rotation failed: %+v", res)
	}
}

// The in-memory window is bounded by RingCap while the full chain (WAL) is intact.
func TestWALRingBoundsMemory(t *testing.T) {
	dir := t.TempDir()
	l := walLog(t, dir, 8)
	for i := 0; i < 40; i++ {
		l.Append(sampleRecord(i))
	}
	if got := len(l.records); got > 2*8 {
		t.Fatalf("in-memory window not bounded: %d rows (cap*2=16)", got)
	}
	if l.Len() != 40 || len(l.Records()) != 40 {
		t.Fatalf("full chain lost: Len=%d Records=%d", l.Len(), len(l.Records()))
	}
	if n := len(l.Recent(5)); n != 5 {
		t.Fatalf("Recent(5) = %d, want 5", n)
	}
}

// A WAL write failure fails closed: Append panics (no un-durable record advances
// the chain), which net/http recovers into a 500 at the request layer.
func TestWALAppendFailClosed(t *testing.T) {
	dir := t.TempDir()
	l := walLog(t, dir, 0)
	l.Append(sampleRecord(0))
	_ = l.wal.Close() // subsequent writes to the closed segment error
	defer func() {
		if recover() == nil {
			t.Fatal("expected Append to panic (fail closed) on WAL write error")
		}
	}()
	l.Append(sampleRecord(1))
}

var _ = context.Background

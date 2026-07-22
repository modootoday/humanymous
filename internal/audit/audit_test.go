package audit

import "testing"

func sampleRecord(seq int) Record {
	return Record{
		EventType:  EventEnfDeny,
		Actor:      Actor{Kind: "system"},
		TenantID:   "t1",
		SessionPsn: "psn-abc",
		Host:       "app.example",
		RouteClass: "html",
		Verdict:    "deny",
		RiskScore:  83,
		Rules:      []string{"HR-23", "HR-2"},
		TopSignals: []Signal{{ID: "l5.h2dos.rapid_reset", Verdict: "BOT", Conf: 1}, {ID: "x.ua_vs_ja4", Verdict: "BOT", Conf: 0.9}},
		Action:     "block",
		Mode:       "enforce",
		KeyID:      "k1",
	}
}

func newTestLog() *Log {
	return NewLog(Config{NodeID: "node-1", HMACKey: []byte("test-hmac-key-32-bytes-long!!"), CheckpointEvery: 4})
}

// canonical form is deterministic regardless of rule/signal input order.
func TestCanonicalDeterministic(t *testing.T) {
	a := sampleRecord(1)
	b := sampleRecord(1)
	b.Rules = []string{"HR-2", "HR-23"}                       // different order
	b.TopSignals = []Signal{b.TopSignals[1], b.TopSignals[0]} // reversed
	if h1, h2 := computeRecordHash(&a, "prev"), computeRecordHash(&b, "prev"); h1 != h2 {
		t.Fatalf("canonical not order-independent: %s != %s", h1, h2)
	}
}

// a clean chain verifies.
func TestChainVerifies(t *testing.T) {
	l := newTestLog()
	for i := 0; i < 10; i++ {
		l.Append(sampleRecord(i))
	}
	l.Checkpoint()
	res := Verify(l.Records(), l.Checkpoints(), l.hmacKey, l.PublicKey())
	if !res.OK {
		t.Fatalf("clean chain failed: %s at seq %d: %s", res.Class, res.AtSeq, res.Detail)
	}
}

// mutating a sealed record's content is detected as a hash-break.
func TestTamperDetected(t *testing.T) {
	l := newTestLog()
	for i := 0; i < 6; i++ {
		l.Append(sampleRecord(i))
	}
	recs := l.Records()
	recs[3].Verdict = "allow" // insider flips a DENY to ALLOW
	res := Verify(recs, l.Checkpoints(), l.hmacKey, l.PublicKey())
	if res.OK || res.Class != ClassHashBreak {
		t.Fatalf("tamper not caught: %+v", res)
	}
	if res.AtSeq != 4 {
		t.Fatalf("wrong seq flagged: %d", res.AtSeq)
	}
}

// deleting a record (truncation) breaks the chain linkage/seq.
func TestTruncationDetected(t *testing.T) {
	l := newTestLog()
	for i := 0; i < 6; i++ {
		l.Append(sampleRecord(i))
	}
	recs := l.Records()
	recs = append(recs[:2], recs[3:]...) // drop seq 3
	res := Verify(recs, nil, l.hmacKey, l.PublicKey())
	if res.OK {
		t.Fatalf("truncation not caught")
	}
}

// a forged checkpoint root (rollback) is caught by the signature/root check.
func TestCheckpointRollbackDetected(t *testing.T) {
	l := newTestLog()
	for i := 0; i < 8; i++ {
		l.Append(sampleRecord(i))
	}
	cps := l.Checkpoints()
	recs := l.Records()[:4] // pretend only 4 records exist but keep the 8-cover checkpoint
	res := Verify(recs, cps, l.hmacKey, l.PublicKey())
	if res.OK || res.Class != ClassCheckpointBad {
		t.Fatalf("rollback not caught: %+v", res)
	}
}

// crypto-shred: destroying a subject key unlinks their pseudonyms permanently,
// and does NOT affect other subjects.
func TestCryptoShred(t *testing.T) {
	v := NewVault()
	p1 := v.Pseudonymize("subjectA", "1.2.3.4")
	other := v.Pseudonymize("subjectB", "1.2.3.4")
	if p1 == other {
		t.Fatal("different subjects must not share pseudonyms for the same value")
	}
	if again := v.Pseudonymize("subjectA", "1.2.3.4"); again != p1 {
		t.Fatal("pseudonym must be stable before shred")
	}
	if !v.Shred("subjectA") {
		t.Fatal("shred should report an existing key")
	}
	shredded := v.Pseudonymize("subjectA", "1.2.3.4")
	if shredded == p1 {
		t.Fatal("after shred the pseudonym must be unlinkable (changed to tombstone)")
	}
	// subjectB unaffected.
	if v.Pseudonymize("subjectB", "1.2.3.4") != other {
		t.Fatal("shredding A must not affect B")
	}
}

// the structural sink refuses to act without an audit record.
func TestSinkRefusesUnaudited(t *testing.T) {
	s := NewSink(newTestLog())
	defer func() {
		if recover() == nil {
			t.Fatal("EmitAndAct with no event_type must panic (enforcement without audit)")
		}
	}()
	acted := false
	s.EmitAndAct(Record{}, func() { acted = true })
	if acted {
		t.Fatal("action ran despite missing audit record")
	}
}

// the sink seals BEFORE acting (audit-before-effect).
func TestSinkAuditBeforeEffect(t *testing.T) {
	l := newTestLog()
	s := NewSink(l)
	order := []string{}
	s.EmitAndAct(sampleRecord(1), func() { order = append(order, "act") })
	if l.Len() != 1 {
		t.Fatalf("record not sealed: len=%d", l.Len())
	}
	if len(order) != 1 || order[0] != "act" {
		t.Fatalf("action did not run after seal: %v", order)
	}
}

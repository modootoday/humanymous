package audit

import "testing"

// WS8: a witnessed chain carries valid co-signatures and verifies against the
// witness public key.
func TestWitnessCoSign(t *testing.T) {
	w := NewWitness()
	l := NewLog(Config{NodeID: "n", HMACKey: []byte("k"), CheckpointEvery: 4, Witness: w})
	for i := 0; i < 10; i++ {
		l.Append(sampleRecord(i))
	}
	l.Checkpoint()
	cps := l.Checkpoints()
	if len(cps) == 0 {
		t.Fatal("expected checkpoints")
	}
	for _, cp := range cps {
		if cp.WitnessSig == "" {
			t.Fatalf("checkpoint at %d missing witness co-signature", cp.TreeSize)
		}
	}
	if at, ok := VerifyWitness(cps, l.WitnessPublicKey()); !ok {
		t.Fatalf("witnessed chain should verify, failed at %d", at)
	}
}

// WS8: the witness refuses to co-sign a rewrite (root changed at a witnessed
// tree size) or a rollback (tree size regressed) — the exact moves a history
// forgery requires.
func TestWitnessRejectsRewriteAndRollback(t *testing.T) {
	w := NewWitness()
	if _, err := w.CounterSign(Checkpoint{TreeSize: 8, Root: "R1"}); err != nil {
		t.Fatalf("first sign should succeed: %v", err)
	}
	// Re-signing the same size with a DIFFERENT root (a rewrite) is refused.
	if _, err := w.CounterSign(Checkpoint{TreeSize: 8, Root: "R2"}); err != errWitnessRewrite {
		t.Fatalf("rewrite must be refused, got %v", err)
	}
	// Regressing the tree size (a rollback) is refused.
	if _, err := w.CounterSign(Checkpoint{TreeSize: 4, Root: "Rx"}); err != errWitnessRegress {
		t.Fatalf("rollback must be refused, got %v", err)
	}
	// A genuine larger tree is accepted.
	if _, err := w.CounterSign(Checkpoint{TreeSize: 12, Root: "R3"}); err != nil {
		t.Fatalf("growth should be accepted: %v", err)
	}
}

// WS8: a writer that rewrites history cannot produce witness-valid checkpoints —
// re-checkpointing the tampered tree yields checkpoints the witness never signed,
// so VerifyWitness rejects them even though the writer re-signed with its own key.
func TestForgedChainFailsWitness(t *testing.T) {
	w := NewWitness()
	l := NewLog(Config{NodeID: "n", HMACKey: []byte("k"), CheckpointEvery: 4, Witness: w})
	for i := 0; i < 8; i++ {
		l.Append(sampleRecord(i))
	}
	l.Checkpoint()
	goodCPs := l.Checkpoints()
	witPub := l.WitnessPublicKey()

	// The attacker (writer) forges a NEW checkpoint over rewritten history using
	// its own signing key — but the witness, asked to attest a changed root at an
	// already-witnessed size, refuses, so the forged checkpoint has no WitnessSig.
	forged := goodCPs[len(goodCPs)-1]
	forged.Root = "TAMPERED_ROOT"
	if _, err := w.CounterSign(forged); err == nil {
		t.Fatal("witness must refuse to co-sign the rewritten root")
	}
	forged.WitnessSig = "" // attacker cannot obtain a valid one
	if at, ok := VerifyWitness([]Checkpoint{forged}, witPub); ok {
		t.Fatalf("forged (unwitnessed) checkpoint must fail verification, passed at %d", at)
	}
}

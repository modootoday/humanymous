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

// deep-review: a witness reconstructed from a persisted seed keeps the SAME verification
// key, so checkpoints co-signed before a restart still verify — no spurious witnessed:false
// alarm on every reboot; and after Restore it still refuses a rewrite behind the restored
// size, so anti-equivocation survives a (writer-triggerable) restart.
func TestWitnessSeedPersistenceAndRestore(t *testing.T) {
	seed := make([]byte, 32)
	for i := range seed {
		seed[i] = byte(i + 1)
	}
	w1 := NewWitnessFromSeed(seed)
	sig, err := w1.CounterSign(Checkpoint{TreeSize: 8, Root: "R1", MerkleRoot: ""}, nil)
	if err != nil {
		t.Fatalf("co-sign: %v", err)
	}
	// A DIFFERENT process boot reconstructs the witness from the same persisted seed.
	w2 := NewWitnessFromSeed(seed)
	if string(w2.Public()) != string(w1.Public()) {
		t.Fatal("reconstructed witness must have the SAME public key (else every restart false-alarms)")
	}
	if at, ok := VerifyWitness([]Checkpoint{{TreeSize: 8, Root: "R1", WitnessSig: sig}}, w2.Public()); !ok {
		t.Fatalf("a pre-restart checkpoint must verify under the reconstructed witness key (failed at %d)", at)
	}
	// Restore the monotonic state from the last replayed checkpoint, then a rewrite at that
	// size (or a rollback below it) must still be refused — no cross-restart amnesia.
	w2.Restore(8, "", "R1")
	if _, err := w2.CounterSign(Checkpoint{TreeSize: 8, Root: "R2"}, nil); err != errWitnessRewrite {
		t.Fatalf("after Restore a rewrite must be refused, got %v", err)
	}
	if _, err := w2.CounterSign(Checkpoint{TreeSize: 4, Root: "Rx"}, nil); err != errWitnessRegress {
		t.Fatalf("after Restore a rollback must be refused, got %v", err)
	}
}

// WS8: the witness refuses to co-sign a rewrite (root changed at a witnessed
// tree size) or a rollback (tree size regressed) — the exact moves a history
// forgery requires.
func TestWitnessRejectsRewriteAndRollback(t *testing.T) {
	w := NewWitness()
	if _, err := w.CounterSign(Checkpoint{TreeSize: 8, Root: "R1"}, nil); err != nil {
		t.Fatalf("first sign should succeed: %v", err)
	}
	// Re-signing the same size with a DIFFERENT root (a rewrite) is refused.
	if _, err := w.CounterSign(Checkpoint{TreeSize: 8, Root: "R2"}, nil); err != errWitnessRewrite {
		t.Fatalf("rewrite must be refused, got %v", err)
	}
	// Regressing the tree size (a rollback) is refused.
	if _, err := w.CounterSign(Checkpoint{TreeSize: 4, Root: "Rx"}, nil); err != errWitnessRegress {
		t.Fatalf("rollback must be refused, got %v", err)
	}
	// A genuine larger tree is accepted.
	if _, err := w.CounterSign(Checkpoint{TreeSize: 12, Root: "R3"}, nil); err != nil {
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
	if _, err := w.CounterSign(forged, nil); err == nil {
		t.Fatal("witness must refuse to co-sign the rewritten root")
	}
	forged.WitnessSig = "" // attacker cannot obtain a valid one
	if at, ok := VerifyWitness([]Checkpoint{forged}, witPub); ok {
		t.Fatalf("forged (unwitnessed) checkpoint must fail verification, passed at %d", at)
	}
}

// PLAN-08 R6: the witness co-signs only an append-only extension of the last tree it
// saw, verified by an RFC 6962 consistency proof. A fork that diverges from the
// witnessed history — even at a NEW, larger tree size the monotonicity check alone
// would accept — cannot produce a valid consistency proof and is refused.
func TestWitnessRejectsFork(t *testing.T) {
	honest := leavesN(8)
	root4 := hexStr(merkleRoot(honest[:4]))

	// Honest growth 4 -> 8 with a valid consistency proof is accepted.
	w1 := NewWitness()
	if _, err := w1.CounterSign(Checkpoint{TreeSize: 4, Root: "r4", MerkleRoot: root4}, nil); err != nil {
		t.Fatalf("first co-sign: %v", err)
	}
	if _, err := w1.CounterSign(Checkpoint{TreeSize: 8, Root: "r8", MerkleRoot: hexStr(merkleRoot(honest))},
		consistencyProof(honest, 4)); err != nil {
		t.Fatalf("honest append-only extension must be co-signed: %v", err)
	}

	// A fork: same first-4 root witnessed, but an 8-leaf tree whose history diverges
	// (leaf 0 changed). Its consistency proof is for the FORKED old root, not the
	// witnessed one, so verification fails and the witness refuses.
	forked := leavesN(8)
	forked[0] = []byte("rewritten-leaf-0")
	w2 := NewWitness()
	if _, err := w2.CounterSign(Checkpoint{TreeSize: 4, Root: "r4", MerkleRoot: root4}, nil); err != nil {
		t.Fatalf("first co-sign: %v", err)
	}
	if _, err := w2.CounterSign(Checkpoint{TreeSize: 8, Root: "r8f", MerkleRoot: hexStr(merkleRoot(forked))},
		consistencyProof(forked, 4)); err != errWitnessFork {
		t.Fatalf("a forked history must be refused with errWitnessFork, got %v", err)
	}
}

func hexStr(b []byte) string {
	const hexdigits = "0123456789abcdef"
	out := make([]byte, len(b)*2)
	for i, c := range b {
		out[i*2] = hexdigits[c>>4]
		out[i*2+1] = hexdigits[c&0x0f]
	}
	return string(out)
}

package audit

import (
	"crypto/ed25519"
	"encoding/hex"
	"testing"
)

// proofs_test.go ties the RFC 6962 proofs to the SIGNED tree head (PLAN-08 R6): an
// inclusion proof verifies against a checkpoint's Merkle root, a consistency proof
// verifies append-only growth between two checkpoints, and tampering with the signed
// Merkle root breaks the STH signature.

func appendN(l *Log, n int) {
	for i := 0; i < n; i++ {
		l.Append(Record{EventType: EventEnfDeny})
	}
}

func TestInclusionProofAgainstSignedSTH(t *testing.T) {
	l := NewLog(Config{NodeID: "n1", HMACKey: []byte("k"), CheckpointEvery: 4})
	appendN(l, 4) // triggers checkpoint cp1 at tree size 4

	cps := l.Checkpoints()
	if len(cps) == 0 {
		t.Fatal("no checkpoint produced")
	}
	cp1 := cps[len(cps)-1]
	root1, _ := hex.DecodeString(cp1.MerkleRoot)
	if len(root1) != 32 {
		t.Fatalf("checkpoint carries no Merkle root: %q", cp1.MerkleRoot)
	}

	res, ok := l.InclusionProof(2) // record seq 2
	if !ok || res.TreeSize != int(cp1.TreeSize) {
		t.Fatalf("inclusion proof unavailable or wrong size: ok=%v size=%d", ok, res.TreeSize)
	}
	if !VerifyInclusion(res.LeafData, res.LeafIndex, res.TreeSize, res.Proof, root1) {
		t.Fatal("record 2 must verify against the signed Merkle root")
	}
	// A forged leaf must not verify against the honest proof + signed root.
	if VerifyInclusion([]byte("forged-record-hash"), res.LeafIndex, res.TreeSize, res.Proof, root1) {
		t.Fatal("a forged leaf verified against the signed root")
	}
}

func TestConsistencyProofAcrossCheckpoints(t *testing.T) {
	l := NewLog(Config{NodeID: "n1", HMACKey: []byte("k"), CheckpointEvery: 4})
	appendN(l, 4) // cp1 @ size 4
	cp1 := lastCP(t, l)
	appendN(l, 4) // cp2 @ size 8
	cp2 := lastCP(t, l)

	proof, ok := l.ConsistencyProof(cp1.TreeSize)
	if !ok {
		t.Fatal("consistency proof unavailable")
	}
	r1, _ := hex.DecodeString(cp1.MerkleRoot)
	r2, _ := hex.DecodeString(cp2.MerkleRoot)
	if !VerifyConsistency(int(cp1.TreeSize), int(cp2.TreeSize), proof, r1, r2) {
		t.Fatal("append-only growth must verify between the two signed tree heads")
	}
	// A wrong old root (equivocation) must be rejected.
	if VerifyConsistency(int(cp1.TreeSize), int(cp2.TreeSize), proof, r2, r2) {
		t.Fatal("consistency accepted a mismatched old root")
	}
}

func TestTamperedMerkleRootBreaksSTH(t *testing.T) {
	l := NewLog(Config{NodeID: "n1", HMACKey: []byte("k"), CheckpointEvery: 4})
	appendN(l, 4)
	cp := lastCP(t, l)
	pub := l.PublicKey()

	// Honest STH verifies.
	if !ed25519.Verify(pub, sthBytes(cp), mustHex(cp.Sig)) {
		t.Fatal("honest STH must verify")
	}
	// Flip the Merkle root: the signature (which covers it) must now fail.
	cp.MerkleRoot = cp.MerkleRoot[:60] + "dead"
	if ed25519.Verify(pub, sthBytes(cp), mustHex(cp.Sig)) {
		t.Fatal("a tampered Merkle root must break the STH signature")
	}
}

func lastCP(t *testing.T, l *Log) Checkpoint {
	t.Helper()
	cps := l.Checkpoints()
	if len(cps) == 0 {
		t.Fatal("no checkpoint")
	}
	return cps[len(cps)-1]
}

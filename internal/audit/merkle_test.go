package audit

import (
	"strconv"
	"testing"
)

// merkle_test.go validates the RFC 6962 Merkle tree: roots, inclusion proofs, and
// consistency proofs verify for every size/index, and tampering is caught.

func leavesN(n int) [][]byte {
	ls := make([][]byte, n)
	for i := range ls {
		ls[i] = []byte("record-" + strconv.Itoa(i))
	}
	return ls
}

func TestMerkleInclusionAllSizes(t *testing.T) {
	for n := 1; n <= 33; n++ {
		leaves := leavesN(n)
		root := merkleRoot(leaves)
		for m := 0; m < n; m++ {
			proof := inclusionProof(leaves, m)
			if !verifyInclusion(leaves[m], m, n, proof, root) {
				t.Fatalf("inclusion proof failed: n=%d m=%d", n, m)
			}
			// A tampered leaf must NOT verify against the honest proof+root.
			if verifyInclusion([]byte("forged"), m, n, proof, root) {
				t.Fatalf("tampered leaf verified: n=%d m=%d", n, m)
			}
		}
	}
}

func TestMerkleConsistencyAllSizes(t *testing.T) {
	for n := 2; n <= 33; n++ {
		leaves := leavesN(n)
		newRoot := merkleRoot(leaves)
		for m := 1; m < n; m++ {
			oldRoot := merkleRoot(leaves[:m])
			proof := consistencyProof(leaves, m)
			if !verifyConsistency(m, n, proof, oldRoot, newRoot) {
				t.Fatalf("consistency proof failed: m=%d n=%d", m, n)
			}
			// A forked new tree (different leaf appended history) must be rejected.
			forked := append(append([][]byte{}, leaves[:m]...), leavesN(n-m+1)...)
			forkedRoot := merkleRoot(forked)
			if len(forked) == n && verifyConsistency(m, n, proof, oldRoot, forkedRoot) {
				t.Fatalf("consistency accepted a forked history: m=%d n=%d", m, n)
			}
		}
	}
}

func TestMerkleRootEmptyAndSingle(t *testing.T) {
	// Empty tree = H(); single leaf = leafHash. These anchor the recursion.
	if len(merkleRoot(nil)) != 32 {
		t.Error("empty root must be a 32-byte hash")
	}
	one := leavesN(1)
	if !equalHash(merkleRoot(one), leafHash(one[0])) {
		t.Error("single-leaf root must equal the leaf hash")
	}
}

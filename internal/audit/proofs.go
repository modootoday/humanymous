package audit

// proofs.go exposes RFC 6962 inclusion + consistency proofs over the audit log
// (PLAN-08 R6). The Merkle root they prove against is folded into every Signed Tree
// Head (see chain.go), so an offline auditor verifies a proof against a root that
// carries the writer's Ed25519 signature AND the independent witness co-signature —
// giving efficient, tamper-evident proof that a specific record is in the log and
// that the log only ever grew (append-only, no equivocation) between two STHs.

// InclusionResult is an offline-verifiable inclusion proof for one record.
type InclusionResult struct {
	LeafData  []byte   // the record-hash bytes committed as the Merkle leaf
	LeafIndex int      // 0-based position in the tree
	TreeSize  int      // tree size the proof is anchored to
	Proof     [][]byte // RFC 6962 audit path, leaf-to-root
}

// MerkleRoot returns the current RFC 6962 Merkle root over all record hashes.
func (l *Log) MerkleRoot() []byte {
	l.mu.Lock()
	defer l.mu.Unlock()
	return merkleRoot(l.leaves)
}

// InclusionProof returns the inclusion proof for the record at seq (1-based).
func (l *Log) InclusionProof(seq uint64) (InclusionResult, bool) {
	l.mu.Lock()
	defer l.mu.Unlock()
	if seq == 0 || seq > uint64(len(l.leaves)) {
		return InclusionResult{}, false
	}
	idx := int(seq - 1)
	return InclusionResult{
		LeafData:  append([]byte(nil), l.leaves[idx]...),
		LeafIndex: idx,
		TreeSize:  len(l.leaves),
		Proof:     inclusionProof(l.leaves, idx),
	}, true
}

// InclusionProofAt returns the inclusion proof for seq anchored to a specific
// (checkpointed) tree size, so it verifies against THAT STH's signed Merkle root
// rather than the unsigned current tip.
func (l *Log) InclusionProofAt(seq, size uint64) (InclusionResult, bool) {
	l.mu.Lock()
	defer l.mu.Unlock()
	if seq == 0 || seq > size || size > uint64(len(l.leaves)) {
		return InclusionResult{}, false
	}
	sub := l.leaves[:size]
	idx := int(seq - 1)
	return InclusionResult{
		LeafData:  append([]byte(nil), sub[idx]...),
		LeafIndex: idx,
		TreeSize:  int(size),
		Proof:     inclusionProof(sub, idx),
	}, true
}

// ConsistencyProof returns the append-only consistency proof from oldSize to the
// current tree size (0 < oldSize < size).
func (l *Log) ConsistencyProof(oldSize uint64) ([][]byte, bool) {
	l.mu.Lock()
	defer l.mu.Unlock()
	if oldSize == 0 || oldSize >= uint64(len(l.leaves)) {
		return nil, false
	}
	return consistencyProof(l.leaves, int(oldSize)), true
}

// VerifyInclusion verifies that leaf at index in a tree of size reproduces root
// (offline; RFC 6962 §2.1.1).
func VerifyInclusion(leaf []byte, index, size int, proof [][]byte, root []byte) bool {
	return verifyInclusion(leaf, index, size, proof, root)
}

// VerifyConsistency verifies that oldRoot (oldSize leaves) is an append-only prefix
// of newRoot (newSize leaves) (offline; RFC 6962 §2.1.2).
func VerifyConsistency(oldSize, newSize int, proof [][]byte, oldRoot, newRoot []byte) bool {
	return verifyConsistency(oldSize, newSize, proof, oldRoot, newRoot)
}

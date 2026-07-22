package audit

import "crypto/sha256"

// merkle.go implements the RFC 6962 (Certificate Transparency) Merkle tree over the
// audit log's record hashes (PLAN-08 R6). The linear hash chain already makes any
// tamper break the chain, but proving a SPECIFIC record's inclusion, or that the log
// grew append-only between two Signed Tree Heads, meant replaying the whole log. The
// Merkle tree adds O(log n) inclusion and consistency proofs that an offline auditor
// can verify against a signed tree head — the substrate for witness/split-view
// (equivocation) protection. The Merkle root is folded into the STH (see sthBytes),
// so it is covered by the same Ed25519 signature + independent witness co-signature.
//
// Domain separation follows RFC 6962 §2.1: leaf hash = H(0x00 || leaf), interior
// hash = H(0x01 || left || right), empty tree = H(). Leaves here are the record
// hashes (as bytes), so the Merkle tree commits to exactly the chained records.

func leafHash(d []byte) []byte {
	h := sha256.New()
	h.Write([]byte{0x00})
	h.Write(d)
	return h.Sum(nil)
}

func nodeHash(l, r []byte) []byte {
	h := sha256.New()
	h.Write([]byte{0x01})
	h.Write(l)
	h.Write(r)
	return h.Sum(nil)
}

// largestPow2Below returns the largest power of two strictly less than n (n >= 2).
func largestPow2Below(n int) int {
	k := 1
	for k<<1 < n {
		k <<= 1
	}
	return k
}

// merkleRoot computes the RFC 6962 Merkle Tree Hash over leaves.
func merkleRoot(leaves [][]byte) []byte {
	n := len(leaves)
	if n == 0 {
		s := sha256.Sum256(nil)
		return s[:]
	}
	if n == 1 {
		return leafHash(leaves[0])
	}
	k := largestPow2Below(n)
	return nodeHash(merkleRoot(leaves[:k]), merkleRoot(leaves[k:]))
}

// inclusionProof returns the audit path proving leaf index m is in a tree of the
// given leaves (RFC 6962 §2.1.1). Order is leaf-to-root.
func inclusionProof(leaves [][]byte, m int) [][]byte {
	n := len(leaves)
	if m < 0 || m >= n {
		return nil
	}
	if n == 1 {
		return nil
	}
	k := largestPow2Below(n)
	if m < k {
		return append(inclusionProof(leaves[:k], m), merkleRoot(leaves[k:]))
	}
	return append(inclusionProof(leaves[k:], m-k), merkleRoot(leaves[:k]))
}

// consistencyProof returns the proof that a tree of the first m leaves is a prefix
// of the tree of all leaves (RFC 6962 §2.1.2), for 0 < m < len(leaves).
func consistencyProof(leaves [][]byte, m int) [][]byte {
	n := len(leaves)
	if m <= 0 || m >= n {
		return nil
	}
	return subProof(m, leaves, true)
}

func subProof(m int, leaves [][]byte, b bool) [][]byte {
	n := len(leaves)
	if m == n {
		if b {
			return nil
		}
		return [][]byte{merkleRoot(leaves)}
	}
	k := largestPow2Below(n)
	if m <= k {
		return append(subProof(m, leaves[:k], b), merkleRoot(leaves[k:]))
	}
	return append(subProof(m-k, leaves[k:], false), merkleRoot(leaves[:k]))
}

// verifyInclusion checks an inclusion proof: that leaf (raw data) at index m in a
// tree of `size` leaves reproduces root (RFC 6962 §2.1.1 verification).
func verifyInclusion(leaf []byte, m, size int, proof [][]byte, root []byte) bool {
	if m < 0 || m >= size {
		return false
	}
	node := leafHash(leaf)
	fn, sn := m, size-1
	for _, p := range proof {
		if fn == sn || fn%2 == 1 {
			node = nodeHash(p, node)
			for fn%2 == 0 && fn != 0 {
				fn >>= 1
				sn >>= 1
			}
		} else {
			node = nodeHash(node, p)
		}
		fn >>= 1
		sn >>= 1
	}
	return sn == 0 && equalHash(node, root)
}

// verifyConsistency checks that oldRoot (m leaves) is consistent with newRoot (n
// leaves) given proof (RFC 6962 §2.1.2 verification).
func verifyConsistency(m, n int, proof [][]byte, oldRoot, newRoot []byte) bool {
	if m <= 0 || m > n {
		return false
	}
	if m == n {
		return len(proof) == 0 && equalHash(oldRoot, newRoot)
	}
	pr := proof
	if isPow2(m) { // the old tree is a complete subtree: its root is implied, prepend it
		pr = append([][]byte{oldRoot}, pr...)
	}
	if len(pr) == 0 {
		return false
	}
	fn, sn := m-1, n-1
	for fn%2 == 1 {
		fn >>= 1
		sn >>= 1
	}
	fr, sr := pr[0], pr[0]
	for _, c := range pr[1:] {
		if sn == 0 {
			return false
		}
		if fn%2 == 1 || fn == sn {
			fr = nodeHash(c, fr)
			sr = nodeHash(c, sr)
			for fn%2 == 0 && fn != 0 {
				fn >>= 1
				sn >>= 1
			}
		} else {
			sr = nodeHash(sr, c)
		}
		fn >>= 1
		sn >>= 1
	}
	return sn == 0 && equalHash(fr, oldRoot) && equalHash(sr, newRoot)
}

func isPow2(n int) bool { return n > 0 && n&(n-1) == 0 }

func equalHash(a, b []byte) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

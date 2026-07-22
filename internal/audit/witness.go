package audit

import (
	"crypto/ed25519"
	"encoding/hex"
	"errors"
	"sync"
)

// witness.go implements a local witness co-signer (SoT-28 WS8). The audit writer
// holds the STH signing key, so between external anchors it could rewrite history
// and re-sign — the writer verifying its own chain (SelfVerify) provides no
// independent assurance. A witness is a SEPARATE key-holder that observes each
// Signed Tree Head and counter-signs it ONLY if the tree grows monotonically and
// no previously-witnessed root is silently changed. A rewrite forces the writer
// to present a regressed/altered STH, which the witness refuses to counter-sign,
// so a chain missing a valid witness signature is detectably forged.
//
// This reference runs the witness in-process with its own key; production runs it
// as a separate process or a peer node (the interface is identical).

// Witness counter-signs monotonic tree heads with an independent key.
type Witness struct {
	mu         sync.Mutex
	priv       ed25519.PrivateKey
	seenRoot   map[uint64]string // TreeSize -> the root this witness attested
	maxSize    uint64
	lastSize   uint64 // size of the last STH this witness co-signed (PLAN-08 R6)
	lastMerkle string // its Merkle root, for the append-only consistency check
}

// NewWitness creates a witness with a fresh keypair.
func NewWitness() *Witness {
	_, priv, _ := ed25519.GenerateKey(nil)
	return &Witness{priv: priv, seenRoot: map[uint64]string{}}
}

// Public returns the witness verification key (auditors hold this).
func (wt *Witness) Public() ed25519.PublicKey { return wt.priv.Public().(ed25519.PublicKey) }

// LastSize returns the tree size of the last STH this witness co-signed (0 = none).
// The writer uses it to generate the consistency proof the witness will demand.
func (wt *Witness) LastSize() uint64 {
	wt.mu.Lock()
	defer wt.mu.Unlock()
	return wt.lastSize
}

// CounterSign attests a Signed Tree Head. Beyond rejecting a regressed tree or a
// changed root at an already-witnessed size, it now cryptographically verifies —
// via an RFC 6962 consistency proof — that the new tree is an APPEND-ONLY extension
// of the last one it co-signed (PLAN-08 R6). This closes the split-view /
// equivocation gap: a writer that rewrites history before the last witnessed size
// cannot produce a valid consistency proof, so the witness refuses to co-sign and
// the fork is detectable. On success it returns a hex counter-signature.
func (wt *Witness) CounterSign(cp Checkpoint, consistencyProof [][]byte) (string, error) {
	wt.mu.Lock()
	defer wt.mu.Unlock()
	if prev, ok := wt.seenRoot[cp.TreeSize]; ok {
		if prev != cp.Root {
			return "", errWitnessRewrite // root at a witnessed size changed
		}
	} else if cp.TreeSize <= wt.maxSize {
		return "", errWitnessRegress // tree went backwards / sideways
	}
	// Append-only proof: the new Merkle root must consistency-extend the last one.
	// (Skipped only when a checkpoint carries no Merkle root — a real chain always
	// does; then the monotonicity checks above still apply.)
	if wt.lastSize > 0 && wt.lastSize < cp.TreeSize && wt.lastMerkle != "" && cp.MerkleRoot != "" {
		oldRoot, err1 := hex.DecodeString(wt.lastMerkle)
		newRoot, err2 := hex.DecodeString(cp.MerkleRoot)
		if err1 != nil || err2 != nil ||
			!verifyConsistency(int(wt.lastSize), int(cp.TreeSize), consistencyProof, oldRoot, newRoot) {
			return "", errWitnessFork // not an append-only extension of the witnessed tree
		}
	}
	wt.seenRoot[cp.TreeSize] = cp.Root
	if cp.TreeSize > wt.maxSize {
		wt.maxSize = cp.TreeSize
	}
	wt.lastSize, wt.lastMerkle = cp.TreeSize, cp.MerkleRoot
	return hex.EncodeToString(ed25519.Sign(wt.priv, sthBytes(cp))), nil
}

var (
	errWitnessRewrite = errors.New("witness: root changed at an already-witnessed tree size (rewrite)")
	errWitnessRegress = errors.New("witness: tree size regressed (rollback)")
	errWitnessFork    = errors.New("witness: new tree is not an append-only extension (fork/rewrite)")
)

// VerifyWitness checks that every checkpoint carries a valid witness
// counter-signature under witnessPub (SoT-28 WS8). A checkpoint with no or an
// invalid witness signature means the chain was never independently attested —
// treat it as a forgery. Returns the first offending TreeSize (0 => all good).
func VerifyWitness(checkpoints []Checkpoint, witnessPub ed25519.PublicKey) (uint64, bool) {
	for _, cp := range checkpoints {
		if cp.WitnessSig == "" {
			return cp.TreeSize, false
		}
		sig, err := hex.DecodeString(cp.WitnessSig)
		if err != nil || !ed25519.Verify(witnessPub, sthBytes(cp), sig) {
			return cp.TreeSize, false
		}
	}
	return 0, true
}

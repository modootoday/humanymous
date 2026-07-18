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
	mu       sync.Mutex
	priv     ed25519.PrivateKey
	seenRoot map[uint64]string // TreeSize -> the root this witness attested
	maxSize  uint64
}

// NewWitness creates a witness with a fresh keypair.
func NewWitness() *Witness {
	_, priv, _ := ed25519.GenerateKey(nil)
	return &Witness{priv: priv, seenRoot: map[uint64]string{}}
}

// Public returns the witness verification key (auditors hold this).
func (wt *Witness) Public() ed25519.PublicKey { return wt.priv.Public().(ed25519.PublicKey) }

// CounterSign attests a Signed Tree Head. It rejects any attempt to regress the
// tree (a smaller/equal TreeSize than already seen) or to change the root at a
// TreeSize it has already witnessed — the exact moves a history rewrite requires.
// On success it returns a hex counter-signature over the STH bytes.
func (wt *Witness) CounterSign(cp Checkpoint) (string, error) {
	wt.mu.Lock()
	defer wt.mu.Unlock()
	if prev, ok := wt.seenRoot[cp.TreeSize]; ok {
		if prev != cp.Root {
			return "", errWitnessRewrite // root at a witnessed size changed
		}
	} else if cp.TreeSize <= wt.maxSize {
		return "", errWitnessRegress // tree went backwards / sideways
	}
	wt.seenRoot[cp.TreeSize] = cp.Root
	if cp.TreeSize > wt.maxSize {
		wt.maxSize = cp.TreeSize
	}
	return hex.EncodeToString(ed25519.Sign(wt.priv, sthBytes(cp))), nil
}

var (
	errWitnessRewrite = errors.New("witness: root changed at an already-witnessed tree size (rewrite)")
	errWitnessRegress = errors.New("witness: tree size regressed (rollback)")
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

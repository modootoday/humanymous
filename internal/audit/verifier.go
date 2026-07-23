package audit

import (
	"crypto/ed25519"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
)

// verifier.go is the standalone offline verifier (SoT-18 §4). It recomputes the
// canonical hash, the chain linkage, the per-record HMAC, and the Ed25519
// checkpoint signatures, reporting PASS or the first divergent seq with a
// mismatch class. It needs only the public key — no forging power — so an
// auditor can run it independently of the writer.

// MismatchClass names the kind of integrity break found.
type MismatchClass string

const (
	ClassOK            MismatchClass = "ok"
	ClassHashBreak     MismatchClass = "hash-break"
	ClassHMACInvalid   MismatchClass = "hmac-invalid"
	ClassSeqGap        MismatchClass = "seq-gap"
	ClassLinkageBreak  MismatchClass = "linkage-break"
	ClassCheckpointBad MismatchClass = "checkpoint-mismatch"
	ClassNodeMissing   MismatchClass = "node-missing"
)

// VerifyResult is the verifier's report.
type VerifyResult struct {
	OK     bool
	Class  MismatchClass
	AtSeq  uint64
	Detail string
}

// pass is the success result.
func pass() VerifyResult { return VerifyResult{OK: true, Class: ClassOK} }

// SelfVerify replays this log's own chain (it holds the HMAC key and pubkey).
// Used by the admin/integrity endpoint; production runs an independent verifier
// against a read replica with only the public key (SoT-18 §1 SoD).
func (l *Log) SelfVerify() VerifyResult {
	return Verify(l.Records(), l.Checkpoints(), l.hmacKey, l.PublicKey())
}

// Verify replays the chain against the HMAC key and STH public key. The HMAC key
// is needed only because HMAC is a secondary symmetric layer; the PRIMARY trust
// (checkpoint signatures) verifies with pub alone.
func Verify(records []Record, checkpoints []Checkpoint, hmacKey []byte, pub ed25519.PublicKey) VerifyResult {
	prevHash := ""
	var expectSeq uint64 = 1
	for i := range records {
		r := records[i]
		if r.Seq != expectSeq {
			return VerifyResult{Class: ClassSeqGap, AtSeq: r.Seq,
				Detail: "expected seq " + itoa(expectSeq)}
		}
		expectSeq++

		// Linkage: the record's prev_hash must equal the running chain head.
		if r.PrevHash != prevHash {
			return VerifyResult{Class: ClassLinkageBreak, AtSeq: r.Seq,
				Detail: "prev_hash does not match chain head"}
		}
		// Recompute record hash over the frozen canonical form.
		want := computeRecordHash(&r, prevHash)
		if !hmac.Equal([]byte(want), []byte(r.RecordHash)) {
			return VerifyResult{Class: ClassHashBreak, AtSeq: r.Seq,
				Detail: "record content was altered"}
		}
		// Secondary HMAC layer.
		mac := hmac.New(sha256.New, hmacKey)
		mac.Write([]byte(r.RecordHash))
		if !hmac.Equal([]byte(hex.EncodeToString(mac.Sum(nil))), []byte(r.HMAC)) {
			return VerifyResult{Class: ClassHMACInvalid, AtSeq: r.Seq,
				Detail: "hmac invalid"}
		}
		prevHash = r.RecordHash
	}

	// Verify each checkpoint signature and that its root matches the chain state
	// at TreeSize (detects truncation/rollback up to the anchor cadence).
	rootAt := map[uint64]string{}
	pv := ""
	for i := range records {
		rootAt[records[i].Seq] = records[i].RecordHash
	}
	rootAt[0] = ""
	for _, cp := range checkpoints {
		if !ed25519.Verify(pub, sthBytes(cp), mustHex(cp.Sig)) {
			return VerifyResult{Class: ClassCheckpointBad, AtSeq: cp.TreeSize,
				Detail: "checkpoint signature invalid"}
		}
		if cp.PrevCP != pv {
			return VerifyResult{Class: ClassCheckpointBad, AtSeq: cp.TreeSize,
				Detail: "checkpoint chain broken"}
		}
		if root, ok := rootAt[cp.TreeSize]; !ok || root != cp.Root {
			return VerifyResult{Class: ClassCheckpointBad, AtSeq: cp.TreeSize,
				Detail: "checkpoint root does not match records (truncation/rollback)"}
		}
		// Reconcile the SIGNED Merkle root against the actual record set (deep-review). The
		// chained Root check above ties the STH to the last record hash, but the STH ALSO
		// commits an RFC-6962 MerkleRoot that inclusion proofs verify against — and it was
		// trusted on the writer's signature alone. A compromised writer could sign a
		// checkpoint whose chained Root matches the records yet whose MerkleRoot commits to a
		// DIFFERENT leaf set, so an offline inclusion proof would validate a record not in the
		// chain. Recompute the tree over the first TreeSize record hashes and require a match.
		if cp.MerkleRoot != "" {
			if uint64(len(records)) < cp.TreeSize {
				return VerifyResult{Class: ClassCheckpointBad, AtSeq: cp.TreeSize,
					Detail: "checkpoint merkle root: fewer records than tree size"}
			}
			leaves := make([][]byte, cp.TreeSize)
			for i := uint64(0); i < cp.TreeSize; i++ {
				leaves[i] = []byte(records[i].RecordHash)
			}
			if hex.EncodeToString(merkleRoot(leaves)) != cp.MerkleRoot {
				return VerifyResult{Class: ClassCheckpointBad, AtSeq: cp.TreeSize,
					Detail: "checkpoint merkle root does not match records (STH committed a different leaf set)"}
			}
		}
		pv = cp.Sig
	}
	return pass()
}

func mustHex(s string) []byte { b, _ := hex.DecodeString(s); return b }

func itoa(v uint64) string {
	if v == 0 {
		return "0"
	}
	var b [20]byte
	i := len(b)
	for v > 0 {
		i--
		b[i] = byte('0' + v%10)
		v /= 10
	}
	return string(b[i:])
}

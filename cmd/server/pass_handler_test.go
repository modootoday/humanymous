package main

import (
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/pow"
)

// TestReserveTraceBlocksReplay is the anti-replay invariant (SoT-36 §5): a motor
// trace is admissible once; the identical capture replayed collides on its digest.
func TestReserveTraceBlocksReplay(t *testing.T) {
	ps := newPassStore()
	now := time.Now()
	d := traceDigest(passProof{RawT: []float64{1, 7, 15, 24}, Durations: []float64{6, 8, 9}})
	if !ps.reserveTrace(d, now) {
		t.Fatal("first use of a fresh trace must be admitted")
	}
	if ps.reserveTrace(d, now) {
		t.Fatal("replay of the same trace must be rejected")
	}
}

// TestTraceDigestIgnoresOffsetsAndNonce: a replay wrapped around a fresh challenge
// (new offsets/nonce, same captured motor data) must still collide — the digest is
// over the raw motor evidence only.
func TestTraceDigestIgnoresOffsetsAndNonce(t *testing.T) {
	base := passProof{RawT: []float64{2, 9, 18}, KeyDurs: []float64{131, 88, 210}}
	wrapped := base
	wrapped.ChallengeNonce = "a-different-nonce"
	wrapped.Offsets = []int{3, 1, 7}
	if traceDigest(base) != traceDigest(wrapped) {
		t.Fatal("digest must ignore per-instance nonce/offsets so replays collide")
	}
}

// TestTraceDigestDistinguishesDifferentMotion: two genuinely different captures do
// not collide (no false-positive replay block for real humans).
func TestTraceDigestDistinguishesDifferentMotion(t *testing.T) {
	a := passProof{KeyDurs: []float64{137, 89, 211, 123}}
	b := passProof{KeyDurs: []float64{140, 92, 205, 119}}
	if traceDigest(a) == traceDigest(b) {
		t.Fatal("distinct motor traces must produce distinct digests")
	}
}

// TestPoWInstanceBinding: a PoW solved for one challenge instance (nonce) must NOT
// verify against another instance, even in the same session + time bucket. This is
// what stops a bot from solving one PoW and reusing it across /new instances.
func TestPoWInstanceBinding(t *testing.T) {
	key := []byte("test-master-key")
	sid := "sess-xyz"
	bucket := uint64(time.Now().Unix() / pow.Window)
	diff := 12

	nonceA := "instance-aaaa"
	nonceB := "instance-bbbb"
	chA := pow.Issue(key, passPoWSession(sid, nonceA), diff, bucket)
	sol, ok := pow.Solve(chA.Seed, diff, 1<<24)
	if !ok {
		t.Fatal("could not solve PoW for instance A")
	}
	// Correct instance verifies.
	if !pow.Verify(key, passPoWSession(sid, nonceA), diff, bucket, bucket, sol) {
		t.Fatal("PoW solution must verify for its own instance")
	}
	// The SAME solution against instance B must fail (different bound seed).
	if pow.Verify(key, passPoWSession(sid, nonceB), diff, bucket, bucket, sol) {
		t.Fatal("PoW solution must NOT verify against a different instance nonce")
	}
}

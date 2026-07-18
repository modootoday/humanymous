package main

import (
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/abuse"
	"github.com/modootoday/humanymous/internal/pow"
	"github.com/modootoday/humanymous/internal/signals"
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

// TestPassVelocitySignalsRegistered: the axis-③ fusion only raises risk if its
// signal IDs carry weight in the registry (unknown IDs are weight 0). solved stays
// 0 (a trust upgrade via the OK verdict, not a weighted penalty).
func TestPassVelocitySignalsRegistered(t *testing.T) {
	cases := map[string]float64{
		"l7.pass.solved":   0,
		"l7.pass.velocity": 25,
		"l7.pass.flood":    45,
	}
	for id, want := range cases {
		d, ok := signals.Lookup(id)
		if !ok {
			t.Fatalf("%s not registered", id)
		}
		if d.Weight != want {
			t.Fatalf("%s weight = %v, want %v", id, d.Weight, want)
		}
	}
}

// TestPassVelocityThresholds documents the delay-free governor's levels: the 30s
// window with soft 4 / hard 8 leaves a human's 1-few attempts at level 0, while a
// sustained loop crosses into elevated (1) then flood (2). No lockout — the window
// self-clears, so the wargame stays iterable.
func TestPassVelocityThresholds(t *testing.T) {
	lim := abuse.NewLimiter(30*time.Second, 4, 8)
	now := time.Now()
	levels := make([]int, 0, 8)
	for i := 0; i < 8; i++ {
		levels = append(levels, lim.Level(lim.Observe("k", now)))
	}
	if levels[1] != 0 {
		t.Fatalf("a human's 2nd attempt must stay level 0, got %d", levels[1])
	}
	if levels[3] != 1 {
		t.Fatalf("4th observation must be elevated (level 1), got %d", levels[3])
	}
	if levels[7] != 2 {
		t.Fatalf("8th observation must be flood (level 2), got %d", levels[7])
	}
}

// TestAttestIssuanceBudget documents the axis-① identity cap: attestation issuance is
// rate-limited PER FINGERPRINT (30s window, budget 3), so a cookie-rotating flood from
// one fingerprint runs out of tokens. handlePassAttest denies once Level >= 2. Short,
// self-clearing window — never a multi-minute lockout.
func TestAttestIssuanceBudget(t *testing.T) {
	lim := abuse.NewLimiter(30*time.Second, 3, 3)
	now := time.Now()
	if lim.Level(lim.Observe("fp", now)) >= 2 {
		t.Fatal("1st issuance must be allowed")
	}
	if lim.Level(lim.Observe("fp", now)) >= 2 {
		t.Fatal("2nd issuance must be allowed")
	}
	if lim.Level(lim.Observe("fp", now)) < 2 {
		t.Fatal("3rd issuance must hit the per-fingerprint cap (deny)")
	}
}

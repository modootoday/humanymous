package main

import (
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/abuse"
	"github.com/modootoday/humanymous/internal/pow"
)

// Pass plane residual locks (handler/store) — complements pass_wargame.mjs Docker path.

func TestWargameR333_PassTraceReplayBlocked(t *testing.T) {
	ps := newPassStore()
	now := time.Now()
	d := traceDigest(passProof{RawT: []float64{3, 11, 22, 40}, Durations: []float64{8, 11, 18}})
	if !ps.reserveTrace(d, now) {
		t.Fatal("first admit")
	}
	if ps.reserveTrace(d, now) {
		t.Fatal("replay must block")
	}
}

func TestWargameR334_PassTraceDigestIgnoresChallengeWrapper(t *testing.T) {
	base := passProof{RawT: []float64{5, 12, 20}, KeyDurs: []float64{100, 120, 90}}
	wrap := base
	wrap.ChallengeNonce = "fresh-nonce"
	wrap.Offsets = []int{1, 2, 3}
	if traceDigest(base) != traceDigest(wrap) {
		t.Fatal("digest must ignore challenge wrapper so replays collide")
	}
}

func TestWargameR335_PassPoWBoundToChallengeNonce(t *testing.T) {
	key := []byte("pass-wargame-pow-key!!!!!!!!!")
	sid := "sid-r335"
	bucket := uint64(time.Now().Unix() / pow.Window)
	diff := 10
	nA, nB := "nonce-A-r335", "nonce-B-r335"
	chA := pow.Issue(key, passPoWSession(sid, nA), diff, bucket)
	sol, ok := pow.Solve(chA.Seed, diff, 1<<22)
	if !ok {
		t.Fatal("solve")
	}
	if !pow.Verify(key, passPoWSession(sid, nA), diff, bucket, bucket, sol) {
		t.Fatal("own instance")
	}
	if pow.Verify(key, passPoWSession(sid, nB), diff, bucket, bucket, sol) {
		t.Fatal("cross-instance PoW reuse must fail")
	}
}

func TestWargameR336_PassVelocityHumanFloor(t *testing.T) {
	// Soft governor: human few attempts stay level 0 (no multi-minute lockout).
	lim := abuse.NewLimiter(30*time.Second, 4, 8)
	now := time.Now()
	for i := 0; i < 3; i++ {
		n := lim.Observe("fp|human", now.Add(time.Duration(i)*time.Second))
		if lim.Level(n) != 0 {
			t.Fatalf("human pace must stay level 0, n=%d level=%d", n, lim.Level(n))
		}
	}
}

func TestWargameR337_PassVelocityFloodElevates(t *testing.T) {
	lim := abuse.NewLimiter(30*time.Second, 4, 8)
	now := time.Now()
	var n int
	for i := 0; i < 10; i++ {
		n = lim.Observe("fp|bot", now.Add(time.Duration(i)*10*time.Millisecond))
	}
	if lim.Level(n) < 2 {
		t.Fatalf("sustained flood must reach hard band, n=%d level=%d", n, lim.Level(n))
	}
}

func TestWargameR338_PassAxisHandlerClose(t *testing.T) {
	ps := newPassStore()
	d := traceDigest(passProof{KeyDurs: []float64{50, 60, 70}})
	if !ps.reserveTrace(d, time.Now()) || ps.reserveTrace(d, time.Now()) {
		t.Fatal("replay lock")
	}
}

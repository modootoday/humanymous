package gate

import (
	"testing"
	"time"
)

// gc_test.go verifies the PLAN-08 deployment-review ship-blocker fix: the gate's
// in-memory detection maps are actually swept, so they cannot grow without bound
// under fingerprint/IP churn.

var gcT0 = time.Unix(1_700_000_000, 0)

func TestVerdictStoreGC(t *testing.T) {
	vs := NewVerdictStore(time.Minute)
	vs.Set("sid-1", stickyVerdict{verdict: VerdictDeny, updated: gcT0})
	vs.GC(gcT0.Add(90 * time.Second)) // past the 1-minute TTL
	vs.mu.RLock()
	n := len(vs.m)
	vs.mu.RUnlock()
	if n != 0 {
		t.Fatalf("expired sticky verdict must be swept, %d remain", n)
	}
}

func TestBanStoreGCSweepsStrikesAndExpired(t *testing.T) {
	// hard=1 so a single Observe triggers a strike + auto-ban.
	bs := NewBanStore(10*time.Second, 1, 1)
	bs.nowFn = func() time.Time { return gcT0 }
	bs.Observe("ip:1.2.3.4") // -> strike + 1h ban
	bs.mu.Lock()
	strikes := len(bs.strikes)
	bs.mu.Unlock()
	if strikes == 0 {
		t.Fatal("expected a strike record after a hard breach")
	}
	// Sweep well past strikeDecay + the 1h ban.
	bs.GC(gcT0.Add(strikeDecay + 2*time.Hour))
	bs.mu.Lock()
	defer bs.mu.Unlock()
	if len(bs.strikes) != 0 {
		t.Errorf("stale strikes must be swept, %d remain", len(bs.strikes))
	}
	if len(bs.bans) != 0 {
		t.Errorf("expired bans must be swept, %d remain", len(bs.bans))
	}
}

func TestSweepDetectorGC(t *testing.T) {
	sd := NewSweepDetector(30*time.Second, 8)
	sd.Observe("fp-1", "sid-1", gcT0)
	sd.GC(gcT0.Add(90 * time.Second)) // > 2*window
	sd.mu.Lock()
	n := len(sd.byBind)
	sd.mu.Unlock()
	if n != 0 {
		t.Fatalf("aged-out binding window must be swept, %d remain", n)
	}
}

func TestServerGCRunsWithoutPanic(t *testing.T) {
	// Server.GC must sweep every in-memory map (and be a no-op for nil optionals).
	srv, _ := banStack(t, "http://origin.invalid", 10)
	srv.GC(time.Now()) // must not panic with the default in-memory ledgers
}

package correlation

import (
	"fmt"
	"sync"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/redis"
)

// fakeDoer scripts the Redis EVAL reply and records the commands issued, so the fleet path is
// unit-testable with no live Redis.
type fakeDoer struct {
	mu    sync.Mutex
	calls [][]string
	vels  []int // scripted fleet velocity per call; `added` is always 1 (globally-new /24)
	i     int
	err   error // when set, every call returns this (simulates a coordinator outage)
}

func (f *fakeDoer) Do(args ...string) (redis.Reply, error) {
	f.mu.Lock()
	defer f.mu.Unlock()
	f.calls = append(f.calls, append([]string(nil), args...))
	if f.err != nil {
		return redis.Reply{}, f.err
	}
	v := 1
	if f.i < len(f.vels) {
		v = f.vels[f.i]
	}
	f.i++
	return redis.Reply{Array: []redis.Reply{{Int: 1}, {Int: int64(v)}}}, nil
}

func TestShared_EvalCommandContract(t *testing.T) {
	f := &fakeDoer{}
	r := NewShared(time.Hour, f, nil)
	r.Observe("fp|ja4", "10.0.0", "s1", time.Unix(1_700_000_000, 0))

	if len(f.calls) != 1 {
		t.Fatalf("expected exactly one EVAL, got %d", len(f.calls))
	}
	c := f.calls[0]
	h := keyDigest(nil, "fp|ja4")
	if c[0] != "EVAL" || c[2] != "2" || c[3] != "hmn:corr:sub:"+h || c[4] != "hmn:corr:vel:"+h {
		t.Fatalf("bad EVAL keys: %v", c[:5])
	}
	if c[len(c)-1] != "10.0.0" {
		t.Fatalf("last ARGV must be the subnet, got %q", c[len(c)-1])
	}
}

func TestShared_FleetCountDrivesVelocity(t *testing.T) {
	// 40 organic observations report fleet velocity 1 (baseline), then one reports 50 (a campaign
	// aggregated across nodes) — which must fire ip_velocity off the FLEET count.
	vels := make([]int, 40)
	for i := range vels {
		vels[i] = 1
	}
	vels = append(vels, 50)
	f := &fakeDoer{vels: vels}
	r := NewShared(time.Hour, f, nil)
	t0 := time.Unix(1_700_000_000, 0)

	fired := false
	for i := 0; i <= 40; i++ {
		sigs := r.Observe(fmt.Sprintf("fp-%d|ja4", i), fmt.Sprintf("10.%d.0", i), fmt.Sprintf("s-%d", i), t0)
		if _, ok := findSig(sigs, "l5.correlation.ip_velocity"); ok {
			fired = true
		}
	}
	if !fired {
		t.Fatal("a high FLEET velocity vs the organic baseline must fire ip_velocity")
	}
}

func TestShared_RedisOutageFallsBackToLocal(t *testing.T) {
	// With the coordinator down, the fleet path must degrade to EXACTLY the per-process behavior:
	// a local burst still fires ip_velocity (proving fail-open, not fail-closed).
	f := &fakeDoer{err: redis.ErrBreakerOpen}
	r := NewShared(time.Hour, f, nil)
	t0 := time.Unix(1_700_000_000, 0)
	for i := 0; i < 40; i++ { // organic population, local counting
		r.Observe(fmt.Sprintf("org-%d|ja4", i), fmt.Sprintf("10.%d.0", i), fmt.Sprintf("s-%d", i), t0)
	}
	fired := false
	for i := 0; i < 30; i++ { // one fingerprint bursts /24s locally
		sigs := r.Observe("bot|ja4", fmt.Sprintf("172.16.%d", i), fmt.Sprintf("b-%d", i), t0.Add(time.Duration(i)*time.Second))
		if _, ok := findSig(sigs, "l5.correlation.ip_velocity"); ok {
			fired = true
		}
	}
	if !fired {
		t.Fatal("on a Redis outage the registry must fall back to per-process velocity (fail-open)")
	}
	// the coordinator was consulted (and errored) at least once
	if len(f.calls) == 0 {
		t.Fatal("the fleet path must have attempted the coordinator")
	}
}

func TestKeyDigest_StableAndKeyed(t *testing.T) {
	if d1, d2 := keyDigest(nil, "a"), keyDigest(nil, "a"); d1 != d2 {
		t.Fatal("digest must be deterministic")
	}
	if keyDigest(nil, "a") == keyDigest(nil, "b") {
		t.Fatal("distinct keys must not collide (no cross-fingerprint merge)")
	}
	if keyDigest([]byte("fleetkey"), "a") == keyDigest(nil, "a") {
		t.Fatal("HMAC digest must differ from the unkeyed SHA-256")
	}
	if len(keyDigest(nil, "a")) != 64 {
		t.Fatal("digest must be a fixed-width 64-hex (no raw key bytes at rest)")
	}
}

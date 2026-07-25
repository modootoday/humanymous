package abuse

import (
	"testing"
	"time"
)

func TestLimiter_SlidingWindow(t *testing.T) {
	l := NewLimiter(time.Second, 5, 10)
	base := time.Unix(1000, 0)
	// 12 requests within the window -> hard flood.
	var last int
	for i := 0; i < 12; i++ {
		last = l.Observe("fpA", base.Add(time.Duration(i)*10*time.Millisecond))
	}
	if l.Level(last) != 2 {
		t.Fatalf("12 reqs/window want hard(2) got level %d (count %d)", l.Level(last), last)
	}
}

func TestLimiter_WindowExpiry(t *testing.T) {
	l := NewLimiter(time.Second, 5, 10)
	base := time.Unix(1000, 0)
	// spread requests > window apart -> never accumulates.
	var last int
	for i := 0; i < 12; i++ {
		last = l.Observe("fpB", base.Add(time.Duration(i)*2*time.Second))
	}
	if l.Level(last) != 0 {
		t.Fatalf("spread requests should stay ok, got level %d (count %d)", l.Level(last), last)
	}
}

func TestLimiter_PerKeyIsolation(t *testing.T) {
	l := NewLimiter(time.Second, 5, 10)
	base := time.Unix(1000, 0)
	for i := 0; i < 11; i++ {
		l.Observe("flooder", base.Add(time.Duration(i)*10*time.Millisecond))
	}
	// a different key is unaffected.
	if c := l.Observe("innocent", base); l.Level(c) != 0 {
		t.Fatalf("innocent key should be ok, got level %d", l.Level(c))
	}
}

func TestLimiter_EmptyKeyIgnored(t *testing.T) {
	l := NewLimiter(time.Second, 5, 10)
	if c := l.Observe("", time.Unix(1, 0)); c != 0 {
		t.Fatal("empty key must not be counted")
	}
}

func TestLimiterConfigurePreservesRollingHits(t *testing.T) {
	now := time.Unix(1_700_000_000, 0)
	l := NewLimiter(10*time.Second, 3, 5)
	if got := l.Observe("fp:x", now); got != 1 {
		t.Fatalf("first count=%d", got)
	}
	if got := l.Observe("fp:x", now.Add(time.Second)); got != 2 {
		t.Fatalf("second count=%d", got)
	}

	l.Configure(20*time.Second, 2, 3)
	if got := l.Level(2); got != 1 {
		t.Fatalf("hot soft threshold not applied: level=%d", got)
	}
	count := l.Observe("fp:x", now.Add(2*time.Second))
	if count != 3 || l.Level(count) != 2 {
		t.Fatalf("rolling hits reset or hard threshold stale: count=%d level=%d", count, l.Level(count))
	}
}

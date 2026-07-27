package main

import (
	"net"
	"sync"
	"testing"
	"time"

	"golang.org/x/net/netutil"
)

// A slowloris / slow-POST flood can accumulate unbounded concurrent connections
// (open-rate × hold-time) even though each is dropped after ReadHeaderTimeout —
// the per-connection duration cap does not bound concurrency. The accept loop
// wraps the base listener in netutil.LimitListener so total concurrent accepted
// connections are bounded, converting unbounded accumulation into back-pressure.
//
// Regression guard for wargame round R2 (2026-07-27): without the LimitListener
// wrap, all dialers are accepted at once (no cap).
func TestConnConcurrencyIsCapped(t *testing.T) {
	base, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen: %v", err)
	}
	defer base.Close()

	const cap = 4
	lst := netutil.LimitListener(base, cap)
	addr := base.Addr().String()

	// Server: accept and HOLD connections (never close), mirroring slow connections
	// that a real server would keep until their header/read deadline.
	var mu sync.Mutex
	accepted := 0
	held := make([]net.Conn, 0, 16)
	go func() {
		for {
			c, err := lst.Accept()
			if err != nil {
				return
			}
			mu.Lock()
			accepted++
			held = append(held, c)
			mu.Unlock()
		}
	}()

	// Open more dialers than the cap and keep them open.
	const dialers = 12
	dials := make([]net.Conn, 0, dialers)
	for i := 0; i < dialers; i++ {
		c, err := net.DialTimeout("tcp", addr, time.Second)
		if err != nil {
			continue
		}
		dials = append(dials, c)
	}
	defer func() {
		for _, c := range dials {
			c.Close()
		}
	}()

	// Give the accept loop time to accept everything it is allowed to.
	time.Sleep(300 * time.Millisecond)

	mu.Lock()
	got := accepted
	mu.Unlock()
	if got > cap {
		t.Fatalf("LimitListener accepted %d concurrent connections, want <= cap %d", got, cap)
	}
	if got < cap {
		t.Fatalf("expected the cap (%d) to be saturated by %d dialers, only %d accepted", cap, dialers, got)
	}
}

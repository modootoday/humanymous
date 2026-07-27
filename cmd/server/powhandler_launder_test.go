package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/collector"
	"github.com/modootoday/humanymous/internal/pow"
	"github.com/modootoday/humanymous/internal/scoring"
)

// A native/GPU PoW solver reveals itself by producing a valid solution faster than
// any browser JS solver could (l7.pow.too_fast; SoT-13/15). That observation must be
// PERMANENT for the session: the solver must not be able to launder its own too-fast
// DENY into the pow.solved trust upgrade by resubmitting the same valid nonce after a
// browser-plausible delay. No human/browser can solve under the conservative floor
// even once, so persisting the tell is false-positive-safe.
//
// Regression guard for wargame round R1 (2026-07-27): before the fix, handlePoW
// recorded nothing on a too-fast hit, so the delayed resubmit took the success branch
// and granted pow.solved to a proven native solver.
func TestPoWTooFastCannotBeLaunderedByDelayedResubmit(t *testing.T) {
	a := &app{
		store:     collector.NewStore(time.Hour),
		engine:    scoring.NewEngine(),
		masterKey: []byte("test-master-key-0123456789abcdef"),
	}
	sid := "launder-sid"
	now := time.Now()
	a.store.Ensure(sid, now)

	// Difficulty 16 → ~21.8ms floor. Solve FIRST, then start the issuance clock
	// immediately before the first submit, so `elapsed` at that submit is ~microseconds
	// regardless of solve time or `go test` load — deterministically "too fast".
	const d = 16
	bucket := uint64(now.Unix() / pow.Window)
	ch := pow.Issue(a.masterKey, sid, d, bucket)
	nonce, ok := pow.Solve(ch.Seed, d, 1<<24)
	if !ok {
		t.Fatalf("failed to solve test PoW at difficulty %d", d)
	}
	a.store.SetPowIssued(sid, time.Now(), d)

	post := func() map[string]any {
		body := fmt.Sprintf(`{"bucket":%d,"nonce":%q}`, bucket, nonce)
		req := httptest.NewRequest(http.MethodPost, "/api/pow", strings.NewReader(body))
		req.AddCookie(&http.Cookie{Name: sessionCookie, Value: sid})
		rec := httptest.NewRecorder()
		a.handlePoW(rec, req)
		var v map[string]any
		_ = json.Unmarshal(rec.Body.Bytes(), &v)
		return v
	}

	// First submit is instant: elapsed since issuance << plausibleBrowserSolve(d),
	// so the server must reject it too-fast and grant no upgrade.
	if first := post(); first["ok"] != false {
		t.Fatalf("instant submit should be rejected too-fast, got %v", first)
	}

	// Wait past the browser-plausible floor, then resubmit the SAME valid nonce. The
	// session already proved a superhuman solve, so it must stay flagged: no pow.solved.
	floorSec := float64(uint64(1)<<uint(d)) / 3_000_000.0
	floor := time.Duration(floorSec * float64(time.Second))
	time.Sleep(floor + 50*time.Millisecond)
	if second := post(); second["ok"] == true {
		t.Fatalf("delayed resubmit laundered a too-fast solver into pow.solved: %v", second)
	}
}

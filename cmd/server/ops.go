package main

import (
	"crypto/subtle"
	"net/http"
	"os"
	"runtime"
	"strings"
	"time"
)

// ops.go is the operator/observability surface (PLAN-07 R14 + R17).
//
//   - /healthz          — always on, unauthenticated liveness (no secrets, no state).
//   - /api/explain/{id} — operator-gated production ScoreTrace: the richest debugging
//     artifact, previously reachable only via the loopback playground. It re-scores a
//     COPY (store.Get returns by value), so the production verdict path is never
//     mutated. A SessionReport carries no secret material, so nothing is redacted.
//   - /api/counters     — operator-gated runtime counters for on-call.
//
// The two gated endpoints are mounted only when an operator token is configured
// (-ops-token / HMN_OPS_TOKEN); absent, they are not even registered. A present-but-
// wrong token gets 404, so the surface is non-discoverable (matching the gate admin).

// configureOps resolves the operator token from the flag, then the HMN_OPS_TOKEN env.
func (a *app) configureOps(flagVal string) {
	tok := strings.TrimSpace(flagVal)
	if tok == "" {
		tok = strings.TrimSpace(os.Getenv("HMN_OPS_TOKEN"))
	}
	a.opsToken = tok
}

// operatorAuthorized reports whether the request carries the operator bearer token.
// A missing/mismatched token is answered with 404 (non-discoverable), never 401.
func (a *app) operatorAuthorized(w http.ResponseWriter, r *http.Request) bool {
	if a.opsToken == "" { // defense in depth: never authorize when unconfigured
		http.NotFound(w, r)
		return false
	}
	got := strings.TrimPrefix(r.Header.Get("Authorization"), "Bearer ")
	if subtle.ConstantTimeCompare([]byte(got), []byte(a.opsToken)) != 1 {
		http.NotFound(w, r)
		return false
	}
	return true
}

// handleHealthz is unauthenticated liveness — status, build, and uptime only.
func (a *app) handleHealthz(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, map[string]any{
		"status":    "ok",
		"version":   version,
		"uptimeSec": int(time.Since(a.started).Seconds()),
	})
}

// handleExplain returns the additive ScoreTrace for a stored session (SoT-30 §6),
// re-scored from a COPY so the live verdict is untouched (PLAN-07 R14).
func (a *app) handleExplain(w http.ResponseWriter, r *http.Request) {
	if !a.operatorAuthorized(w, r) {
		return
	}
	id := strings.TrimPrefix(r.URL.Path, "/api/explain/")
	rep, ok := a.store.Get(id) // returns by value → the trace scores a copy
	if !ok {
		http.NotFound(w, r)
		return
	}
	_, trace := a.engine.ScoreWithTrace(&rep)
	writeJSON(w, trace)
}

// handleCounters exposes runtime/ops counters for on-call (PLAN-07 R17).
func (a *app) handleCounters(w http.ResponseWriter, r *http.Request) {
	if !a.operatorAuthorized(w, r) {
		return
	}
	var ms runtime.MemStats
	runtime.ReadMemStats(&ms)
	writeJSON(w, map[string]any{
		"version":        version,
		"uptimeSec":      int(time.Since(a.started).Seconds()),
		"goroutines":     runtime.NumGoroutine(),
		"heapAllocBytes": ms.HeapAlloc,
		"numGC":          ms.NumGC,
	})
}

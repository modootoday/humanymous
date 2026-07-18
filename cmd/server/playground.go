package main

import (
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/modootoday/humanymous/internal/livefeed"
	"github.com/modootoday/humanymous/internal/scoring"
	"github.com/modootoday/humanymous/internal/signals"
)

// playground.go implements the dev-gated, loopback-only Detection Observatory
// surface (SoT-30). It is READ-ONLY telemetry: a page, an SSE live feed off the
// broadcast hub, and a policy-constants endpoint. The process-spawning Red
// launcher (SoT-30 Phase 3, with run-scoped state isolation + a launch nonce) is
// deliberately NOT part of this slice, so no unauthenticated endpoint can spawn
// work. The whole surface compiles/serves only when HMN_PLAYGROUND=1, and the
// server refuses to start off-loopback under that flag (see main.go).

// playgroundEnabled reports whether the Detection Observatory is served. Off by
// default; a production/promoted build never sets it.
func playgroundEnabled() bool { return os.Getenv("HMN_PLAYGROUND") == "1" }

// registerPlayground mounts the read-only observatory routes on the mux. Called
// from routes() only when the hub is live (a.hub != nil).
func (a *app) registerPlayground(mux *http.ServeMux) {
	mux.HandleFunc("/playground", a.handlePlaygroundPage)
	mux.HandleFunc("/playground/events", a.handlePlaygroundEvents)
	mux.HandleFunc("/playground/meta", a.handlePlaygroundMeta)
	mux.HandleFunc("/playground/explain/", a.handlePlaygroundExplain)
	mux.HandleFunc("/playground/launch", a.handlePlaygroundLaunch)
	mux.HandleFunc("/playground/nonce", a.handlePlaygroundNonce)
	mux.HandleFunc("/playground/abort", a.handlePlaygroundAbort)
}

// handlePlaygroundExplain returns the additive ScoreTrace (SoT-30 §6) for a
// stored session: per-layer subtotals + LayerCap hits, dedup drops (with the
// group each signal belongs to — NOT parseable from the dotted id), and the
// ordered HR-1..HR-21 evaluation. Re-scores a COPY (store.Get returns by value),
// so the production verdict path is untouched. Secret material is never present
// in a SessionReport, so nothing is redacted.
func (a *app) handlePlaygroundExplain(w http.ResponseWriter, r *http.Request) {
	if !a.playgroundGuard(w, r) {
		return
	}
	id := strings.TrimPrefix(r.URL.Path, "/playground/explain/")
	rep, ok := a.store.Get(id)
	if !ok {
		http.NotFound(w, r)
		return
	}
	_, trace := a.engine.ScoreWithTrace(&rep)
	writeJSON(w, trace)
}

// playgroundGuard enforces the loopback-only invariant + anti-embed headers on
// every playground response (SoT-30 §11.1-11.2). Rejecting a non-loopback Host
// header defeats DNS-rebinding; frame-ancestors 'none' defeats iframe wrappers.
// Returns false (and writes the error) when the request must be refused.
func (a *app) playgroundGuard(w http.ResponseWriter, r *http.Request) bool {
	if a.hub == nil {
		http.NotFound(w, r)
		return false
	}
	if !hostIsLoopback(r.Host) {
		http.Error(w, "detection observatory is loopback-only", http.StatusForbidden)
		return false
	}
	w.Header().Set("X-Frame-Options", "DENY")
	w.Header().Set("Content-Security-Policy", "frame-ancestors 'none'")
	w.Header().Set("Cache-Control", "no-store")
	return true
}

// handlePlaygroundPage serves the self-contained observatory SPA.
func (a *app) handlePlaygroundPage(w http.ResponseWriter, r *http.Request) {
	if !a.playgroundGuard(w, r) {
		return
	}
	http.ServeFile(w, r, a.webDir+"/playground.html")
}

// handlePlaygroundMeta exposes the policy constants the client needs to render
// the verdict bands and score decomposition (SoT-30 §6.3) — these are NOT in the
// /api/collect or /api/report response, so a client that hard-codes them would
// silently drift. HR scope is HR-1..HR-21 (the engine plane; HR-22..30 are
// Sentinel-plane rules the detection engine never evaluates, SoT-30 §2.1).
func (a *app) handlePlaygroundMeta(w http.ResponseWriter, r *http.Request) {
	if !a.playgroundGuard(w, r) {
		return
	}
	p := scoring.DefaultPolicy()
	writeJSON(w, map[string]any{
		"policy": map[string]any{
			"version":     p.Version,
			"layerCap":    p.LayerCap,
			"challengeAt": p.ChallengeAt,
			"denyAt":      p.DenyAt,
		},
		"bands": []map[string]any{
			{"verdict": "ALLOW", "min": 0.0, "max": p.ChallengeAt},
			{"verdict": "CHALLENGE", "min": p.ChallengeAt, "max": p.DenyAt},
			{"verdict": "DENY", "min": p.DenyAt, "max": 100.0},
		},
		"hardRuleRange": "HR-1..HR-21",
		"layers":        []string{"L1", "L2", "L3", "L4", "L5", "L6", "L7"},
	})
}

// handlePlaygroundEvents is the SSE live feed off the broadcast hub. It honors
// Last-Event-ID (header or ?lastId) for replay, sends a 15s heartbeat comment,
// and exits when the client disconnects. One goroutine per subscriber; the hub's
// non-blocking publish means a wedged reader here never affects /api/collect.
func (a *app) handlePlaygroundEvents(w http.ResponseWriter, r *http.Request) {
	if !a.playgroundGuard(w, r) {
		return
	}
	fl, ok := w.(http.Flusher)
	if !ok {
		http.Error(w, "streaming unsupported", http.StatusInternalServerError)
		return
	}
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("X-Accel-Buffering", "no")
	fmt.Fprint(w, "retry: 3000\n\n")
	fl.Flush()

	var lastID uint64
	if v := r.Header.Get("Last-Event-ID"); v != "" {
		lastID, _ = strconv.ParseUint(v, 10, 64)
	} else if v := r.URL.Query().Get("lastId"); v != "" {
		lastID, _ = strconv.ParseUint(v, 10, 64)
	}

	backlog, ch, cancel := a.hub.Subscribe(lastID)
	defer cancel()
	for _, e := range backlog {
		writeSSE(w, e)
	}
	fl.Flush()

	ticker := time.NewTicker(15 * time.Second)
	defer ticker.Stop()
	ctx := r.Context()
	for {
		select {
		case e, ok := <-ch:
			if !ok {
				return
			}
			writeSSE(w, e)
			fl.Flush()
		case <-ticker.C:
			fmt.Fprint(w, ": ping\n\n")
			fl.Flush()
		case <-ctx.Done():
			return
		}
	}
}

// writeSSE serializes one event in the text/event-stream framing.
func writeSSE(w io.Writer, e livefeed.Event) {
	fmt.Fprintf(w, "id: %d\nevent: %s\ndata: %s\n\n", e.ID, e.Type, e.Data)
}

// buildLiveEvent wraps a scored session for the live feed (SoT-30 §4.2). It
// carries the trimmed real SessionReport verbatim — the developer's own local
// self-test data. No secret key material (RIT seed/HMAC, watermark master key)
// is present in a SessionReport, so nothing is redacted here (SoT-30 §11.4).
func buildLiveEvent(sid, source string, rep *signals.SessionReport) map[string]any {
	return map[string]any{
		"sessionId": sid,
		"source":    source,
		"ts":        rep.Timestamp,
		"report":    rep,
	}
}

// liveSource tags where a scored session originated for the feed. A future
// launcher (Phase 3) sets the label; organic browsing is the human-baseline.
func liveSource(r *http.Request) string {
	if lbl := r.URL.Query().Get("label"); lbl != "" {
		return "profile:" + lbl
	}
	return "organic"
}

// hostIsLoopback reports whether an HTTP Host header names a loopback target
// (DNS-rebinding defense). Accepts 127.0.0.1[:port], [::1][:port], localhost.
func hostIsLoopback(host string) bool {
	h := host
	if hh, _, err := net.SplitHostPort(host); err == nil {
		h = hh
	}
	h = strings.TrimSuffix(strings.TrimPrefix(h, "["), "]")
	if h == "localhost" {
		return true
	}
	if ip := net.ParseIP(h); ip != nil {
		return ip.IsLoopback()
	}
	return false
}

// listenAddrIsLoopback reports whether a listen address binds only to loopback.
// An empty host (":8443") means ALL interfaces and is NOT loopback — the
// fail-closed check in main.go refuses to start the playground on it.
func listenAddrIsLoopback(addr string) bool {
	host, _, err := net.SplitHostPort(addr)
	if err != nil {
		host = addr
	}
	if host == "" {
		return false
	}
	if host == "localhost" {
		return true
	}
	if ip := net.ParseIP(host); ip != nil {
		return ip.IsLoopback()
	}
	return false
}

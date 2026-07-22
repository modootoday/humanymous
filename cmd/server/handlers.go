package main

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"io"
	"net/http"
	"strings"
	"time"

	"github.com/modootoday/humanymous/internal/signals"
)

// handlers.go holds the HTTP request logic. Each handler does one thing (SRP).

const sessionCookie = "hsid"

// handlePage serves index.html and issues a session cookie + RIT seed header.
func (a *app) handlePage(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path != "/" {
		http.NotFound(w, r)
		return
	}
	sid := a.ensureSession(w, r)
	a.store.Ensure(sid, time.Now())
	http.ServeFile(w, r, a.webDir+"/index.html")
}

// handleDemo serves the public-safe "score your browser" demo page. It reuses
// the same detection pipeline as / but renders a read-only, polished verdict view
// (no launcher, no target field, self-scoring only). It issues a session cookie
// so the WASM client + /api/collect + /api/report/{sid} flow works.
func (a *app) handleDemo(w http.ResponseWriter, r *http.Request) {
	sid := a.ensureSession(w, r)
	a.store.Ensure(sid, time.Now())
	http.ServeFile(w, r, a.webDir+"/demo.html")
}

// handleSession returns the session id + Accept-CH hints (for UA-CH cross-check).
func (a *app) handleSession(w http.ResponseWriter, r *http.Request) {
	sid := a.ensureSession(w, r)
	a.store.Ensure(sid, time.Now())
	w.Header().Set("Accept-CH", "Sec-CH-UA-Full-Version-List, Sec-CH-UA-Platform-Version, Sec-CH-UA-Arch")
	resp := map[string]any{"sessionId": sid, "ritOn": a.ritOn}
	if a.ritOn {
		seed, n, wnd := a.issueSeed(sid)
		resp["ritSeed"] = seed
		resp["ritN"] = n
		resp["ritW"] = wnd
	}
	writeJSON(w, resp)
}

// handleCollect merges the client report with the network observation, scores
// the session, stores it and returns the verdict (SoT-00 §5).
func (a *app) handleCollect(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "POST only", http.StatusMethodNotAllowed)
		return
	}
	sid := a.ensureSession(w, r)
	now := time.Now()

	body, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 1<<18))
	if err != nil {
		http.Error(w, "read error", http.StatusBadRequest)
		return
	}

	// The /api/collect pipeline, one domain step per call (see collect_pipeline.go).
	// Order is load-bearing: the RIT signal is appended by enrichServerSignals only
	// AFTER mergeObservation, which replaces Network.Signals on the first collect.
	ritSig := a.verifyAndRotateRIT(w, r, sid, body)

	client, ok := parseClientReport(w, body, sid)
	if !ok {
		return
	}
	if lbl := r.URL.Query().Get("label"); lbl != "" { // ground-truth label for e2e
		a.store.SetLabel(sid, lbl, now)
	}

	a.mergeObservation(sid, r, client, now)
	a.enrichServerSignals(sid, r, client, ritSig, now)

	res, rep := a.scoreAndStore(sid, now)
	a.publishScored(sid, r, &rep)                        // SoT-30 tap A
	a.issuePoWHeader(w, sid, res.Verdict, res.RiskScore) // SoT-13
	writeCollectResponse(w, sid, res)
}

// handleReportOne returns the full SessionReport for a session id.
func (a *app) handleReportOne(w http.ResponseWriter, r *http.Request) {
	id := strings.TrimPrefix(r.URL.Path, "/api/report/")
	rep, ok := a.store.Get(id)
	if !ok {
		http.NotFound(w, r)
		return
	}
	writeJSON(w, rep)
}

// handleReportList returns recent session summaries.
func (a *app) handleReportList(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, a.store.List(100))
}

// handleCSPReport records CSP violations as a guard signal (SoT-11).
func (a *app) handleCSPReport(w http.ResponseWriter, r *http.Request) {
	sid := cookieValue(r, sessionCookie)
	if sid != "" {
		sig := signals.New("l3.guard.csp_violation", true, signals.VerdictSuspicious, 0.7,
			signals.SourceServer, "CSP violation reported")
		a.store.AppendNetworkSignals(sid, []signals.Signal{sig}, time.Now())
	}
	w.WriteHeader(http.StatusNoContent)
}

// ensureSession returns the session id from the cookie, creating one if absent.
func (a *app) ensureSession(w http.ResponseWriter, r *http.Request) string {
	if sid := cookieValue(r, sessionCookie); sid != "" {
		return sid
	}
	sid := randHex(16)
	http.SetCookie(w, &http.Cookie{
		Name:     sessionCookie,
		Value:    sid,
		Path:     "/",
		HttpOnly: true,
		Secure:   true, // HTTPS-only (audit): humanymous is served over TLS everywhere
		SameSite: http.SameSiteLaxMode,
	})
	return sid
}

func cookieValue(r *http.Request, name string) string {
	if c, err := r.Cookie(name); err == nil {
		return c.Value
	}
	return ""
}

func randHex(n int) string {
	b := make([]byte, n)
	if _, err := rand.Read(b); err != nil {
		// A failed CSPRNG must never silently yield a predictable/zero session id
		// or launch nonce (SoT-31 R4). Fail closed: net/http recovers this panic
		// into a 500 for the request rather than issuing a guessable identifier.
		panic("crypto/rand read failed: " + err.Error())
	}
	return hex.EncodeToString(b)
}

func writeJSON(w http.ResponseWriter, v any) {
	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(v)
}

package main

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/modootoday/humanymous/internal/pow"
	"github.com/modootoday/humanymous/internal/signals"
)

// powSolvedSignal is the OK signal that grants the PoW trust upgrade (SoT-13).
func powSolvedSignal() signals.Signal {
	return signals.New("l7.pow.solved", true, signals.VerdictOK, 1.0, signals.SourceServer, "valid PoW solution")
}

// powhandler.go wires the proof-of-work active defense (SoT-13): a CHALLENGE
// verdict issues a difficulty-scaled challenge; /api/pow verifies the solution
// and, on success, upgrades the session's trust and re-scores.

const powWindow = pow.Window

// issuePoWHeader sets X-HM-PoW when a session is challenged and warrants work.
// The difficulty is derived from the session's risk so the client cannot lower it.
func (a *app) issuePoWHeader(w http.ResponseWriter, sid string, verdict string, risk float64) {
	if verdict != "CHALLENGE" || a.store.PowSolved(sid) {
		return
	}
	d := pow.DifficultyFor(risk)
	if d <= 0 {
		return
	}
	bucket := uint64(time.Now().Unix() / powWindow)
	ch := pow.Issue(a.masterKey, sid, d, bucket)
	w.Header().Set("X-HM-PoW", ch.Encode())
	a.store.SetPowIssued(sid, time.Now(), d)
}

// plausibleBrowserSolve returns a DELIBERATELY conservative lower bound on how long a real
// browser JS solver needs for a difficulty (SoT-15). A browser does ~150-300 kH/s of SHA-256
// via Web Crypto; we intentionally use a much higher 3 MH/s ceiling so the too-fast threshold
// stays small. This is not a mistake (deep-review): PoW solve time is EXPONENTIALLY
// distributed, so a mean-based threshold would flag a meaningful fraction of lucky-but-honest
// browsers — and l7.pow.too_fast LONE-DENYs via HR-14, so any false positive is a false DENY
// that violates the no-lockout constraint. The conservative ceiling keeps the control
// FP-safe: it fires only for solutions so fast they beat even a 3 MH/s solver, i.e. a
// native/GPU/precomputed solver. (The companion correctness fix is measuring elapsed from the
// FIRST challenge issuance — see Store.SetPowIssued — which previously reset every collect.)
func plausibleBrowserSolve(difficulty int) time.Duration {
	attempts := float64(uint64(1) << uint(difficulty)) // 2^difficulty mean-ish
	const ceilingHashesPerSec = 3_000_000.0
	sec := attempts / ceilingHashesPerSec
	return time.Duration(sec * float64(time.Second))
}

// handlePoW verifies a submitted PoW solution and upgrades the session.
func (a *app) handlePoW(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "POST only", http.StatusMethodNotAllowed)
		return
	}
	sid := cookieValue(r, sessionCookie)
	if sid == "" {
		http.Error(w, "no session", http.StatusBadRequest)
		return
	}
	var body struct {
		Bucket uint64 `json:"bucket"`
		Nonce  string `json:"nonce"`
	}
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 4096)).Decode(&body); err != nil {
		http.Error(w, "bad body", http.StatusBadRequest)
		return
	}

	// Verify at the ISSUE-TIME difficulty stored server-side, not one re-derived from the
	// session's CURRENT risk: a benign mid-challenge risk-band change would otherwise shift
	// the required difficulty and reject a valid human solution (deep-review). The client
	// still cannot lower it — the difficulty is server-held, not client-supplied.
	rep, _ := a.store.Get(sid)
	d := a.store.PowDifficulty(sid)
	current := uint64(time.Now().Unix() / powWindow)

	if d > 0 && pow.Verify(a.masterKey, sid, d, body.Bucket, current, body.Nonce) {
		// Solve-time analysis (SoT-15): a solution returned faster than a real
		// browser JS solver could compute is a native/GPU solver.
		elapsed := time.Since(a.store.PowIssuedAt(sid))
		if issued := a.store.PowIssuedAt(sid); !issued.IsZero() && elapsed < plausibleBrowserSolve(d) {
			rep.Network.Signals = append(rep.Network.Signals,
				signals.New("l7.pow.too_fast", elapsed.Milliseconds(), signals.VerdictBot, 1.0,
					signals.SourceServer, "PoW solved faster than a browser could (native/GPU solver)"))
			res := a.engine.Score(&rep)
			a.store.StoreScored(sid, rep, time.Now())
			writeJSON(w, map[string]any{"ok": false, "verdict": res.Verdict, "reason": "solve too fast"})
			return
		}
		a.store.SetPowSolved(sid, time.Now())
		// Re-score with the pow.solved trust upgrade applied.
		rep.Network.Signals = append(rep.Network.Signals, powSolvedSignal())
		res := a.engine.Score(&rep)
		a.store.StoreScored(sid, rep, time.Now())
		writeJSON(w, map[string]any{"ok": true, "verdict": res.Verdict, "riskScore": res.RiskScore})
		return
	}
	writeJSON(w, map[string]any{"ok": false})
}

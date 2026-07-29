package main

import (
	"encoding/json"
	"net/http"
	"strings"
	"time"

	"github.com/modootoday/humanymous/internal/attestation"
	"github.com/modootoday/humanymous/internal/gate"
	"github.com/modootoday/humanymous/internal/pass"
	"github.com/modootoday/humanymous/internal/pow"
)

// pass_handler.go is the HTTP surface of humanymous Pass (SoT-36): the interactive
// challenge served on a CHALLENGE verdict. /api/pass/new issues a freshly-seeded
// scene; /api/pass/solve verifies the player's placement + the real-event
// interaction proof server-side, and on success grants the trust upgrade and
// re-scores (mirrors the PoW upgrade path). All scoring is server-side (SoT-36 §2.4);
// the client only renders the public scene and collects interaction.
//
// The Pass concern is split by responsibility (PLAN-07 R12): per-session state and
// the anti-replay registry live in pass_store.go, the client proof + real-event
// pre-filter in pass_proof.go, and the engine-fusion axes (velocity, behavioral,
// difficulty, KPI labels) in pass_scoring.go. This file wires those into HTTP.

// passPoWSession binds the PoW seed to BOTH the session and this specific
// challenge instance, so a solved PoW cannot be reused across /new instances
// within the same time bucket.
func passPoWSession(sid, nonce string) string { return sid + "|pass|" + nonce }

// handlePassPage serves the humanymous Pass challenge/research page (SoT-36 §8:
// a non-blocking Pass section for Demo/Playground so red teams can study bypass).
func (a *app) handlePassPage(w http.ResponseWriter, r *http.Request) {
	sid := a.ensureSession(w, r)
	a.store.Ensure(sid, time.Now())
	http.ServeFile(w, r, a.webDir+"/pass.html")
}

// handlePassNew issues a FRESH Pass instance for the session. Each call resets the
// per-instance state (a new puzzle, retries reset, replay allowed) and advances the
// instance counter so the seed — and therefore the challenge — is different every time.
func (a *app) handlePassNew(w http.ResponseWriter, r *http.Request) {
	sid := a.ensureSession(w, r)
	ps := a.pass.get(sid)
	now := time.Now()
	nonce, err := passNonce()
	if err != nil {
		http.Error(w, "challenge unavailable", http.StatusInternalServerError)
		return
	}
	// Axis ③ (SoT-36 §7): meter Pass velocity first so this instance's cost already
	// reflects any automation cadence. No lockout — just a re-scored risk + PoW tax.
	velLvl, sessLvl := a.applyPassVelocity(sid, r, now)
	a.pass.mu.Lock()
	bucket := uint64(now.Unix() / passWindow)
	ps.instance++ // fresh puzzle every /new
	ps.bucket = bucket
	ps.issuedAt = now
	ps.tries = 0
	ps.solved = false
	ps.nonce = nonce
	ps.powOK = false
	ps.difficulty = a.passDifficulty(sid, r) // blue controller: harder for suspicious
	// Crypto axis (SoT-36 §2 ①): PoW cost scales with suspicion AND velocity. Velocity
	// taxes the CPU cost only — NOT the puzzle — so accessibility never regresses.
	ps.powDifficulty = 14 + ps.difficulty + velLvl*2
	if ps.powDifficulty > 20 {
		ps.powDifficulty = 20 // keep the demo/wargame solver snappy
	}
	// Axis ① upgrade (SoT-36 §2): once the identity is flagged for velocity, PoW (a CPU
	// tax) is no longer sufficient — the instance also requires a rate-limited attestation
	// token, so the crypto axis costs an identity budget, not just cycles. Gated on the
	// SESSION cadence here (production extends the trigger to the fp level, already metered).
	ps.attestReq = sessLvl >= 1
	diff := ps.difficulty
	inst := ps.instance
	powDiff := ps.powDifficulty
	attestReq := ps.attestReq
	a.pass.mu.Unlock()

	challenge := pass.Generate(a.masterKey, sid, bucket, inst, diff)
	powBucket := uint64(now.Unix() / pow.Window)
	powChallenge := pow.Issue(a.masterKey, passPoWSession(sid, nonce), powDiff, powBucket)
	writeJSON(w, map[string]any{
		"ok":             true,
		"bucket":         bucket,
		"difficulty":     diff,
		"challenge":      challenge,
		"challengeNonce": nonce,
		"triesLeft":      passMaxTries,
		"expiresInS":     passWindow * 2, // TTL spans a couple of buckets (SoT-36 §5)
		"preflight": map[string]any{
			"pow":            powChallenge, // axis ①: the non-interactive crypto proof
			"attestRequired": attestReq,    // axis ①: identity-costed token also required when flagged
		},
	})
}

// handlePassAttest issues an attestation token (SoT-36 §2 axis ①) bound to the
// caller's fingerprint + this instance's challenge nonce. Issuance is rate-limited
// PER FINGERPRINT (short, self-clearing window — never a lockout): a cookie-rotating
// flood from one JA4|subnet runs out of tokens and can no longer satisfy the crypto
// axis, closing the one-shot-per-fresh-cookie bypass that PoW alone leaves open.
func (a *app) handlePassAttest(w http.ResponseWriter, r *http.Request) {
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
		ChallengeNonce string `json:"challengeNonce"`
	}
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 4096)).Decode(&body); err != nil {
		http.Error(w, "bad body", http.StatusBadRequest)
		return
	}
	ps := a.pass.get(sid)
	a.pass.mu.Lock()
	issuedNonce := ps.nonce
	a.pass.mu.Unlock()
	if body.ChallengeNonce == "" || body.ChallengeNonce != issuedNonce {
		writeJSON(w, map[string]any{"ok": false, "reason": "stale challenge"})
		return
	}
	now := time.Now()
	// Issuance budget is keyed on the FINGERPRINT (the identity a cookie-rotating flood
	// shares); exhausted -> deny (short window; retry shortly). The token, however, binds
	// to the SESSION id so it verifies across whatever connection redeems it.
	if a.attestLim.Level(a.attestLim.Observe("att|"+a.passFP(r), now)) >= 2 {
		writeJSON(w, map[string]any{"ok": false, "reason": "attestation budget exhausted — retry shortly"})
		return
	}
	window := uint64(now.Unix() / attestation.Window)
	token := attestation.Issue(a.masterKey, sid, issuedNonce, window)
	writeJSON(w, map[string]any{"ok": true, "token": token})
}

// handlePassPoW verifies the non-interactive crypto proof (axis ①, SoT-36 §2):
// the PoW is bound to this instance's nonce, so it cannot be pre-computed or
// reused across instances. Clearing it marks the crypto axis satisfied; the solve
// still requires the real-event proof + the alignment.
func (a *app) handlePassPoW(w http.ResponseWriter, r *http.Request) {
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
		Bucket         uint64 `json:"bucket"`
		Nonce          string `json:"nonce"`          // the PoW solution
		ChallengeNonce string `json:"challengeNonce"` // must match the issued instance nonce
	}
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 4096)).Decode(&body); err != nil {
		http.Error(w, "bad body", http.StatusBadRequest)
		return
	}
	ps := a.pass.get(sid)
	a.pass.mu.Lock()
	issuedNonce, powDiff, solved := ps.nonce, ps.powDifficulty, ps.solved
	a.pass.mu.Unlock()

	current := uint64(time.Now().Unix() / pow.Window)
	ok := !solved && body.ChallengeNonce == issuedNonce && issuedNonce != "" &&
		pow.Verify(a.masterKey, passPoWSession(sid, issuedNonce), powDiff, body.Bucket, current, body.Nonce)
	if ok {
		a.pass.mu.Lock()
		if ps.nonce == issuedNonce { // still the same instance
			ps.powOK = true
		}
		a.pass.mu.Unlock()
	}
	writeJSON(w, map[string]any{"ok": ok})
}

// handlePassSolve verifies the placement + interaction proof server-side.
func (a *app) handlePassSolve(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "POST only", http.StatusMethodNotAllowed)
		return
	}
	sid := cookieValue(r, sessionCookie)
	if sid == "" {
		http.Error(w, "no session", http.StatusBadRequest)
		return
	}
	var pr passProof
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 1<<15)).Decode(&pr); err != nil {
		http.Error(w, "bad body", http.StatusBadRequest)
		return
	}

	ps := a.pass.get(sid)
	a.pass.mu.Lock()
	if ps.solved {
		a.pass.mu.Unlock()
		out := map[string]any{"ok": true, "alreadySolved": true}
		// Ceiling-guard #1: re-issue a fresh step-up receipt on re-poll so a benign receipt
		// expiry / redemption latency stall recovers WITHOUT forcing a full /api/pass/new
		// re-solve (the Pass was already genuinely solved). Only when keyed (behind a Gate).
		if a.stepUpKey != nil {
			out["stepUpReceipt"] = gate.IssueStepUpReceipt(a.stepUpKey, sid, time.Now().Add(2*time.Minute))
		}
		writeJSON(w, out)
		return
	}
	if ps.tries >= passMaxTries {
		a.pass.mu.Unlock()
		writeJSON(w, map[string]any{"ok": false, "reason": "retry cap reached", "triesLeft": 0})
		return
	}
	ps.tries++
	tries := ps.tries
	diff := ps.difficulty
	issuedBucket := ps.bucket
	instance := ps.instance
	issuedNonce := ps.nonce
	powOK := ps.powOK
	attestReq := ps.attestReq
	a.pass.mu.Unlock()

	current := uint64(time.Now().Unix() / passWindow)
	a.applyPassVelocity(sid, r, time.Now()) // axis ③: count this attempt into the velocity fusion
	rep0, _ := a.store.Get(sid)
	label := passLabel(rep0.Scoring.RiskScore, r) // wargame ground-truth-ish label

	reject := func(why string) {
		a.pass.record(label, diff, false)
		a.feedPassOutcome(rep0, false) // SoT-42: solve-rate guard only (a failed Pass is not a bot; ACC-1)
		a.publishPass(sid, false, why)
		writeJSON(w, map[string]any{"ok": false, "reason": why, "triesLeft": passMaxTries - tries})
	}

	// 1. Instance binding (axis ①) — the solve must carry this instance's nonce.
	if issuedNonce == "" || pr.ChallengeNonce != issuedNonce {
		reject("stale or missing challenge")
		return
	}

	// 2. Crypto axis (axis ①, SoT-36 §2) — the non-interactive proof must be paid.
	// This carries the weak/keyboard lane where motor microstructure is inadmissible.
	if !powOK {
		reject("no cryptographic preflight")
		return
	}

	// 2b. Identity gate (axis ① upgrade): when the fingerprint is flagged, PoW alone
	// no longer buys an attempt — a rate-limited attestation token (bound to this
	// fingerprint + nonce) is also required, so throughput costs an identity budget.
	if attestReq {
		window := uint64(time.Now().Unix() / attestation.Window)
		if !attestation.Verify(a.masterKey, sid, issuedNonce, pr.AttestToken, window) {
			reject("attestation required")
			return
		}
	}

	// 3. Real-event pre-filter (SoT-36 §5) — degenerate/synthetic submissions.
	if ok, why := realEventOK(pr); !ok {
		reject(why)
		return
	}

	// 3b. Deeper real-event model (SoT-36 §5), fused SOFT — behavioral tells raise risk
	// but never block (accessibility), so even a fresh-identity forgery with motor tells
	// is held at elevated verdict.
	a.applyPassBehavior(sid, pr, time.Now())

	// 5a. Expiry is NOT a wrong answer (audit ACC-1): a slow assistive-tech user must
	// never be told "not solved" for taking their time. Report expiry honestly so the
	// client can announce it and offer a fresh puzzle.
	if !pass.Fresh(issuedBucket, current) {
		a.publishPass(sid, false, "expired")
		writeJSON(w, map[string]any{"ok": false, "expired": true, "reason": "this check expired — press New puzzle to start a fresh one"})
		return
	}

	// 5b. Server-side re-simulation of the placement (the whole verdict; no oracle).
	if !pass.Verify(a.masterKey, sid, issuedBucket, current, instance, diff, pr.Offsets) {
		a.pass.record(label, diff, false)
		a.publishPass(sid, false, "misaligned")
		writeJSON(w, map[string]any{"ok": false, "reason": "the keys are not all in the slot", "triesLeft": passMaxTries - tries})
		return
	}

	// 5c. Anti-replay (SoT-36 §5) — a captured human trace may be used at most once. Reserved
	// ONLY after the solve is verified (round-5): reserving before Verify let a failed/expired
	// submission permanently consume a slot (registry pollution) and, at the bounded cap, lock
	// out every subsequent legitimate first solve. A slot is now consumed only by a genuine,
	// verified solving trace, so saturation is bounded by real human solve throughput.
	if !a.pass.reserveTrace(traceDigest(pr), time.Now()) {
		reject("duplicate interaction trace")
		return
	}
	a.pass.record(label, diff, true)

	// 6. Cleared — grant the trust upgrade and re-score (mirrors PoW upgrade).
	a.pass.mu.Lock()
	ps.solved = true
	a.pass.mu.Unlock()
	a.publishPass(sid, true, "")

	rep, _ := a.store.Get(sid)
	rep.Network.Signals = append(rep.Network.Signals, passSolvedSignal())
	res := a.engine.Score(&rep)
	a.store.StoreScored(sid, rep, time.Now())
	a.feedPassOutcome(rep0, true) // SoT-42: a solved Pass is a confirmed human — the ACI oracle signal
	out := map[string]any{"ok": true, "verdict": res.Verdict, "riskScore": res.RiskScore}
	// Ceiling-guard #1: a verified human Pass solve is exactly the step-up the attestation
	// floor demands. When the Core runs as a Pass front-end behind a Gate (shared token
	// key present), hand the client a short-lived, session-bound receipt to redeem at the
	// Gate's /stepup for the socket-bound hmn_su proof. Standalone (no key) => omitted.
	if a.stepUpKey != nil {
		out["stepUpReceipt"] = gate.IssueStepUpReceipt(a.stepUpKey, sid, time.Now().Add(2*time.Minute))
	}
	writeJSON(w, out)
}

// handlePassKPI reports the wargame KPIs (SoT-36 §8): the closed-loop scoreboard
// the blue team hardens against — bypass-rate (bots/red that obtained a pass) and
// the human pass-rate floor, plus per-difficulty solve rates.
func (a *app) handlePassKPI(w http.ResponseWriter, r *http.Request) {
	a.pass.mu.Lock()
	snap := make(map[string]int, len(a.pass.metrics))
	for k, v := range a.pass.metrics {
		snap[k] = v
	}
	a.pass.mu.Unlock()

	// aggregate: label -> {pass, attempts}; and per-difficulty.
	type pa struct{ pass, att int }
	byLabel := map[string]*pa{}
	byDiff := map[string]*pa{}
	for k, n := range snap {
		// key = "label|outcome|dN"
		var label, outcome, dd string
		parts := strings.Split(k, "|") // PLAN-07 R7
		if len(parts) != 3 {
			continue
		}
		label, outcome, dd = parts[0], parts[1], parts[2]
		if byLabel[label] == nil {
			byLabel[label] = &pa{}
		}
		if byDiff[dd] == nil {
			byDiff[dd] = &pa{}
		}
		byLabel[label].att += n
		byDiff[dd].att += n
		if outcome == "pass" {
			byLabel[label].pass += n
			byDiff[dd].pass += n
		}
	}
	rate := func(p *pa) float64 {
		if p == nil || p.att == 0 {
			return 0
		}
		return float64(p.pass) / float64(p.att)
	}
	botAtt, botPass := 0, 0
	perStrategy := map[string]float64{}
	for label, p := range byLabel {
		if !isBotLabel(label) {
			continue
		}
		botAtt += p.att
		botPass += p.pass
		perStrategy[label] = rate(p)
	}
	bypass := 0.0
	if botAtt > 0 {
		bypass = float64(botPass) / float64(botAtt)
	}

	diffRates := map[string]float64{}
	for d, p := range byDiff {
		diffRates[d] = rate(p)
	}
	humanAtt := 0
	if p := byLabel["human"]; p != nil {
		humanAtt = p.att
	}
	writeJSON(w, map[string]any{
		"bypassRate":      bypass, // bots/red that obtained a pass — drive DOWN
		"botAttempts":     botAtt,
		"perStrategy":     perStrategy,            // bypass rate per red-team strategy
		"humanPassRate":   rate(byLabel["human"]), // the floor the blue team may never breach
		"humanAttempts":   humanAtt,
		"unknownPassRate": rate(byLabel["unknown"]),
		"perDifficulty":   diffRates,
		"raw":             snap,
	})
}

// publishPass emits a Pass attempt to the live telemetry hub (wargame surface,
// SoT-36 §8) when the Observatory is enabled; a no-op otherwise.
func (a *app) publishPass(sid string, solved bool, reason string) {
	if a.hub == nil {
		return
	}
	short := sid
	if len(short) > 8 {
		short = short[:8]
	}
	a.hub.Publish("pass.attempt", map[string]any{"sid": short, "solved": solved, "reason": reason})
}

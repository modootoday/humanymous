package main

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"math"
	"net/http"
	"sync"
	"time"

	"github.com/modootoday/humanymous/internal/pass"
	"github.com/modootoday/humanymous/internal/pow"
	"github.com/modootoday/humanymous/internal/signals"
)

// pass_handler.go wires humanymous Pass (SoT-36) into the reference engine: the
// interactive challenge served on a CHALLENGE verdict. /api/pass/new issues a
// freshly-seeded scene; /api/pass/solve verifies the player's placement + the
// real-event interaction proof server-side, and on success grants the trust
// upgrade and re-scores (mirrors the PoW upgrade path). All scoring is server-side
// (SoT-36 §2.4); the client only renders the public scene and collects interaction.

const (
	passWindow   = 30 // seconds per bucket (challenge TTL granularity)
	passMaxTries = 4  // hard retry cap per session (SoT-36 §2.3: fail -> new instance)
)

// passSession is the per-session Pass state (in-memory; the challenge is transient).
type passSession struct {
	bucket        uint64
	instance      uint64 // increments on each /new so every puzzle is fresh
	difficulty    int
	issuedAt      time.Time
	tries         int
	solved        bool
	nonce         string // per-instance challenge nonce; binds the crypto proof + the solve
	powDifficulty int    // PoW leading-zero-bit target for this instance
	powOK         bool   // the crypto axis (①) has been satisfied for this instance
}

// passStore holds transient per-session Pass state + the wargame metrics, guarded
// by a mutex. metrics is the blue team's evidence base (SoT-36 §8): attempts
// counted by label × outcome × difficulty. traces is the anti-replay registry:
// the digest of every accepted motor trace, so a captured human trace can be
// replayed at most once (SoT-36 §5).
type passStore struct {
	mu      sync.Mutex
	m       map[string]*passSession
	metrics map[string]int       // "label|outcome|difficulty" -> count
	traces  map[string]time.Time // motor-trace digest -> first-seen (anti-replay)
}

func newPassStore() *passStore {
	return &passStore{
		m:       map[string]*passSession{},
		metrics: map[string]int{},
		traces:  map[string]time.Time{},
	}
}

// reserveTrace registers a motor-trace digest and reports whether it is FRESH
// (never seen inside the retention window). A replayed capture — identical raw
// timings — collides on the digest and is rejected. Old entries are swept lazily.
func (p *passStore) reserveTrace(digest string, now time.Time) bool {
	p.mu.Lock()
	defer p.mu.Unlock()
	for k, seen := range p.traces {
		if now.Sub(seen) > 10*time.Minute {
			delete(p.traces, k)
		}
	}
	if _, exists := p.traces[digest]; exists {
		return false
	}
	p.traces[digest] = now
	return true
}

// traceDigest fingerprints the raw motor evidence (NOT the offsets/nonce, which
// change per instance) so a replay of the same captured trace collides even when
// wrapped around a fresh challenge (SoT-36 §5).
func traceDigest(pr passProof) string {
	h := sha256.New()
	fmt.Fprintf(h, "%v|%v|%v|%v", pr.RawT, pr.Durations, pr.Pressures, pr.KeyDurs)
	return hex.EncodeToString(h.Sum(nil))
}

// passNonce mints a fresh per-instance challenge nonce.
func passNonce() (string, error) {
	var b [18]byte
	if _, err := rand.Read(b[:]); err != nil {
		return "", err
	}
	return hex.EncodeToString(b[:]), nil
}

// passPoWSession binds the PoW seed to BOTH the session and this specific
// challenge instance, so a solved PoW cannot be reused across /new instances
// within the same time bucket.
func passPoWSession(sid, nonce string) string { return sid + "|pass|" + nonce }

// record logs one attempt into the wargame metrics (caller need not hold the lock).
func (p *passStore) record(label string, difficulty int, passed bool) {
	outcome := "fail"
	if passed {
		outcome = "pass"
	}
	key := label + "|" + outcome + "|d" + itoaSmall(difficulty)
	p.mu.Lock()
	p.metrics[key]++
	p.mu.Unlock()
}

func itoaSmall(n int) string {
	if n < 0 || n > 9 {
		return "?"
	}
	return string(rune('0' + n))
}

func (p *passStore) get(sid string) *passSession {
	p.mu.Lock()
	defer p.mu.Unlock()
	s := p.m[sid]
	if s == nil {
		s = &passSession{}
		p.m[sid] = s
	}
	return s
}

// passSolvedSignal is the OK signal that grants the Pass trust upgrade (SoT-36 §3).
func passSolvedSignal() signals.Signal {
	return signals.New("l7.pass.solved", true, signals.VerdictOK, 1.0, signals.SourceServer, "humanymous Pass cleared")
}

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
	ps.powDifficulty = 14 + ps.difficulty    // crypto axis scales with suspicion (SoT-36 §2 ①)
	diff := ps.difficulty
	inst := ps.instance
	powDiff := ps.powDifficulty
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
		"expiresInS":     passWindow * 2,                      // TTL spans a couple of buckets (SoT-36 §5)
		"preflight":      map[string]any{"pow": powChallenge}, // axis ①: the non-interactive crypto proof
	})
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

// passDifficulty is the blue controller's knob (SoT-36 §8): harder challenges for
// suspicious sessions, trivial for likely-humans. Difficulty scales with the
// session's current risk; a self-labelled red-team probe always gets the hardest.
func (a *app) passDifficulty(sid string, r *http.Request) int {
	if r.Header.Get("X-HM-Redteam") != "" {
		return 3
	}
	rep, _ := a.store.Get(sid)
	risk := rep.Scoring.RiskScore
	d := int((risk - 20) / 15) // 30->0, 45->1, 60->2, 75->3
	if d < 0 {
		d = 0
	}
	if d > 3 {
		d = 3
	}
	return d
}

// passLabel classifies an attempt for the wargame KPIs (SoT-36 §8). A self-declared
// red-team probe reports its STRATEGY name (ground-truth bot, per-strategy KPI);
// otherwise the session's risk band is a weak label (bot >=70, human <30, else
// unknown). Strategy names are header values with no '|' so the metric key is safe.
func passLabel(risk float64, r *http.Request) string {
	if s := r.Header.Get("X-HM-Redteam"); s != "" {
		return s
	}
	switch {
	case risk >= 70:
		return "bot"
	case risk < 30:
		return "human"
	default:
		return "unknown"
	}
}

// isBotLabel reports whether a KPI label is an adversary (a red-team strategy or a
// high-risk "bot"), as opposed to the human floor / ambiguous "unknown" band.
func isBotLabel(label string) bool { return label != "human" && label != "unknown" }

// passProof is the client submission: the 3-row offsets + an interaction proof.
type passProof struct {
	Bucket         uint64 `json:"bucket"`
	ChallengeNonce string `json:"challengeNonce"` // binds the solve to the issued instance (axis ①)
	Offsets        []int  `json:"offsets"`        // per-row shift; key lands at (keyIndex+offset) mod N
	Trusted        bool   `json:"trusted"`        // all events had isTrusted === true (pre-filter)
	// Pointer/touch channel (SoT-36 §5): mouse/touch users produce these.
	Moves     int       `json:"moves"`     // distinct pointermove events
	Coalesced int       `json:"coalesced"` // total getCoalescedEvents() sub-samples
	Durations []float64 `json:"durations"` // inter-event Δt (ms)
	PathLen   float64   `json:"pathLen"`   // total pointer path length (px)
	RawT      []float64 `json:"rawT"`      // raw coalesced sample timestamps (ms)
	Pressures []float64 `json:"pressures"` // touch/pen pressure samples (0..1)
	// Keyboard channel (accessible lane): keyboard users produce these instead.
	Keys    int       `json:"keys"`    // distinct arrow/Home keydowns
	KeyDurs []float64 `json:"keyDurs"` // inter-key Δt (ms)
}

// realEventOK is the SoT-36 §5 pre-filter, accessibility-aware: it accepts EITHER a
// pointer/touch channel OR a keyboard channel, rejecting only the obviously synthetic
// (untrusted, no interaction, perfectly-uniform timing). Keyboard users are NEVER
// required to produce pointer microstructure (that would exclude blind/AT users). It
// is a soft pre-filter, not the whole gate — attestation + engine fusion carry the
// weak/keyboard case (SoT-36 §2), and the deeper motor model is the wargame's job.
func realEventOK(pr passProof) (bool, string) {
	if !pr.Trusted {
		return false, "untrusted events"
	}
	pointer := pr.Moves >= 5 && pr.PathLen >= 20
	keyboard := pr.Keys >= 3
	if !pointer && !keyboard {
		return false, "insufficient interaction"
	}
	// Keyboard path: irregular inter-key timing is human; uniform is a bot tell.
	if keyboard && len(pr.KeyDurs) >= 4 && stddev(pr.KeyDurs) < 0.4 {
		return false, "uniform key timing"
	}
	// Pointer path: uniform Δt + missing coalesced sub-samples + no raw stream are the
	// CDP/forged-aggregate tells. Only enforced when the user actually used a pointer.
	if pointer && !keyboard {
		if len(pr.Durations) >= 5 && stddev(pr.Durations) < 0.5 {
			return false, "uniform event timing"
		}
		if pr.Coalesced != 0 && pr.Coalesced <= pr.Moves {
			return false, "no coalesced sub-samples"
		}
		if len(pr.RawT) < 10 {
			return false, "missing raw input stream"
		}
		diffs := make([]float64, 0, len(pr.RawT)-1)
		for i := 1; i < len(pr.RawT); i++ {
			dd := pr.RawT[i] - pr.RawT[i-1]
			if dd < 0 {
				return false, "non-monotonic raw timestamps"
			}
			diffs = append(diffs, dd)
		}
		if stddev(diffs) < 0.15 {
			return false, "uniform raw sample spacing"
		}
	}
	return true, ""
}

func stddev(xs []float64) float64 {
	if len(xs) == 0 {
		return 0
	}
	var sum float64
	for _, x := range xs {
		sum += x
	}
	mean := sum / float64(len(xs))
	var v float64
	for _, x := range xs {
		d := x - mean
		v += d * d
	}
	return math.Sqrt(v / float64(len(xs)))
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
		writeJSON(w, map[string]any{"ok": true, "alreadySolved": true})
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
	a.pass.mu.Unlock()

	current := uint64(time.Now().Unix() / passWindow)
	rep0, _ := a.store.Get(sid)
	label := passLabel(rep0.Scoring.RiskScore, r) // wargame ground-truth-ish label

	reject := func(why string) {
		a.pass.record(label, diff, false)
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

	// 3. Real-event pre-filter (SoT-36 §5) — degenerate/synthetic submissions.
	if ok, why := realEventOK(pr); !ok {
		reject(why)
		return
	}

	// 4. Anti-replay (SoT-36 §5) — a captured human trace may be used at most once.
	if !a.pass.reserveTrace(traceDigest(pr), time.Now()) {
		reject("duplicate interaction trace")
		return
	}

	// 5. Server-side re-simulation of the placement (the whole verdict; no oracle).
	if !pass.Verify(a.masterKey, sid, issuedBucket, current, instance, diff, pr.Offsets) {
		a.pass.record(label, diff, false)
		a.publishPass(sid, false, "misaligned")
		writeJSON(w, map[string]any{"ok": false, "reason": "the keys are not all in the slot", "triesLeft": passMaxTries - tries})
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
	writeJSON(w, map[string]any{"ok": true, "verdict": res.Verdict, "riskScore": res.RiskScore})
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
		parts := splitKey(k)
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

// splitKey splits "a|b|c" into 3 parts (no strings.Split import churn needed here,
// but keep it simple).
func splitKey(s string) []string {
	out := []string{}
	cur := ""
	for _, c := range s {
		if c == '|' {
			out = append(out, cur)
			cur = ""
		} else {
			cur += string(c)
		}
	}
	return append(out, cur)
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

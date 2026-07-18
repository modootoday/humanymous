package main

import (
	"encoding/json"
	"math"
	"net/http"
	"sync"
	"time"

	"github.com/modootoday/humanymous/internal/pass"
	"github.com/modootoday/humanymous/internal/signals"
)

// pass_handler.go wires humanymous Pass (SoT-36) into the reference engine: the
// interactive challenge served on a CHALLENGE verdict. /api/pass/new issues a
// freshly-seeded scene; /api/pass/solve verifies the player's placement + the
// real-event interaction proof server-side, and on success grants the trust
// upgrade and re-scores (mirrors the PoW upgrade path). All scoring is server-side
// (SoT-36 §2.4); the client only renders the public scene and collects interaction.

const (
	passWindow  = 30 // seconds per bucket (challenge TTL granularity)
	passMaxTries = 4  // hard retry cap per session (SoT-36 §2.3: fail -> new instance)
)

// passSession is the per-session Pass state (in-memory; the challenge is transient).
type passSession struct {
	bucket     uint64
	difficulty int
	issuedAt   time.Time
	tries      int
	solved     bool
}

// passStore holds transient per-session Pass state + the wargame metrics, guarded
// by a mutex. metrics is the blue team's evidence base (SoT-36 §8): attempts
// counted by label × outcome × difficulty.
type passStore struct {
	mu      sync.Mutex
	m       map[string]*passSession
	metrics map[string]int // "label|outcome|difficulty" -> count
}

func newPassStore() *passStore {
	return &passStore{m: map[string]*passSession{}, metrics: map[string]int{}}
}

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

// handlePassNew issues a fresh Pass instance (scene + bucket) for the session.
func (a *app) handlePassNew(w http.ResponseWriter, r *http.Request) {
	sid := a.ensureSession(w, r)
	ps := a.pass.get(sid)
	a.pass.mu.Lock()
	if ps.solved {
		a.pass.mu.Unlock()
		writeJSON(w, map[string]any{"ok": true, "alreadySolved": true})
		return
	}
	bucket := uint64(time.Now().Unix() / passWindow)
	ps.bucket = bucket
	ps.issuedAt = time.Now()
	ps.difficulty = a.passDifficulty(sid, r) // blue controller: harder for suspicious
	diff := ps.difficulty
	a.pass.mu.Unlock()

	scene := pass.Generate(a.masterKey, sid, bucket, diff)
	writeJSON(w, map[string]any{
		"ok":         true,
		"bucket":     bucket,
		"difficulty": diff,
		"scene":      scene,
		"triesLeft":  passMaxTries - ps.tries,
		"expiresInS": passWindow * 2, // TTL spans a couple of buckets (SoT-36 §5)
	})
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
// red-team probe is ground-truth "bot"; otherwise the session's risk band is a weak
// label (bot >=70, human <30, else unknown).
func passLabel(risk float64, r *http.Request) string {
	if r.Header.Get("X-HM-Redteam") != "" {
		return "redteam"
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

// passProof is the client submission: the ramp placement + an interaction proof.
type passProof struct {
	Bucket    uint64  `json:"bucket"`
	RampX     float64 `json:"rampX"`
	RampY     float64 `json:"rampY"`
	RampAngle float64 `json:"rampAngle"`
	// real-event interaction evidence (SoT-36 §5): pointer-move samples collected
	// during placement. isTrusted is a pre-filter; the discriminator is the stream.
	Moves      int       `json:"moves"`      // distinct pointermove events
	Coalesced  int       `json:"coalesced"`  // total getCoalescedEvents() sub-samples
	Trusted    bool      `json:"trusted"`    // all events had isTrusted === true
	Durations  []float64 `json:"durations"`  // inter-event Δt (ms), for uniformity check
	PathLen    float64   `json:"pathLen"`    // total pointer path length (px)
}

// realEventOK is a lightweight real-event check (SoT-36 §5, v1): reject the obvious
// synthetic/empty cases (no interaction, untrusted events, perfectly-uniform timing,
// no coalesced sub-samples). Scored leniently — full motor modeling is a follow-up;
// this is a hard pre-filter for degenerate bot submissions, not the whole gate.
func realEventOK(pr passProof) (bool, string) {
	if !pr.Trusted {
		return false, "untrusted events"
	}
	if pr.Moves < 6 || pr.PathLen < 20 {
		return false, "insufficient interaction"
	}
	// Uniform timing tell (CDP emits identical Δt regardless of distance).
	if len(pr.Durations) >= 5 && stddev(pr.Durations) < 0.5 {
		return false, "uniform event timing"
	}
	// Real pointer moves coalesce sub-frame samples; a total of exactly `moves`
	// (one sample each) is the CDP structural tell. Neutral if unsupported (== 0).
	if pr.Coalesced != 0 && pr.Coalesced <= pr.Moves {
		return false, "no coalesced sub-samples"
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
	a.pass.mu.Unlock()

	current := uint64(time.Now().Unix() / passWindow)
	rep0, _ := a.store.Get(sid)
	label := passLabel(rep0.Scoring.RiskScore, r) // wargame ground-truth-ish label

	// 1. Real-event pre-filter (SoT-36 §5) — degenerate/synthetic submissions.
	if ok, why := realEventOK(pr); !ok {
		a.pass.record(label, diff, false)
		a.publishPass(sid, false, why)
		writeJSON(w, map[string]any{"ok": false, "reason": why, "triesLeft": passMaxTries - tries})
		return
	}

	// 2. Server-side re-simulation of the placement (the whole verdict; no oracle).
	if !pass.Verify(a.masterKey, sid, pr.Bucket, current, diff, pr.RampX, pr.RampY, pr.RampAngle) {
		a.pass.record(label, diff, false)
		a.publishPass(sid, false, "placement missed")
		writeJSON(w, map[string]any{"ok": false, "reason": "the ball missed the cup", "triesLeft": passMaxTries - tries})
		return
	}
	a.pass.record(label, diff, true)

	// 3. Cleared — grant the trust upgrade and re-score (mirrors PoW upgrade).
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
	for _, l := range []string{"redteam", "bot"} {
		if p := byLabel[l]; p != nil {
			botAtt += p.att
			botPass += p.pass
		}
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

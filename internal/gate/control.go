package gate

import (
	"crypto/rand"
	"encoding/hex"
	"io"
	"net/http"
	"sync/atomic"
	"time"

	"github.com/modootoday/humanymous/internal/abuse"
	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/collector"
	"github.com/modootoday/humanymous/internal/httpx"
	"github.com/modootoday/humanymous/internal/network"
	"github.com/modootoday/humanymous/internal/scoring"
	"github.com/modootoday/humanymous/internal/signals"
)

// control.go is the reserved control-plane (SoT-19 §3): session issuance, the
// /collect scoring beacon that updates the sticky verdict, and the loader. It
// reuses the validated scoring engine + collector store so verdicts are real.
// Every beacon scoring is emitted into the audit chain.

// ControlPlane serves /__hmn/* (prefix already stripped by the proxy).
type ControlPlane struct {
	store       *collector.Store
	engine      *scoring.Engine
	verdicts    VerdictLedger // distribution seam (PLAN-08 R1): in-mem now, shared Redis later
	sink        *audit.Sink
	vault       *audit.Vault
	tokenKey    []byte         // verdict-token HMAC key (SoT-21 §3)
	epoch       string         // fallback token epoch
	tokenEpochs *EpochManager  // shared rotating epoch (SoT-28 WS6)
	nonces      *NonceCache    // single-use beacon nonce cache (HR-29)
	climiter    *abuse.Limiter // control-plane flood limiter (SoT-28 WS7)
	cpHard      int            // control-plane hard threshold (for sampled audit)
	cohort      *cohortShadow  // ceiling-guard #3: population/cohort behavioral shadow (log-only); nil = off
	nowFn       func() time.Time
	ready       atomic.Bool // readiness gate: true once serving, flipped false on shutdown so an LB drains first
	// configureEngine optionally reloads SoT-39 Effective into the Engine before Score (P2).
	// Nil = never touch Engine (freeze defaults). Must be pure Configure of scoring inputs.
	configureEngine func(*scoring.Engine)
}

// NewControlPlane wires the control plane against shared state. The control
// plane is metered so /collect and /session cannot be flooded to exhaust CPU or
// bloat the audit chain (SoT-28 WS7): the ban gate does not cover it, so it must
// protect itself. Defaults are generous (300 requests / 10s per IP) — a real
// page makes ~2 control-plane calls; a flood is thousands/sec.
// WithEngineConfigurator sets the SoT-39 hook that applies Effective overlay knobs
// to the Engine before each score. Empty-overlay Configure must remain freeze-identical.
func (c *ControlPlane) WithEngineConfigurator(fn func(*scoring.Engine)) *ControlPlane {
	c.configureEngine = fn
	return c
}

func NewControlPlane(store *collector.Store, engine *scoring.Engine, verdicts VerdictLedger, sink *audit.Sink, vault *audit.Vault) *ControlPlane {
	c := &ControlPlane{store: store, engine: engine, verdicts: verdicts, sink: sink, vault: vault,
		epoch: "e1", nonces: NewNonceCache(10 * time.Minute),
		climiter: abuse.NewLimiter(10*time.Second, 150, 300), cpHard: 300, nowFn: time.Now}
	c.ready.Store(true) // ready to serve once constructed; SetReady(false) on shutdown
	return c
}

// SetReady toggles the readiness gate. Call SetReady(false) at the start of a
// graceful shutdown so /__hmn/readyz returns 503 and a load balancer stops routing
// new traffic to this node before the drain begins. Liveness (/healthz) stays ok.
func (c *ControlPlane) SetReady(ready bool) { c.ready.Store(ready) }

// WithControlLimiter overrides the control-plane flood thresholds (tests/tuning).
func (c *ControlPlane) WithControlLimiter(window time.Duration, soft, hard int) *ControlPlane {
	c.climiter = abuse.NewLimiter(window, soft, hard)
	c.cpHard = hard
	return c
}

// cpGuard meters the caller and drops a flooding source with 429 BEFORE any
// scoring/session work (SoT-28 WS7). It audits only the FIRST hard breach per
// window so a flood does not itself bloat the tamper-evident chain.
func (c *ControlPlane) cpGuard(w http.ResponseWriter, r *http.Request, endpoint string) bool {
	if c.climiter == nil {
		return false
	}
	count := c.climiter.Observe("cp:"+clientIP(r), c.nowFn())
	if c.climiter.Level(count) < 2 {
		return false
	}
	if count == c.cpHard+1 { // sampled: chain the breach once, not every flood request
		c.sink.Emit(audit.Record{
			EventType: audit.EventRateHardExceeded, Actor: audit.Actor{Kind: "system"},
			TenantID: "control", RouteClass: "control", Verdict: string(VerdictDeny),
			Action: "rate_limit", Mode: "enforce", FailReason: "control-plane flood: " + endpoint, KeyID: "k1",
		})
	}
	w.Header().Set("Retry-After", "10")
	http.Error(w, "control plane rate limited", http.StatusTooManyRequests)
	return true
}

// WithTokenKey enables verdict-token issuance on ALLOW (SoT-21 §3, HR-28).
func (c *ControlPlane) WithTokenKey(key []byte) *ControlPlane { c.tokenKey = key; return c }

// WithCohortShadow enables the ceiling-guard #3 population/cohort behavioral shadow
// (log-only; never affects the verdict). Off by default.
func (c *ControlPlane) WithCohortShadow() *ControlPlane { c.cohort = newCohortShadow(); return c }

// GC age-evicts the control-plane observers' bounded state (the cohort shadow). Call
// it periodically alongside the server/store GC.
func (c *ControlPlane) GC(now time.Time) {
	if c.cohort != nil {
		c.cohort.gc(now)
	}
}

// WithTokenEpochs shares the rotating epoch manager with the edge (SoT-28 WS6).
func (c *ControlPlane) WithTokenEpochs(em *EpochManager) *ControlPlane { c.tokenEpochs = em; return c }

// tokEpoch returns the epoch to mint under (shared manager, else the fixed one).
func (c *ControlPlane) tokEpoch() string {
	if c.tokenEpochs != nil {
		return c.tokenEpochs.Current()
	}
	return c.epoch
}

// Handler returns the control-plane mux.
func (c *ControlPlane) Handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("/session", c.handleSession)
	mux.HandleFunc("/collect", c.handleCollect)
	mux.HandleFunc("/stepup", c.handleStepUp)
	mux.HandleFunc("/loader.js", c.handleLoader)
	mux.HandleFunc("/csp-report", func(w http.ResponseWriter, r *http.Request) { w.WriteHeader(http.StatusNoContent) })
	// Always-on unauthenticated liveness probe (PLAN-08 backlog): LBs/orchestrators
	// need a real probe for a wedged Gate. Unlike the /health origin-bypass route, this
	// is answered by the Gate itself and carries no data.
	mux.HandleFunc("/healthz", func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, map[string]any{"status": "ok", "plane": "gate"})
	})
	// Readiness probe: 200 while serving, 503 once a graceful shutdown has flipped the
	// readiness gate — so a load balancer removes this node BEFORE the drain begins.
	// Point the orchestrator's readinessProbe here and its livenessProbe at /healthz.
	mux.HandleFunc("/readyz", func(w http.ResponseWriter, r *http.Request) {
		if !c.ready.Load() {
			w.WriteHeader(http.StatusServiceUnavailable)
			writeJSON(w, map[string]any{"status": "draining", "plane": "gate"})
			return
		}
		writeJSON(w, map[string]any{"status": "ready", "plane": "gate"})
	})
	return mux
}

func (c *ControlPlane) handleSession(w http.ResponseWriter, r *http.Request) {
	if c.cpGuard(w, r, "session") {
		return
	}
	sid := c.ensureSession(w, r)
	c.store.Ensure(sid, c.nowFn())
	writeJSON(w, map[string]any{"sessionId": sid})
}

// handleCollect scores the client beacon + header-derived network signals,
// updates the sticky verdict, and audits the evaluation (SoT-19 step 5).
func (c *ControlPlane) handleCollect(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "POST only", http.StatusMethodNotAllowed)
		return
	}
	if c.cpGuard(w, r, "collect") {
		return
	}
	sid := c.ensureSession(w, r)
	now := c.nowFn()

	body, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 1<<18))
	if err != nil {
		http.Error(w, "read error", http.StatusBadRequest)
		return
	}
	client, ok := parseControlClient(w, body, sid)
	if !ok {
		return
	}

	// One domain step per call (see control_collect.go).
	if c.rejectBeaconReplay(w, r, sid, now) { // HR-29
		return
	}
	res := c.scoreBeacon(sid, r, client, now)
	c.updateStickyVerdict(sid, res, now)
	c.issueVerdictTokenOnAllow(w, r, sid, res, now) // HR-28
	c.linkAndAuditScoring(sid, r, res)
	writeControlVerdict(w, sid, res)
}

// handleStepUp mints the attestation-floor STEP-UP PROOF cookie (ceiling-guard #1).
// It is the redemption endpoint for an LLM-resistant SoT-36 Pass solve: the Pass
// server (which may sit behind an XFF hop) hands the client a SESSION-bound receipt
// on a verified solve; the client presents it here, and the gate — which sees the
// authoritative live socket — RE-BINDS the proof to this fingerprint+subnet and sets
// hmn_su. Re-binding at the gate is what makes the proof non-liftable to another
// machine even though the receipt itself is only session-bound. A missing/invalid/
// expired receipt is a 403; this endpoint grants nothing on its own.
func (c *ControlPlane) handleStepUp(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "POST only", http.StatusMethodNotAllowed)
		return
	}
	if c.cpGuard(w, r, "stepup") {
		return
	}
	if len(c.tokenKey) == 0 {
		http.Error(w, "step-up not configured", http.StatusNotImplemented)
		return
	}
	sid := sessionID(r)
	if sid == "" {
		http.Error(w, "no session", http.StatusBadRequest)
		return
	}
	now := c.nowFn()
	receipt := r.Header.Get(stepUpReceiptHeader)
	if reason := verifyStepUpReceipt(c.tokenKey, receipt, sid, now); reason != receiptOK {
		// A VALID-signature receipt whose sid does not match this request's hsid is the
		// fingerprint of a diverged cookie jar (Core Pass not co-served in the Gate's cookie
		// scope) — a real human who genuinely solved the Pass now loops. Surface it as an
		// actionable misconfig, distinct from an absent/forged/expired receipt, so operators
		// are not blind to a human soft-lockout that reads like a blocked attacker.
		msg := "step-up: missing/invalid Pass receipt (" + string(reason) + ")"
		if reason == receiptSIDMismatch && receipt != "" {
			msg = "step-up receipt sid mismatch — Core Pass likely not co-served in the Gate cookie jar (hsid diverged); real human may loop on the attested route"
		} else if reason == receiptExpired {
			msg = "step-up receipt expired (check Core↔Gate clock skew / NTP; the client can re-solve for a fresh receipt)"
		}
		c.sink.Emit(audit.Record{
			EventType: audit.EventTokenBindingMismatch, Actor: audit.Actor{Kind: "subject", IDPsn: c.pseudo(sid, sid)},
			TenantID: "control", RouteClass: "control", Verdict: string(VerdictDeny), Mode: "enforce",
			FailReason: msg, KeyID: "k1",
		})
		http.Error(w, "invalid or missing step-up receipt", http.StatusForbidden)
		return
	}
	tok := issueStepUpToken(c.tokenKey, sid, tokenBind(r), c.tokEpoch(), now.Add(30*time.Minute))
	http.SetCookie(w, &http.Cookie{Name: stepUpCookie, Value: tok, Path: "/", HttpOnly: true, Secure: true, SameSite: http.SameSiteLaxMode})
	c.sink.Emit(audit.Record{
		EventType: audit.EventTokenIssued, Actor: audit.Actor{Kind: "subject", IDPsn: c.pseudo(sid, sid)},
		TenantID: "control", RouteClass: "control", Verdict: "allow", Mode: "enforce", KeyID: "k1",
		FailReason: "step-up proof minted (Pass solved)",
	})
	writeJSON(w, map[string]any{"ok": true})
}

// handleLoader serves a minimal collector that beacons a client report to the
// control plane. Production serves the WASM bundle; this reference keeps a slim
// JS collector so the end-to-end path is exercised.
func (c *ControlPlane) handleLoader(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/javascript; charset=utf-8")
	w.Header().Set("Cache-Control", "no-store")
	_, _ = w.Write([]byte(loaderJS))
}

func (c *ControlPlane) ensureSession(w http.ResponseWriter, r *http.Request) string {
	if sid := sessionID(r); sid != "" {
		return sid
	}
	b := make([]byte, 16)
	if _, err := rand.Read(b); err != nil {
		panic("crypto/rand read failed: " + err.Error()) // fail closed (audit LOW-2)
	}
	sid := hex.EncodeToString(b)
	http.SetCookie(w, &http.Cookie{Name: sessionCookie, Value: sid, Path: "/", HttpOnly: true, Secure: true, SameSite: http.SameSiteLaxMode})
	return sid
}

func (c *ControlPlane) pseudo(sid, value string) string {
	if c.vault == nil || sid == "" {
		return ""
	}
	return c.vault.Pseudonymize(sid, value)
}

func topSignals(cs []signals.Contributor) []audit.Signal {
	out := make([]audit.Signal, 0, len(cs))
	for i, s := range cs {
		if i >= 5 {
			break
		}
		out = append(out, audit.Signal{ID: s.ID, Verdict: "BOT", Conf: 1})
	}
	return out
}

func headerInfo(r *http.Request) network.HeaderInfo {
	names := make([]string, 0, len(r.Header))
	for k := range r.Header {
		names = append(names, k)
	}
	xcache := r.Header.Get("X-Cache")
	if xcache == "" {
		xcache = r.Header.Get("X-Cache-Lookup")
	}
	return network.HeaderInfo{
		Method:                 r.Method,
		Version:                protoVer(r),
		IsH2:                   r.ProtoMajor == 2,
		Names:                  names,
		CasingReliable:         false,
		HasCookie:              r.Header.Get("Cookie") != "",
		HasReferer:             r.Header.Get("Referer") != "",
		AcceptLanguage:         r.Header.Get("Accept-Language"),
		AcceptEncoding:         r.Header.Get("Accept-Encoding"),
		UserAgent:              r.Header.Get("User-Agent"),
		Via:                    r.Header.Get("Via"),
		ProxyConnection:        r.Header.Get("Proxy-Connection"),
		XCache:                 xcache,
		XSquidError:            r.Header.Get("X-Squid-Error"),
		XForwardedFor:          r.Header.Get("X-Forwarded-For"),
		Forwarded:              r.Header.Get("Forwarded"),
		CFConnectingIP:         r.Header.Get("CF-Connecting-IP"),
		TrueClientIP:           r.Header.Get("True-Client-IP"),
		XClientIP:              r.Header.Get("X-Client-IP"),
		XOriginalForwardedFor:  r.Header.Get("X-Original-Forwarded-For"),
		XBlueCoatVia:           r.Header.Get("X-BlueCoat-Via"),
	}
}

func protoVer(r *http.Request) string {
	if r.ProtoMajor == 2 {
		return "20"
	}
	return "11"
}

func writeJSON(w http.ResponseWriter, v any) { httpx.WriteJSON(w, v) } // PLAN-07 R7

// loaderJS is the slim reference collector (posts a client report to /collect).
const loaderJS = `(function(){
  var base = document.currentScript.src.replace(/loader\.js.*$/, '');
  function report(){
    var rep = {
      userAgent: navigator.userAgent,
      uaClientHints: {},
      engineVersion: 'proxy-loader-1.0',
      signals: [],
      behavior: { mouse: { samples: 0 }, events: { totalEvents: 0 } },
      environment: { probed: true },
      advanced: { probed: true }
    };
    if (navigator.webdriver) rep.signals.push({id:'l1.navigator.webdriver',layer:'L1',value:true,verdict:'BOT',weight:40,score:40,confidence:0.95,collected:'js'});
    fetch(base+'collect', {method:'POST', headers:{'Content-Type':'application/json'}, credentials:'include', body: JSON.stringify(rep)});
  }
  if (document.readyState !== 'loading') report(); else document.addEventListener('DOMContentLoaded', report);
})();`

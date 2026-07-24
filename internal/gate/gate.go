package gate

import (
	"errors"
	"net/http"
	"net/http/httputil"
	"net/url"
	"strings"
	"sync/atomic"
	"time"

	"github.com/modootoday/humanymous/internal/abuse"
	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/correlation"
)

// errAttestedCatchAll is returned by NewServer when the attestation floor
// (ceiling-guard #1) is mapped onto a catch-all/root prefix — flooring an entire
// site's browse traffic to Pass is a misconfiguration, refused fail-closed at boot.
var errAttestedCatchAll = errors.New("gate: the \"attested\" preset (attestation floor) is refused on the catch-all/root prefix — mark specific high-value routes (e.g. /checkout), not the whole site")

// gate.go is the two-plane reverse proxy handler (SoT-19). It routes the
// reserved control-plane namespace locally, enforces the sticky verdict at the
// pre-upstream gate, forwards ALLOW traffic to the origin, and injects the
// detection bundle into HTML responses on the way back.

// Server is the humanymous proxy.
type Server struct {
	cfg             Config
	sink            *audit.Sink
	vault           *audit.Vault
	verdicts        VerdictLedger // distribution seam (PLAN-07 R18): in-mem now, shared later
	control         http.Handler  // /__hmn/* control plane (session/collect/pow/loader)
	rp              *httputil.ReverseProxy
	snippet         []byte
	originKey       []byte                // origin-cloaking HMAC key (SoT-23 §1)
	tokenKey        []byte                // verdict-token HMAC key (SoT-21 §3)
	tokenEpochs     *EpochManager         // rotating verdict-token epoch (SoT-28 WS6)
	sweep           *SweepDetector        // decision-probing recon detector (HR-30)
	decFloor        time.Duration         // constant decision-latency floor (HR-30)
	bans            BanLedger             // rate-limit-triggered + manual bans (SoT-27); distribution seam (R18)
	auth            *AdminAuth            // admin-plane authentication (SoT-28 WS1)
	approvals       *ApprovalStore        // two-phase dual-control (SoT-28 WS2)
	shreds          *ShredQueue           // erasure hold-window queue (SoT-28 WS3)
	incidentLimiter *abuse.Limiter        // per-operator incident-lookup enumeration cap (SoT-28 WS4)
	devConsoleToken string                // dev bearer injected into the served console (SoT-28 §1)
	killSwitch      atomic.Bool           // runtime global-monitor override (SoT-28 WS9 kill switch)
	agentKeys       KeyDirectory          // Web Bot Auth trusted-key directory (PLAN-08 R3); nil = feature off
	agentNonces     *NonceCache           // Web Bot Auth single-use signature nonces (anti-replay)
	patVerifier     *PATVerifier          // Privacy Pass PAT issuer keys (PLAN-08 R2); nil = feature off
	webauthn        *WebAuthnRegistry     // WebAuthn credential registry (PLAN-08 R2); nil = feature off
	anomaly         *anomalyShadow        // PLAN-08 R5 shadow anomaly observer (log-only); nil = off
	credCorr        *correlation.Registry // ceiling-guard #2: per-WebAuthn-credential /24 fan-out tracker
	credFanoutCap   int                   // ceiling-guard #2: distinct /24s per credential before its fast-path reverts to Pass
	nowFn           func() time.Time
}

// RunDueShreds executes any erasures whose hold window has elapsed and seals a
// signed erasure certificate for each (SoT-28 WS3). Call periodically.
func (s *Server) RunDueShreds(now time.Time) int {
	if s.shreds == nil {
		return 0
	}
	due := s.shreds.Due(now)
	for _, d := range due {
		existed := s.vault != nil && s.vault.Shred(d.Subject)
		s.emitErasureCertificate(d, existed)
	}
	return len(due)
}

// emitErasureCertificate seals a verifiable erasure certificate: the STH root at
// erasure time, key id, legal basis, and the two principals (SoT-28 §7). Because
// records store only pseudonyms, the certificate proves severance without PII.
func (s *Server) emitErasureCertificate(d ScheduledShred, existed bool) {
	root, size := "", uint64(0)
	if cps := s.sink.Log().Checkpoints(); len(cps) > 0 {
		root, size = cps[len(cps)-1].Root, cps[len(cps)-1].TreeSize
	}
	evt := audit.EventErasureCompleted
	if !existed {
		evt = "erasure.no_subject"
	}
	rootShort := root
	if len(rootShort) > 12 {
		rootShort = rootShort[:12]
	}
	s.sink.Emit(audit.Record{
		EventType: evt, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(d.Approver, d.Approver)},
		TenantID: s.cfg.NodeID, Mode: "enforce", KeyID: "k1",
		FailReason: "erasure-cert: sth_root=" + rootShort + " tree_size=" + itoaInt(int(size)) + " legal_basis=" + d.LegalBasis + " " + d.Requester + "->" + d.Approver,
	})
}

// monitorOn reports whether enforcement is globally suppressed (config or the
// runtime kill switch) — everything drops to monitor/shadow (SoT-28 WS9).
func (s *Server) monitorOn() bool { return s.cfg.GlobalMonitor || s.killSwitch.Load() }

// SetKillSwitch flips the runtime global-monitor override (dual-controlled at the
// admin layer). When on, every route scores + logs but enforces nothing.
func (s *Server) SetKillSwitch(on bool) { s.killSwitch.Store(on) }

// enforcing reports the EFFECTIVE enforcement for a route (route policy AND not
// globally monitored).
func (s *Server) enforcing(route routePolicy) bool { return route.enforce && !s.monitorOn() }

// banLedger returns the configured shared ban ledger, or a default in-memory
// BanStore with the standard thresholds (PLAN-08 R1).
func banLedger(cfg Config) BanLedger {
	if cfg.BanLedger != nil {
		return cfg.BanLedger
	}
	return NewBanStore(rlWindow(cfg), rlSoft(cfg), rlHard(cfg))
}

// GC sweeps the gate's in-memory detection state (verdicts, bans + strikes + rate
// windows, sweep-detector bindings, shadow-anomaly state). Call it periodically —
// without it these fingerprint/IP-keyed maps grow without bound under the exact bot
// churn the product defends against (PLAN-08 deployment-review ship-blocker). The
// core engine already does this every minute; this brings the gate to parity.
func (s *Server) GC(now time.Time) {
	if g, ok := s.verdicts.(interface{ GC(time.Time) }); ok {
		g.GC(now)
	}
	if g, ok := s.bans.(interface{ GC(time.Time) }); ok {
		g.GC(now)
	}
	if s.sweep != nil {
		s.sweep.GC(now)
	}
	if s.anomaly != nil {
		s.anomaly.gc(now)
	}
	if s.credCorr != nil {
		s.credCorr.GC(now)
	}
}

// Bans exposes the ban store for console management (SoT-26/27).
func (s *Server) Bans() BanLedger { return s.bans }

// Auth exposes the admin auth table so cmd/gate can seed operators (SoT-28).
func (s *Server) Auth() *AdminAuth { return s.auth }

// SetDevConsoleToken injects a dev bearer token into the served console so the
// SPA can call the authenticated admin API (dev only; prod uses SSO/mTLS login).
func (s *Server) SetDevConsoleToken(t string) { s.devConsoleToken = t }

// AdminHandler is the admin plane mounted on a SEPARATE authenticated listener
// (SoT-28 WS1). It answers only the reserved /__hmn/admin/* prefix; anything
// else 404s. It is NEVER mounted on the public proxied listener.
func (s *Server) AdminHandler() http.Handler {
	prefix := s.cfg.ControlPath + "admin/"
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !strings.HasPrefix(r.URL.Path, prefix) {
			http.NotFound(w, r)
			return
		}
		s.handleAdmin(w, r, strings.TrimPrefix(r.URL.Path, prefix))
	})
}

// NewServer builds the proxy. control handles the reserved namespace (with the
// prefix already stripped). upstream is the origin base URL.
func NewServer(cfg Config, sink *audit.Sink, vault *audit.Vault, verdicts VerdictLedger, control http.Handler) (*Server, error) {
	if cfg.ControlPath == "" {
		cfg.ControlPath = "/__hmn/"
	}
	target, err := url.Parse(cfg.Upstream)
	if err != nil {
		return nil, err
	}
	rp := httputil.NewSingleHostReverseProxy(target)
	s := &Server{
		cfg:             cfg,
		sink:            sink,
		vault:           vault,
		verdicts:        verdicts,
		control:         control,
		rp:              rp,
		snippet:         []byte(injectMarker + `<script src="` + cfg.ControlPath + `loader.js" defer></script>`),
		originKey:       cfg.OriginKey,
		tokenKey:        cfg.TokenKey,
		tokenEpochs:     orDefaultEpochs(cfg.TokenEpochs),
		sweep:           NewSweepDetector(30*time.Second, 8), // >8 sessions/fp/30s = recon
		decFloor:        2 * time.Millisecond,                // HR-30 timing-oracle floor (SoT-28 WS2 breakout)
		bans:            banLedger(cfg),                      // in-memory, or a shared Redis ledger (PLAN-08 R1)
		auth:            NewAdminAuth(),
		approvals:       NewApprovalStore(cfg.TokenKey, 10*time.Minute),
		shreds:          NewShredQueue(erasureHold(cfg)),
		incidentLimiter: abuse.NewLimiter(time.Minute, 60, 120), // >120 incident lookups/min = trawl
		agentKeys:       cfg.AgentKeys,                          // Web Bot Auth directory (PLAN-08 R3), nil = off
		agentNonces:     NewNonceCache(time.Hour),               // Web Bot Auth anti-replay nonces (≥ signature max lifetime)
		patVerifier:     cfg.PATIssuers,                         // Privacy Pass PAT issuers (PLAN-08 R2), nil = off
		webauthn:        cfg.WebAuthnCreds,                      // WebAuthn credential registry (PLAN-08 R2), nil = off
		nowFn:           time.Now,
	}
	// Ceiling-guard #1 defense-in-depth: refuse the attestation floor on a catch-all
	// (root/empty) prefix at CONSTRUCTION, not only in the CLI routes loader. A programmatic
	// or alternate Config.Routes population would otherwise bypass cmd/gate/routes.go and
	// Pass-wall an entire site — the exact misconfiguration the ceiling-guard meeting rules
	// out. Fail closed so the operator sees it at boot, never in production.
	for prefix, preset := range cfg.Routes {
		if presetByName(preset).attestFloor && (normalizeMatchPath(prefix) == "/" || prefix == "") {
			return nil, errAttestedCatchAll
		}
	}
	// Ceiling-guard #2: bound a single WebAuthn credential's /24 fan-out so a stolen/shared
	// passkey fronting a proxy farm reverts to paying Pass friction past N subnets. Only
	// armed when WebAuthn is configured (PAT keyids are unlinkable-by-design and agent keys
	// are SUPPOSED to fan out — both are excluded, per the ceiling-guard meeting).
	if cfg.WebAuthnCreds != nil {
		s.credCorr = correlation.New(time.Hour)
		s.credFanoutCap = cfg.CredFanoutCap
		if s.credFanoutCap <= 0 {
			s.credFanoutCap = DefaultCredFanoutCap
		}
	}
	if cfg.AnomalyShadow {
		s.anomaly = newAnomalyShadow() // PLAN-08 R5: log-only shadow observer, off by default
	}
	// Response hook: inject into HTML (identity upstream is forced in Rewrite).
	rp.ModifyResponse = s.modifyResponse
	// Upstream-error hook (PLAN-07 R16): the default handler logs to ErrorLog and
	// writes a bare 502 with NO audit trail — an origin outage was invisible in the
	// tamper-evident log. Emit a record (correlation-linked, latency-stamped, with the
	// upstream status + error class) and THEN reproduce the default response exactly:
	// StatusBadGateway with an empty body, so clients see no change.
	rp.ErrorHandler = s.handleUpstreamError
	// Use the modern Rewrite hook (Go 1.20+) instead of Director. A Director-based
	// proxy ALSO auto-appends the peer IP to X-Forwarded-For, which doubled the value
	// ("ip, ip") next to the one we set. Rewrite + SetXForwarded writes a SINGLE
	// authoritative X-Forwarded-For / -Host / -Proto derived from the inbound socket;
	// a client cannot forge them (inbound trust headers are stripped, and the spoof
	// gate blocks forgeries before we ever forward). Setting Rewrite requires clearing
	// the Director that NewSingleHostReverseProxy installed.
	rp.Director = nil
	rp.Rewrite = func(pr *httputil.ProxyRequest) {
		pr.SetURL(target)                                // route to upstream (scheme/host + path join)
		pr.Out.Host = pr.In.Host                         // preserve the client Host (origin reads X-Forwarded-Host)
		stripInbound(pr.Out)                             // drop any client-forged internal/trust headers
		pr.SetXForwarded()                               // authoritative X-Forwarded-For (socket IP) + Host + Proto
		pr.Out.Header.Set("Accept-Encoding", "identity") // SoT-20 §1: no upstream recompress
		// Origin cloaking (SoT-23 §1, HR-24): the origin accepts ONLY proxied traffic
		// carrying this rotating token; a direct hit to the origin lacks it.
		pr.Out.Header.Set("X-Hmny-Origin-Auth", originAuth(s.originKey, originEpoch(time.Now())))
	}
	return s, nil
}

// ServeHTTP is the request entry point.
func (s *Server) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	// Stamp per-request audit metadata (PLAN-07 R15) before any gate runs so every
	// record this request emits — verdict, upstream error, egress — shares one
	// correlation id and can be latency-stamped from a single start.
	r = r.WithContext(withReqMeta(r.Context(), reqMeta{corr: newCorrelationID(), start: s.nowFn()}))
	if s.anomaly != nil { // PLAN-08 R5: observe only — never short-circuits, never changes the verdict
		s.anomaly.observe(bindKey(r), s.nowFn())
	}
	sid := sessionID(r)

	// The edge request path is a sequence of security-domain gates (see
	// gates.go). Each returns true when it has handled (short-circuited)
	// the request. Order is load-bearing: control-plane split, then ban, framing,
	// header hygiene, verdict-token fast-path, upgrade, sweep, then the verdict
	// gate. The admin plane is never on this public listener (SoT-28 WS1).
	if s.routeControlPlane(w, r) { // SoT-19 §3
		return
	}
	if s.banGate(w, r, sid) { // SoT-27, HR-21
		return
	}
	if s.smuggleGate(w, r, sid) { // SoT-23 §3, HR-23
		return
	}
	if s.spoofHeaderGate(w, r, sid) { // SoT-23 §4, HR-27b
		return
	}

	route := s.cfg.resolve(r.URL.Path)
	if s.agentAuthGate(w, r, sid, route) { // PLAN-08 R3: Web Bot Auth (trust-upgrade or forgery-deny)
		return
	}
	if s.patGate(w, r, sid, route) { // PLAN-08 R2: Privacy Pass PAT (trust-upgrade on a valid token)
		return
	}
	if s.webauthnGate(w, r, sid, route) { // PLAN-08 R2: WebAuthn possession (trust-upgrade)
		return
	}
	if s.verdictTokenGate(w, r, sid, route) { // SoT-21 §3, HR-28 (deny or trusted fast-path)
		return
	}
	if s.upgradeGate(w, r, sid, route) { // SoT-21 §5, HR-26
		return
	}
	now := s.nowFn()
	if s.sweepGate(w, r, sid, route, now) { // SoT-21 §8, HR-30
		return
	}
	s.applyVerdict(w, r, sid, route, now) // SoT-19 step 6, SoT-21 §1
}

// maxInjectBody caps the declared HTML size we will scan for an injection point
// (HR-27a). A larger body is passed through un-injected rather than buffered.
const maxInjectBody = 8 << 20 // 8 MiB

// auditInjectSkip records that injection was safely skipped (SoT-20 §8).
func (s *Server) auditInjectSkip(resp *http.Response, reason string) {
	s.sink.Emit(audit.Record{
		EventType:  audit.EventInjectSkipped,
		Actor:      audit.Actor{Kind: "system"},
		TenantID:   s.cfg.NodeID,
		RouteClass: "html",
		Rules:      []string{"HR-27a"},
		FailReason: reason,
		Mode:       "enforce",
		KeyID:      "k1",
	})
}

// hasValidToken reports whether the request carries a verdict token that
// verifies and binds to this client's fingerprint (SoT-21 §3).
func (s *Server) hasValidToken(r *http.Request) bool {
	if len(s.tokenKey) == 0 {
		return false
	}
	c, err := r.Cookie(verdictCookie)
	if err != nil || c.Value == "" {
		return false
	}
	return verifyVerdictToken(s.tokenKey, c.Value, tokenBind(r), sessionID(r), s.nowFn(), s.tokenEpochs.Accepted()...) == tokenOK
}

// hasValidStepUp reports whether the request carries a step-up proof (hmn_su) that
// verifies and binds to this client's fingerprint+subnet + session (ceiling-guard
// #1). This is the token minted on an LLM-resistant Pass solve; possessing it is
// what lets a scoring-ALLOW take the fast-path on an attestFloor route.
func (s *Server) hasValidStepUp(r *http.Request, sid string) bool {
	if len(s.tokenKey) == 0 {
		return false
	}
	c, err := r.Cookie(stepUpCookie)
	if err != nil || c.Value == "" {
		return false
	}
	return verifyStepUpToken(s.tokenKey, c.Value, tokenBind(r), sid, s.nowFn(), s.tokenEpochs.Accepted()...) == tokenOK
}

// handleUpstreamError is the ReverseProxy ErrorHandler (PLAN-07 R16). It audits an
// origin transport failure (unreachable/timeout/reset) then reproduces the default
// proxy response byte-for-byte: a 502 with an empty body. The request passed here is
// the outbound clone, whose context carries the inbound reqMeta + route.
func (s *Server) handleUpstreamError(w http.ResponseWriter, r *http.Request, err error) {
	meta := reqMetaFrom(r.Context())
	s.sink.Emit(audit.Record{
		EventType:   audit.EventUpstreamError,
		Actor:       audit.Actor{Kind: "system"},
		TenantID:    s.cfg.NodeID,
		Host:        r.Host,
		RouteClass:  routeClass(r),
		Correlation: meta.corr,
		LatencyUS:   meta.latencyUS(s.nowFn()),
		Mode:        modeName(routeFrom(r.Context())),
		FailMode:    "degraded",
		FailReason:  "upstream " + upstreamErrorClass(err) + ": " + err.Error(),
		Upstream:    &audit.Upstream{Status: http.StatusBadGateway, ErrorClass: upstreamErrorClass(err)},
		KeyID:       "k1",
	})
	w.WriteHeader(http.StatusBadGateway) // identical to httputil's default (empty body)
}

// forward proxies to the upstream origin (ALLOW path, SoT-19 step 8-9).
func (s *Server) forward(w http.ResponseWriter, r *http.Request, route routePolicy) {
	r = r.WithContext(withRoute(r.Context(), route))
	s.rp.ServeHTTP(w, r)
}

// modifyResponse injects the bundle into HTML responses (SoT-19 step 9). It is
// add-only and drops Content-Length so the framing stays correct (SoT-20 §3).
func (s *Server) modifyResponse(resp *http.Response) error {
	route := routeFrom(resp.Request.Context())
	if !route.inject {
		return nil
	}
	ct := resp.Header.Get("Content-Type")
	if resp.StatusCode != http.StatusOK || !strings.HasPrefix(strings.ToLower(ct), "text/html") {
		return nil
	}
	// HR-27a injector abuse: a compressed body despite our identity request could
	// be a decompression bomb — skip injection (safe pass-through), never expand.
	if resp.Header.Get("Content-Encoding") != "" {
		s.auditInjectSkip(resp, "decomp_bomb_guard")
		return nil
	}
	// HR-27a injector abuse: an oversized HTML body (5GB "no closing head" DoS)
	// must not be scanned/buffered — skip injection above a hard cap.
	if resp.ContentLength > maxInjectBody {
		s.auditInjectSkip(resp, "too_large")
		return nil
	}
	var res injectResult
	resp.Body = readCloser{newInjectingReader(resp.Body, s.snippet, &res), resp.Body}
	resp.Header.Del("Content-Length") // length changes; force chunked
	resp.ContentLength = -1
	// egress scrub: verdict/nonce-bearing HTML must not be cached (SoT-19 §11).
	resp.Header.Set("Cache-Control", "no-store")
	// Merge (add-only) our CSP connect-src to the control plane (SoT-20 §6). We
	// only ADD; if origin has a CSP we leave it and add a report-only companion.
	if resp.Header.Get("Content-Security-Policy") == "" {
		resp.Header.Set("Content-Security-Policy-Report-Only",
			"connect-src 'self'; report-uri "+s.cfg.ControlPath+"csp-report")
	}
	return nil
}

// enforceBan drops a banned request at the edge with a Retry-After for temporary
// bans, and audits ban.enforced (SoT-27 §3). The scorer/origin are never touched.
func (s *Server) enforceBan(w http.ResponseWriter, r *http.Request, sid string, b BanEntry) {
	s.sink.EmitAndAct(audit.Record{
		EventType: audit.EventBanEnforced, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, clientIP(r))},
		TenantID: s.cfg.NodeID, RouteClass: routeClass(r), Verdict: string(VerdictDeny),
		Rules: []string{"HR-21"}, Action: "block", Mode: "enforce", FailReason: banReason(b), KeyID: "k1",
	}, func() {
		w.Header().Set("X-Hmn-Incident", "see-audit-log")
		if !b.Permanent() {
			if secs := int(b.remainingSecs(s.nowFn())); secs > 0 {
				w.Header().Set("Retry-After", itoaInt(secs))
			}
		}
		http.Error(w, "Source temporarily blocked (rate limit / abuse protection).", http.StatusForbidden)
	})
}

// banReason renders a compact ban descriptor for the audit record.
func banReason(b BanEntry) string {
	dur := "permanent"
	if !b.Permanent() {
		dur = "strike " + itoaInt(b.Strike)
	}
	return b.Source + " ban (" + dur + "): " + b.Reason
}

func itoaInt(v int) string {
	if v == 0 {
		return "0"
	}
	neg := v < 0
	if neg {
		v = -v
	}
	var b [12]byte
	i := len(b)
	for v > 0 {
		i--
		b[i] = byte('0' + v%10)
		v /= 10
	}
	if neg {
		i--
		b[i] = '-'
	}
	return string(b[i:])
}

// deny blocks at the edge; origin was never contacted (SoT-21 §1).
func (s *Server) deny(w http.ResponseWriter) {
	w.Header().Set("X-Hmn-Incident", "see-audit-log")
	http.Error(w, "Request blocked by humanymous (automation detected).", http.StatusForbidden)
}

// challenge serves the accessible interstitial (SoT-21 §6). The reference serves
// a minimal page pointing at the control-plane PoW; production self-hosts WCAG UI.
func (s *Server) challenge(w http.ResponseWriter) {
	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	w.Header().Set("Cache-Control", "no-store")
	w.WriteHeader(http.StatusUnauthorized)
	_, _ = w.Write([]byte(`<!doctype html><html lang="en"><head><meta charset="utf-8">` +
		`<title>Verification required</title></head><body>` +
		`<h1>Verification required</h1><p>Please complete a quick check to continue.</p>` +
		`<script src="` + s.cfg.ControlPath + `loader.js" defer></script></body></html>`))
}

// pseudonym applies per-subject linkage-key pseudonymization (SoT-18 §5). The
// subject key is the session id, so a session's events are linkable but the raw
// IP is never stored and can be crypto-shredded.
func (s *Server) pseudonym(sid, value string) string {
	if s.vault == nil || sid == "" {
		return ""
	}
	return s.vault.Pseudonymize(sid, value)
}

// readCloser adapts a reader + the original closer.
type readCloser struct {
	r interface{ Read([]byte) (int, error) }
	c interface{ Close() error }
}

func (rc readCloser) Read(p []byte) (int, error) { return rc.r.Read(p) }
func (rc readCloser) Close() error               { return rc.c.Close() }

func modeName(route routePolicy) string {
	if route.enforce {
		return "enforce"
	}
	return "monitor"
}

func ruleList(rule string) []string {
	if rule == "" {
		return []string{}
	}
	return []string{rule}
}

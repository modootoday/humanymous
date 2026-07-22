package gate

import (
	"net/http"
	"strings"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
)

// gates.go decomposes the edge request path into one method per SECURITY
// DOMAIN (SoT-19..28). ServeHTTP (gate.go) is then a readable sequence of
// gates; each gate here owns a single concern and returns true when it has
// handled (short-circuited) the request. Order is load-bearing and preserved
// exactly as the original inline chain.

// routeControlPlane branches the control-plane trust domain (session/collect/pow/
// loader) off the proxied path (SoT-19 §3); the admin plane is never on the
// public listener (404). Returns true when the request was routed here.
func (s *Server) routeControlPlane(w http.ResponseWriter, r *http.Request) bool {
	if !strings.HasPrefix(r.URL.Path, s.cfg.ControlPath) {
		return false
	}
	if strings.HasPrefix(r.URL.Path, s.cfg.ControlPath+"admin/") {
		http.NotFound(w, r) // admin is off the public plane
		return true
	}
	http.StripPrefix(strings.TrimSuffix(s.cfg.ControlPath, "/"), s.control).ServeHTTP(w, r)
	return true
}

// banGate drops IP-or-fingerprint-banned traffic before the scorer or an upstream
// connection is touched, and auto-bans on a hard rate breach (SoT-27 §1/§3, HR-21).
// Metering BOTH keys catches an IP-rotating flood by fingerprint.
func (s *Server) banGate(w http.ResponseWriter, r *http.Request, sid string) bool {
	if s.bans == nil {
		return false
	}
	keys := []string{"ip:" + clientIP(r)}
	if fp := bindKey(r); fp != "" {
		keys = append(keys, "fp:"+fp)
	}
	for _, k := range keys {
		if b, banned := s.bans.Check(k); banned {
			s.enforceBan(w, r, sid, b)
			return true
		}
	}
	for _, k := range keys {
		entry, banned, level := s.bans.Observe(k)
		if banned {
			s.sink.Emit(audit.Record{
				EventType: audit.EventBanApplied, Actor: audit.Actor{Kind: "system"},
				TenantID: s.cfg.NodeID, RouteClass: routeClass(r), Verdict: string(VerdictDeny),
				Rules: []string{"HR-21"}, Action: "block", Mode: "enforce",
				FailReason: banReason(entry), KeyID: "k1",
			})
			s.enforceBan(w, r, sid, entry)
			return true
		}
		if level == 1 {
			s.sink.Emit(audit.Record{
				EventType: audit.EventRateSoftExceeded, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, clientIP(r))},
				TenantID: s.cfg.NodeID, RouteClass: routeClass(r),
				Action: "rate_limit", Mode: "enforce", KeyID: "k1",
			})
		}
	}
	return false
}

// smuggleGate rejects request-smuggling primitives (CL+TE, dup CL, TE!=chunked,
// obs-fold) before any routing/parse decision (SoT-23 §3, HR-23).
func (s *Server) smuggleGate(w http.ResponseWriter, r *http.Request, sid string) bool {
	reason := smuggleScan(r)
	if reason == smuggleNone {
		return false
	}
	rec := smuggleRecord(s.cfg.NodeID, s.pseudonym(sid, sid), reason)
	s.sink.EmitAndAct(rec, func() { s.deny(w) })
	return true
}

// spoofHeaderGate strips + blocks a client that sends our internal/forwarding
// headers to forge its source or impersonate the proxy (SoT-23 §4, HR-27b).
func (s *Server) spoofHeaderGate(w http.ResponseWriter, r *http.Request, sid string) bool {
	found := spoofScan(r)
	if len(found) == 0 {
		return false
	}
	rec := spoofRecord(s.cfg.NodeID, s.pseudonym(sid, sid), found)
	stripInbound(r)
	s.sink.EmitAndAct(rec, func() { s.deny(w) })
	return true
}

// verdictTokenGate verifies a fingerprint-bound verdict trust token (SoT-21 §3,
// HR-28): an invalid/lifted token is denied; a valid ALLOW token on an enforcing
// route takes the trusted fast-path (forward). Returns true when handled; false
// to continue to full scoring.
func (s *Server) verdictTokenGate(w http.ResponseWriter, r *http.Request, sid string, route routePolicy) bool {
	if len(s.tokenKey) == 0 {
		return false
	}
	c, err := r.Cookie(verdictCookie)
	if err != nil || c.Value == "" {
		return false
	}
	reason := verifyVerdictToken(s.tokenKey, c.Value, tokenBind(r), sid, s.nowFn(), s.tokenEpochs.Accepted()...)
	if reason != tokenOK {
		rec := audit.Record{
			EventType: audit.EventTokenBindingMismatch,
			Actor:     audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, sid)},
			TenantID:  s.cfg.NodeID, RouteClass: routeClass(r), Verdict: string(VerdictDeny),
			Rules: []string{"HR-28"}, Action: "block", Mode: "enforce",
			FailReason: "verdict token " + string(reason), KeyID: "k1",
		}
		s.sink.EmitAndAct(rec, func() { s.deny(w) })
		return true
	}
	if s.enforcing(route) {
		rec := audit.Record{
			EventType: audit.EventEnfAllow, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, sid)},
			TenantID: s.cfg.NodeID, RouteClass: routeClass(r), Verdict: string(VerdictAllow),
			Action: "pass", Mode: "enforce", KeyID: "k1", ConfigVer: route.name,
		}
		s.sink.EmitAndAct(rec, func() { s.forward(w, r, route) })
		return true
	}
	return false
}

// upgradeGate requires a valid fingerprint-bound verdict token before a WS/SSE 101
// completes, so automation cannot tunnel past L1–L4 collection (SoT-21 §5, HR-26).
func (s *Server) upgradeGate(w http.ResponseWriter, r *http.Request, sid string, route routePolicy) bool {
	if routeClass(r) != "upgrade" || !s.enforcing(route) || s.hasValidToken(r) {
		return false
	}
	rec := audit.Record{
		EventType: audit.EventUpgradeNoVerdict, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, sid)},
		TenantID: s.cfg.NodeID, RouteClass: "upgrade", Verdict: string(VerdictDeny),
		Rules: []string{"HR-26"}, Action: "block", Mode: "enforce",
		FailReason: "upgrade with no prior fingerprint-bound verdict", KeyID: "k1",
	}
	s.sink.EmitAndAct(rec, func() { s.deny(w) })
	return true
}

// sweepGate flags decision-probing recon: one fingerprint spinning up many
// near-identical sessions (SoT-21 §8, HR-30).
func (s *Server) sweepGate(w http.ResponseWriter, r *http.Request, sid string, route routePolicy, now time.Time) bool {
	if s.sweep == nil || !s.enforcing(route) || !s.sweep.Observe(bindKey(r), sid, now) {
		return false
	}
	rec := audit.Record{
		EventType: audit.EventReconProbing, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, sid)},
		TenantID: s.cfg.NodeID, RouteClass: routeClass(r), Verdict: string(VerdictDeny),
		Rules: []string{"HR-30"}, Action: "block", Mode: "enforce",
		FailReason: "decision-probing sweep (many sessions, one fingerprint)", KeyID: "k1",
	}
	s.sink.EmitAndAct(rec, func() { s.deny(w) })
	return true
}

// applyVerdict is the pre-upstream verdict gate (SoT-19 step 6, SoT-21 §1): it
// resolves the sticky verdict → action (pass / challenge / block), fails closed on
// unknown-verdict mutations (SoT-28 WS5), honors monitor/kill-switch pass-through,
// and applies a constant-latency floor so deny/challenge are timing-indistinct.
func (s *Server) applyVerdict(w http.ResponseWriter, r *http.Request, sid string, route routePolicy, now time.Time) {
	sticky := s.verdicts.Get(sid, now)
	verdict := sticky.verdict

	unsafe := isUnsafeMethod(r.Method)
	eventType, actionName := verdict.action(route, unsafe)
	if verdict == VerdictUnknown && unsafe && actionName == "challenge_pow" {
		eventType = "enf.failclosed.mutating"
	}
	rec := audit.Record{
		EventType:  eventType,
		Actor:      audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, clientIP(r))},
		TenantID:   s.cfg.NodeID,
		SessionPsn: s.pseudonym(sid, sid),
		Host:       r.Host,
		RouteClass: routeClass(r),
		Verdict:    string(verdict),
		RiskScore:  int(sticky.risk),
		Rules:      ruleList(sticky.rule),
		TopSignals: sticky.top,
		Action:     actionName,
		Mode:       modeName(route),
		KeyID:      "k1",
		ConfigVer:  s.configVersion(),
	}
	// Correlation + decision latency (PLAN-07 R15): tie this verdict to every other
	// record the request emits, and stamp how long reaching the verdict took.
	meta := reqMetaFrom(r.Context())
	rec.Correlation = meta.corr
	rec.LatencyUS = meta.latencyUS(s.nowFn())

	if !s.enforcing(route) { // monitor/shadow or kill switch: log, pass through
		rec.Mode = "monitor"
		s.sink.Emit(rec)
		s.forward(w, r, route)
		return
	}

	switch actionName {
	case "block":
		constantFloor(now, s.decFloor, s.nowFn)
		s.sink.EmitAndAct(rec, func() { s.deny(w) })
	case "challenge_pow":
		constantFloor(now, s.decFloor, s.nowFn)
		s.sink.EmitAndAct(rec, func() { s.challenge(w) })
	default: // pass
		s.sink.EmitAndAct(rec, func() { s.forward(w, r, route) })
	}
}

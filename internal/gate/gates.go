package gate

import (
	"net/http"
	"strings"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/gate/settings"
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
	mode := s.gateModuleMode("gate.ban")
	if mode == settings.ModeOff {
		return false
	}
	keys := []string{"ip:" + clientIP(r)}
	if fp := bindKey(r); fp != "" {
		keys = append(keys, "fp:"+fp)
	}
	for _, k := range keys {
		if b, banned := s.bans.Check(k); banned {
			if mode != settings.ModeEnforce {
				s.sink.Emit(audit.Record{
					EventType: audit.EventBanEnforced, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, clientIP(r))},
					TenantID: s.cfg.NodeID, RouteClass: routeClass(r), Verdict: string(VerdictDeny),
					Rules: []string{"HR-21"}, Action: "would_block", Mode: gateModeName(mode),
					FailReason: banReason(b) + " (audit only)", KeyID: "k1",
				})
				continue
			}
			s.enforceBan(w, r, sid, b)
			return true
		}
	}
	for _, k := range keys {
		if mode != settings.ModeEnforce {
			monitor, ok := s.bans.(RateMonitorBanLedger)
			if !ok {
				continue
			}
			level := monitor.ObserveRate(k)
			if level > 0 {
				eventType := audit.EventRateSoftExceeded
				action := "would_rate_limit"
				if level == 2 {
					eventType = audit.EventRateHardExceeded
					action = "would_auto_ban"
				}
				s.sink.Emit(audit.Record{
					EventType: eventType, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, clientIP(r))},
					TenantID: s.cfg.NodeID, RouteClass: routeClass(r),
					Action: action, Mode: gateModeName(mode), KeyID: "k1",
				})
			}
			continue
		}
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
// SoT-39: when gate.smuggle is monitor/shadow, audit only (no deny).
func (s *Server) smuggleGate(w http.ResponseWriter, r *http.Request, sid string) bool {
	reason := smuggleScan(r)
	if reason == smuggleNone {
		return false
	}
	rec := smuggleRecord(s.cfg.NodeID, s.pseudonym(sid, sid), reason)
	if s.gateModuleMode("gate.smuggle") != settings.ModeEnforce {
		rec.Mode = "monitor"
		s.sink.Emit(rec)
		return false
	}
	s.sink.EmitAndAct(rec, func() { s.deny(w) })
	return true
}

// spoofHeaderGate strips + blocks a client that sends our internal/forwarding
// headers to forge its source or impersonate the proxy (SoT-23 §4, HR-27b).
// SoT-39: monitor/shadow demotion audits + still strips, but does not deny.
func (s *Server) spoofHeaderGate(w http.ResponseWriter, r *http.Request, sid string) bool {
	found := spoofScan(r)
	if len(found) == 0 {
		return false
	}
	rec := spoofRecord(s.cfg.NodeID, s.pseudonym(sid, sid), found)
	stripInbound(r)
	if s.gateModuleMode("gate.spoof_header") != settings.ModeEnforce {
		rec.Mode = "monitor"
		s.sink.Emit(rec)
		return false
	}
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
	mode := s.gateModuleMode("gate.verdict_token")
	if mode == settings.ModeOff {
		return false
	}
	c, err := r.Cookie(verdictCookie)
	if err != nil || c.Value == "" {
		return false
	}
	reason := verifyVerdictToken(s.tokenKey, c.Value, tokenBind(r), sid, s.nowFn(), s.tokenEpochs.Accepted()...)
	if reason != tokenOK {
		if mode != settings.ModeEnforce {
			s.sink.Emit(audit.Record{
				EventType: audit.EventTokenBindingMismatch,
				Actor:     audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, sid)},
				TenantID:  s.cfg.NodeID, RouteClass: routeClass(r), Mode: gateModeName(mode),
				Action: "would_clear_and_rescore", FailReason: "verdict token " + string(reason) + " (audit only)", KeyID: "k1",
			})
			return false
		}
		// A token that does not verify (stale key after a restart, wrong epoch, expired,
		// a fingerprint that legitimately changed, or an actual forgery) must NOT be a
		// terminal deny — that 403s a legitimate returning human whose token was minted
		// under a previous key/node, which is the DEFAULT single-node restart case and any
		// non-sticky fleet node (deep-review ship-blocker; violates the no-lockout / low-FPR
		// hard constraint). Clear the stale cookie and fall through to full scoring, which
		// re-mints a fresh token on ALLOW. A real bot is caught on its own fingerprint by
		// scoring; a stolen token confers no fast-path benefit. Audited, but not a deny.
		http.SetCookie(w, &http.Cookie{Name: verdictCookie, Value: "", Path: "/", MaxAge: -1, HttpOnly: true, Secure: true, SameSite: http.SameSiteLaxMode})
		s.sink.Emit(audit.Record{
			EventType: audit.EventTokenBindingMismatch,
			Actor:     audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, sid)},
			TenantID:  s.cfg.NodeID, RouteClass: routeClass(r), Mode: "enforce",
			FailReason: "verdict token " + string(reason) + " — cleared, re-scoring", KeyID: "k1",
		})
		return false
	}
	if mode != settings.ModeEnforce {
		s.sink.Emit(audit.Record{
			EventType: audit.EventEnfAllow, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, sid)},
			TenantID: s.cfg.NodeID, RouteClass: routeClass(r), Verdict: string(VerdictAllow),
			Action: "would_fast_path", Mode: gateModeName(mode), KeyID: "k1",
		})
		return false
	}
	if s.enforcing(route) {
		// Ceiling-guard #1: on an attestFloor route a VALID verdict (ALLOW) token is NOT
		// enough for the fast-path — the client must also hold a step-up proof (hmn_su). A
		// credential-bearing request never reaches here (the possession pre-gates forwarded
		// it already), so reaching this point on an attestFloor route means neither
		// possession nor a Pass solve. Decline the fast-path and fall through; applyVerdict
		// demotes the scoring-ALLOW to a step-up challenge. This closes the laundering hole
		// where a prior plain ALLOW token would otherwise walk past the floor.
		if route.attestFloor && !s.hasValidStepUp(r, sid) {
			return false
		}
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

// agentAuthGate verifies a Web Bot Auth request signature (PLAN-08 R3). A valid
// signature from an operator-allowlisted key is a trust-upgrade (forward without a
// behavioral fight); a signature that claims an allowlisted key but fails to verify
// is a forgery (deny); an unknown/unsupported key is neutral (fall through to the
// normal pipeline). No signature at all is a no-op.
func (s *Server) agentAuthGate(w http.ResponseWriter, r *http.Request, sid string, route routePolicy) bool {
	if s.agentKeys == nil {
		return false
	}
	v, keyid := verifyAgent(r.Host, r.Method, r.URL.Path, r.Header.Get("Signature-Input"), r.Header.Get("Signature"), s.agentKeys, s.agentNonces, s.nowFn())
	switch v {
	case agentForged:
		if !s.enforcing(route) {
			// Monitor/shadow mode enforces NOTHING (SoT invariant): log the forgery but
			// do not deny — otherwise a shadow deployment would silently block (deep-review).
			s.sink.Emit(audit.Record{
				EventType: audit.EventAgentForged, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, sid)},
				TenantID: s.cfg.NodeID, RouteClass: routeClass(r), Mode: "monitor",
				FailReason: "forged Web Bot Auth signature for allowlisted keyid " + keyid + " (monitor: not blocked)", KeyID: "k1",
			})
			return false
		}
		rec := audit.Record{
			EventType: audit.EventAgentForged, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, sid)},
			TenantID: s.cfg.NodeID, RouteClass: routeClass(r), Verdict: string(VerdictDeny),
			Rules: []string{"web-bot-auth"}, Action: "block", Mode: "enforce",
			FailReason: "forged Web Bot Auth signature for allowlisted keyid " + keyid, KeyID: "k1",
		}
		s.sink.EmitAndAct(rec, func() { s.deny(w) })
		return true
	case agentVerifiedTrusted:
		if !s.enforcing(route) {
			return false // monitor mode: log-only below, no fast-path
		}
		return s.trustUpgrade(w, r, sid, route, audit.EventAgentVerified, "web-bot-auth", "verified Web Bot Auth agent "+keyid)
	case agentVerifiedUnknown:
		s.sink.Emit(audit.Record{
			EventType: audit.EventAgentUnknown, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, sid)},
			TenantID: s.cfg.NodeID, RouteClass: routeClass(r), Mode: "enforce",
			FailReason: "unknown/unsupported agent signature keyid " + keyid, KeyID: "k1",
		})
		return false // neutral: continue the normal detection pipeline
	default:
		return false
	}
}

// patGate trust-upgrades a request carrying a valid Privacy Pass Private Access
// Token from a trusted issuer (PLAN-08 R2) — the redeemer proved humanity/attestation
// to the issuer, so the edge forwards without a behavioral fight. A missing or invalid
// token is a NO-OP (PAT is opt-in proof, never an accusation): the request falls
// through to the normal detection pipeline.
func (s *Server) patGate(w http.ResponseWriter, r *http.Request, sid string, route routePolicy) bool {
	if s.patVerifier == nil || !s.enforcing(route) {
		return false
	}
	v, keyid := s.patVerifier.verifyPrivateToken(r.Header.Get("Authorization"))
	if v != patVerified {
		return false // no valid token → continue the normal pipeline
	}
	return s.trustUpgrade(w, r, sid, route, audit.EventPATVerified, "privacy-pass", "verified Private Access Token, issuer "+keyid[:12])
}

// webauthnGate trust-upgrades a request carrying a valid, fresh WebAuthn assertion
// from a registered credential (PLAN-08 R2) — proof of possession by a returning user.
// A missing/invalid/replayed assertion is a NO-OP (never a deny): the request falls
// through to the normal detection pipeline.
func (s *Server) webauthnGate(w http.ResponseWriter, r *http.Request, sid string, route routePolicy) bool {
	if s.webauthn == nil || !s.enforcing(route) {
		return false
	}
	v, credID := s.webauthn.verify(r.Header.Get("Webauthn-Assertion"))
	if v != webauthnVerified {
		return false
	}
	short := credID
	if len(short) > 16 {
		short = short[:16]
	}
	// Ceiling-guard #2: bound this credential's /24 fan-out. A genuine possession proof
	// still verified, but if ONE credential is now fast-pathing from more than the cap of
	// distinct subnets within the window, it is a stolen/shared passkey fronting a proxy
	// farm — stop honoring its trust-upgrade and let it fall through to the normal
	// pipeline, where an attestFloor route routes it to Pass. Degrades to CHALLENGE (Pass),
	// never DENY: a roaming human on one passkey across a few networks just solves a Pass.
	// WebAuthn-only by construction (s.credCorr is nil for PAT/agent — see NewServer).
	if s.credCorr != nil {
		// Count fan-out on EVERY route (so a farm cannot launder its subnet count by warming
		// up on unmarked routes)…
		s.credCorr.Observe("cred:"+credID, clientSubnet(r), sid, s.nowFn())
		// …but only WITHHOLD the trust-upgrade on attested routes. On a balanced/strict route
		// a valid possession proof past the cap still fast-paths, so a roaming/CGNAT/VPN human
		// on one passkey is never newly challenged on ordinary traffic (restores the
		// "unmarked routes untouched" invariant); the farm still reverts to Pass exactly where
		// the operator priced the ALLOW.
		if subnets, _ := s.credCorr.Stats("cred:" + credID); route.attestFloor && subnets > s.credFanoutCap {
			s.sink.Emit(audit.Record{
				EventType: audit.EventWebAuthnVerified, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, sid)},
				TenantID: s.cfg.NodeID, RouteClass: routeClass(r), Mode: "enforce", KeyID: "k1",
				FailReason: "webauthn credential " + short + " fan-out " + itoaInt(subnets) + " subnets > cap " + itoaInt(s.credFanoutCap) + " — trust-upgrade withheld on attested route, routed to Pass",
			})
			return false // credential-farm fan-out on a priced route: no fast-path; fall through to the floor
		}
	}
	return s.trustUpgrade(w, r, sid, route, audit.EventWebAuthnVerified, "webauthn", "verified WebAuthn assertion, credential "+short)
}

// trustUpgrade is the shared audited ALLOW fast-path for the token gates (Web Bot
// Auth, PAT, WebAuthn): the caller proved trust out-of-band, so forward without a
// behavioral fight. One place so the four verify→forward paths cannot drift
// (PLAN-08 backlog DRY).
func (s *Server) trustUpgrade(w http.ResponseWriter, r *http.Request, sid string, route routePolicy, eventType, rule, reason string) bool {
	rec := audit.Record{
		EventType: eventType, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, sid)},
		TenantID: s.cfg.NodeID, RouteClass: routeClass(r), Verdict: string(VerdictAllow),
		Rules: []string{rule}, Action: "pass", Mode: "enforce",
		FailReason: reason, KeyID: "k1",
	}
	s.sink.EmitAndAct(rec, func() { s.forward(w, r, route) })
	return true
}

// sweepGate flags decision-probing recon: one fingerprint spinning up many
// near-identical sessions (SoT-21 §8, HR-30).
func (s *Server) sweepGate(w http.ResponseWriter, r *http.Request, sid string, route routePolicy, now time.Time) bool {
	if s.sweep == nil {
		return false
	}
	mode := s.gateModuleMode("gate.recon_sweep")
	if mode == settings.ModeOff || mode == settings.ModeEnforce && !s.enforcing(route) {
		return false
	}
	if !s.sweep.Observe(bindKey(r), sid, now) {
		return false
	}
	rec := audit.Record{
		EventType: audit.EventReconProbing, Actor: audit.Actor{Kind: "subject", IDPsn: s.pseudonym(sid, sid)},
		TenantID: s.cfg.NodeID, RouteClass: routeClass(r), Verdict: string(VerdictDeny),
		Rules: []string{"HR-30"}, Action: "block", Mode: "enforce",
		FailReason: "decision-probing sweep (many sessions, one fingerprint)", KeyID: "k1",
	}
	if mode != settings.ModeEnforce {
		rec.Mode = gateModeName(mode)
		rec.Action = "would_block"
		rec.FailReason += " (audit only)"
		s.sink.Emit(rec)
		return false
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
	// Ceiling-guard #1: on an attestFloor route, a scoring-ALLOW that would otherwise pass
	// is demoted to a step-up challenge unless the session holds a step-up proof. This is
	// the second half of the floor (verdictTokenGate handles the token fast-path): it also
	// catches the sticky-ALLOW path (a session scored ALLOW on a prior beacon, no verdict
	// cookie), so a coherent spoof cannot launder its free ALLOW past the floor by any
	// route. A possession-bearing request never reaches applyVerdict (the pre-gates
	// forwarded it), so the only exemption here is a valid step-up proof — exactly
	// Pass-or-possession, never a categorical block (CHALLENGE→Pass, not DENY).
	if s.enforcing(route) && route.attestFloor && actionName == "pass" && !s.hasValidStepUp(r, sid) {
		eventType, actionName = audit.EventEnfChallengeIssued, "challenge_pow"
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

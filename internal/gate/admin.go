package gate

import (
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"runtime"
	"strconv"
	"strings"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/gate/settings"
)

// admin.go is the admin-plane API (SoT-26 §10, hardened per SoT-28 WS1/WS2). It
// is served on a SEPARATE authenticated listener (cmd/gate), removed from
// the public proxied listener. Every data/mutation route requires an
// authenticated operator (bearer token → role); missing/invalid credentials get
// 404 (deny-by-default, non-discoverable). The authenticated operator is the
// server-derived actor for audit — client-supplied identity is never trusted.
// Destructive actions (permanent/CIDR ban, erasure) use genuine two-phase
// dual-control: a distinct second approver must commit (approval.go).

// banView is the console-facing representation of a ban.
type banView struct {
	Key       string `json:"key"`
	Reason    string `json:"reason"`
	Source    string `json:"source"`
	Incident  string `json:"incident,omitempty"`
	Strike    int    `json:"strike"`
	Permanent bool   `json:"permanent"`
	ExpiresIn int    `json:"expiresInSec"`
}

// adminRoute is one row of the declarative admin dispatch table (PLAN-07 R13,
// replacing a stringly-typed switch): a method + a path matcher (exact, prefix, or
// prefix/suffix-wrapped, capturing any dynamic segment) + an RBAC capability
// predicate + the handler. First match wins; a matched route whose capability check
// fails is a 403, an unmatched path is a 404 — identical semantics to the switch it
// replaced, but now the auth matrix is data an auditor can read at a glance.
type adminRoute struct {
	method string
	match  func(sub string) (arg string, ok bool)
	allow  func(op Operator) bool
	handle func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, arg string)
}

// adminExact matches a fixed sub-path and captures nothing.
func adminExact(p string) func(string) (string, bool) {
	return func(sub string) (string, bool) { return "", sub == p }
}

// adminPrefix matches a sub-path under p and captures the remainder (e.g. an id).
func adminPrefix(p string) func(string) (string, bool) {
	return func(sub string) (string, bool) {
		if strings.HasPrefix(sub, p) {
			return strings.TrimPrefix(sub, p), true
		}
		return "", false
	}
}

// adminWrapped matches pre<arg>suf and captures the middle segment.
func adminWrapped(pre, suf string) func(string) (string, bool) {
	return func(sub string) (string, bool) {
		if strings.HasPrefix(sub, pre) && strings.HasSuffix(sub, suf) {
			return strings.TrimSuffix(strings.TrimPrefix(sub, pre), suf), true
		}
		return "", false
	}
}

// RBAC predicates — named so the table reads as a capability matrix.
func adminAnyRole(op Operator) bool    { return op.canRead() } // any authenticated role
func adminCanOperate(op Operator) bool { return op.canOperate() }
func adminCanApprove(op Operator) bool { return op.canApprove() }

// adminRoutes is the ordered dispatch + RBAC table. Reads are open to any
// authenticated role (Auditor default); mutations are role-gated; destructive
// actions additionally require dual-control inside their handlers (approval.go).
var adminRoutes = []adminRoute{
	// --- reads: any authenticated role (Auditor default) ---
	{http.MethodGet, adminExact("bans"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) { s.adminListBans(w) }},
	{http.MethodGet, adminExact("integrity"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) { s.adminIntegrity(w) }},
	{http.MethodGet, adminExact("proof"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) { s.adminProof(w, r) }},
	{http.MethodGet, adminExact("audit"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) { s.adminAudit(w, r) }},
	{http.MethodGet, adminPrefix("incidents/"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, arg string) {
		s.adminIncident(w, arg, op)
	}},
	{http.MethodGet, adminExact("policy"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) { s.adminPolicy(w) }},
	// SoT-39 P1 — Settings read plane (writes land in P3 via Approvals).
	{http.MethodGet, adminExact("settings/effective"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) {
		s.adminSettingsEffective(w)
	}},
	{http.MethodGet, adminExact("settings/schema"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) {
		s.adminSettingsSchema(w)
	}},
	{http.MethodGet, adminExact("settings/overlays"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) {
		s.adminSettingsOverlaysList(w)
	}},
	{http.MethodPost, adminExact("settings/overlays"), adminCanOperate, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) {
		s.adminSettingsPropose(w, r, op)
	}},
	{http.MethodPost, adminExact("settings/dry-run"), adminCanOperate, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) {
		s.adminSettingsDryRun(w, r, op)
	}},
	{http.MethodPost, adminExact("settings/rollback"), adminCanOperate, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) {
		s.adminSettingsRollback(w, r, op)
	}},
	{http.MethodGet, adminExact("approvals"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) {
		s.adminListApprovals(w)
	}},
	{http.MethodGet, adminExact("erasures"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) { s.adminListErasures(w) }},
	{http.MethodGet, adminExact("whoami"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) {
		writeJSON(w, map[string]any{"id": op.ID, "role": op.Role})
	}},
	{http.MethodGet, adminExact("metrics"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) { s.adminMetrics(w) }},
	// SoT-38 WS2 — public verification material (no secrets). Auditors pin these out-of-band.
	{http.MethodGet, adminExact("keys"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) { s.adminKeys(w) }},
	{http.MethodGet, adminExact("checkpoints"), adminAnyRole, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) { s.adminCheckpoints(w) }},

	// --- mutations: role-gated (Operator) ---
	{http.MethodPost, adminExact("bans"), adminCanOperate, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) {
		s.adminAddBan(w, r, op)
	}},
	{http.MethodPost, adminExact("bans/bulk"), adminCanOperate, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) {
		s.adminBulkBan(w, r, op)
	}},
	{http.MethodPost, adminExact("bans/lift"), adminCanOperate, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) {
		s.adminLiftBan(w, r, op)
	}},
	{http.MethodPost, adminExact("erasure"), adminCanOperate, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) {
		s.adminErasure(w, r, op)
	}},
	{http.MethodPost, adminExact("killswitch"), adminCanOperate, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, _ string) {
		s.adminKillSwitch(w, r, op)
	}},
	{http.MethodPost, adminWrapped("erasures/", "/cancel"), adminCanOperate, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, arg string) {
		s.adminCancelErasure(w, arg, op)
	}},

	// --- dual-control commit: Approver ---
	{http.MethodPost, adminPrefix("approvals/"), adminCanApprove, func(s *Server, w http.ResponseWriter, r *http.Request, op Operator, arg string) {
		s.adminApprove(w, arg, op)
	}},
}

// handleAdmin authenticates, meta-audits, RBAC-checks, and dispatches /admin/*.
func (s *Server) handleAdmin(w http.ResponseWriter, r *http.Request, sub string) {
	// The console HTML itself carries no data and is fetched by a top-level
	// navigation that cannot present a bearer header, so it is served without
	// auth; the SPA then authenticates its API calls with an injected token.
	if sub == "" || sub == "console" {
		s.adminConsole(w)
		return
	}

	op, ok := s.auth.Authenticate(r)
	if !ok {
		// deny-by-default 404: the surface is not even discoverable.
		s.sink.Emit(audit.Record{
			EventType: audit.EventAdminAuthFail, Actor: audit.Actor{Kind: "system"},
			TenantID: s.cfg.NodeID, RouteClass: "control", FailReason: "unauthenticated admin " + r.Method + " " + sub, KeyID: "k1",
		})
		http.NotFound(w, r)
		return
	}

	// Meta-audit every authenticated admin access BEFORE serving (SoT-28 §9):
	// the auditors are themselves auditable.
	s.sink.Emit(audit.Record{
		EventType: audit.EventAdminAccess, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
		TenantID: s.cfg.NodeID, RouteClass: "control", Action: r.Method, FailReason: sub + " (role=" + string(op.Role) + ")", KeyID: "k1",
	})

	for _, rt := range adminRoutes {
		if r.Method != rt.method {
			continue
		}
		arg, matched := rt.match(sub)
		if !matched {
			continue
		}
		if !rt.allow(op) {
			// Authenticated but lacking the capability → 403 (distinct from the 404 for
			// unauthenticated / unknown routes).
			http.Error(w, "role "+string(op.Role)+" not permitted", http.StatusForbidden)
			return
		}
		rt.handle(s, w, r, op, arg)
		return
	}
	http.NotFound(w, r)
}

// adminConsole serves the console SPA, injecting a dev bearer token so the SPA
// can call the authenticated API. Production replaces this with an SSO/mTLS
// login that yields a per-operator session (SoT-28 §1-2).
func (s *Server) adminConsole(w http.ResponseWriter) {
	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	w.Header().Set("Cache-Control", "no-store")
	w.Header().Set("X-Frame-Options", "DENY")
	html := styledConsoleHTML
	if s.devConsoleToken != "" {
		html = strings.Replace(html, "<head>", "<head><script>window.__HMN_TOKEN="+strconvQuote(s.devConsoleToken)+"</script>", 1)
	}
	_, _ = w.Write([]byte(html))
}

func strconvQuote(s string) string { return strconv.Quote(s) }

func (s *Server) adminPolicy(w http.ResponseWriter) {
	type routeRow struct {
		Prefix     string `json:"prefix"`
		Preset     string `json:"preset"`
		Enforce    bool   `json:"enforce"`
		FailClosed bool   `json:"failClosed"`
		SyncScore  bool   `json:"syncScore"`
		Inject     bool   `json:"inject"`
	}
	rows := []routeRow{}
	def := presetBalanced
	rows = append(rows, routeRow{"(default)", def.name, def.enforce && !s.cfg.GlobalMonitor, def.failClosed, def.syncScore, def.inject})
	for prefix, name := range s.cfg.Routes {
		p := presetByName(name)
		rows = append(rows, routeRow{prefix, p.name, p.enforce && !s.cfg.GlobalMonitor, p.failClosed, p.syncScore, p.inject})
	}
	writeJSON(w, map[string]any{
		"globalMonitor":    s.cfg.GlobalMonitor,
		"killSwitch":       s.killSwitch.Load(),
		"effectiveMonitor": s.monitorOn(),
		"configVersion":    s.configVersion(),
		"routes":           rows,
		"rateLimit":        map[string]any{"windowSec": int(rlWindow(s.cfg).Seconds()), "soft": rlSoft(s.cfg), "hard": rlHard(s.cfg)},
		"retentionDays":    audit.DefaultRetention().Days(),
		// SoT-39: Policy view is observe-only; Settings is the write plane.
		"settingsWrite":    false,
		"settingsReadPath": "GET /settings/effective",
	})
}

// adminSettingsEffective returns SoT-39 resolved posture (empty overlay ≡ freeze defaults).
func (s *Server) adminSettingsEffective(w http.ResponseWriter) {
	writeJSON(w, adminSettingsEffectiveBody(s))
}

// adminSettingsSchema returns static bounds / integrity catalogs (SoT-39 §7).
func (s *Server) adminSettingsSchema(w http.ResponseWriter) {
	writeJSON(w, settings.Schema())
}

// adminKillSwitch creates a PENDING kill-switch flip (dual-control, SoT-28 §5 /
// WS9): a distinct approver must commit before enforcement is globally
// suppressed to monitor mode — so one rogue operator cannot disable detection.
func (s *Server) adminKillSwitch(w http.ResponseWriter, r *http.Request, op Operator) {
	var req struct{ On bool }
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 1<<12)).Decode(&req); err != nil {
		http.Error(w, "bad request", http.StatusBadRequest)
		return
	}
	p := s.approvals.Create("killswitch", map[string]string{"on": strconv.FormatBool(req.On)}, op.ID, RoleApprover)
	s.sink.Emit(audit.Record{
		EventType: audit.EventApprovalRequested, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
		TenantID: s.cfg.NodeID, Mode: "enforce", FailReason: "kill-switch flip pending approval: on=" + strconv.FormatBool(req.On), KeyID: "k1",
	})
	writeJSON(w, map[string]any{"pending": true, "approvalId": p.ID, "needsRole": RoleApprover})
}

// adminErasure creates a PENDING erasure that a distinct DPO must approve
// (SoT-28 §4-6). It never shreds on the first request and never trusts a
// client-supplied identity.
func (s *Server) adminErasure(w http.ResponseWriter, r *http.Request, op Operator) {
	var req struct{ Subject, LegalBasis string }
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 1<<16)).Decode(&req); err != nil || req.Subject == "" {
		http.Error(w, "subject required", http.StatusBadRequest)
		return
	}
	if req.LegalBasis == "" {
		http.Error(w, "legal basis required (GDPR Art.17 / PIPA)", http.StatusBadRequest)
		return
	}
	// Resolve the operator-supplied handle (a session pseudonym the console shows)
	// to the subject id via the audited reverse index; fall back to the raw value
	// only when it is already a subject id (SoT-28 §6).
	subject := req.Subject
	if sid, ok := s.vault.Resolve(req.Subject); ok {
		subject = sid
	}
	p := s.approvals.Create("erasure", map[string]string{"subject": subject, "legalBasis": req.LegalBasis}, op.ID, RoleDPO)
	s.sink.Emit(audit.Record{
		EventType: audit.EventApprovalRequested, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
		TenantID: s.cfg.NodeID, Mode: "enforce", FailReason: "erasure pending DPO approval; legal_basis=" + req.LegalBasis, KeyID: "k1",
	})
	writeJSON(w, map[string]any{"pending": true, "approvalId": p.ID, "needsRole": RoleDPO})
}

// adminProof returns an offline-verifiable RFC 6962 inclusion proof for a record,
// anchored to the latest Signed Tree Head (PLAN-08 R6). An auditor verifies the
// proof against the STH's Merkle root, whose signature they check independently, to
// prove the record is in exactly the log the STH commits to.
func (s *Server) adminProof(w http.ResponseWriter, r *http.Request) {
	seq, _ := strconv.ParseUint(r.URL.Query().Get("seq"), 10, 64)
	if seq == 0 {
		http.Error(w, "seq query parameter required", http.StatusBadRequest)
		return
	}
	log := s.sink.Log()
	cps := log.Checkpoints()
	if len(cps) == 0 {
		http.Error(w, "no signed tree head yet (record not checkpointed)", http.StatusNotFound)
		return
	}
	sth := cps[len(cps)-1]
	res, ok := log.InclusionProofAt(seq, sth.TreeSize)
	if !ok {
		http.Error(w, "record not covered by the latest signed tree head", http.StatusNotFound)
		return
	}
	proof := make([]string, len(res.Proof))
	for i, p := range res.Proof {
		proof[i] = hex.EncodeToString(p)
	}
	writeJSON(w, map[string]any{
		"seq":       seq,
		"leafData":  hex.EncodeToString(res.LeafData),
		"leafIndex": res.LeafIndex,
		"treeSize":  res.TreeSize,
		"proof":     proof,
		"sth": map[string]any{
			"treeSize":   sth.TreeSize,
			"merkleRoot": sth.MerkleRoot,
			"sig":        sth.Sig,
			"witnessSig": sth.WitnessSig,
		},
	})
}

// adminMetrics serves a Prometheus text-exposition snapshot of gate health on the
// authenticated admin plane (no new dependency). Per-verdict rates are derived from
// the audit stream (GET /audit), which SIEM already ingests; this endpoint covers the
// process, chain-growth, and ban-state gauges an orchestrator or Prometheus scrapes.
func (s *Server) adminMetrics(w http.ResponseWriter) {
	var m runtime.MemStats
	runtime.ReadMemStats(&m)
	log := s.sink.Log()
	uptime := s.nowFn().Sub(s.startedAt).Seconds()
	b01 := func(cond bool) int {
		if cond {
			return 1
		}
		return 0
	}
	w.Header().Set("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
	fmt.Fprintf(w, "# HELP hmn_gate_uptime_seconds Seconds since this gate process started.\n# TYPE hmn_gate_uptime_seconds gauge\nhmn_gate_uptime_seconds %.0f\n", uptime)
	fmt.Fprintf(w, "# HELP hmn_gate_audit_records_total Records sealed into this node's audit chain.\n# TYPE hmn_gate_audit_records_total counter\nhmn_gate_audit_records_total %d\n", log.Len())
	fmt.Fprintf(w, "# HELP hmn_gate_audit_checkpoints_total Signed Tree Heads written.\n# TYPE hmn_gate_audit_checkpoints_total counter\nhmn_gate_audit_checkpoints_total %d\n", len(log.Checkpoints()))
	fmt.Fprintf(w, "# HELP hmn_gate_bans_active Currently active IP/fingerprint bans.\n# TYPE hmn_gate_bans_active gauge\nhmn_gate_bans_active %d\n", len(s.bans.List()))
	// Audit-projection drops: a Tier-1/2 sink (Redis/ClickHouse) shedding records under
	// backpressure/outage is otherwise only a once-a-minute WARN log — alert on any increase.
	fmt.Fprintf(w, "# HELP hmn_gate_audit_projection_dropped_total Audit records dropped by Tier-1/2 projection sinks (the WAL remains the durability authority).\n# TYPE hmn_gate_audit_projection_dropped_total counter\nhmn_gate_audit_projection_dropped_total %d\n", s.projectionDropped())
	// Audit-chain integrity + witness attestation: the highest-severity alert. Served from a
	// cache refreshed OFF the request path (RefreshIntegrityMetrics on the maintenance ticker)
	// so a full-chain verify — O(log size), and in WAL mode a disk read under the audit lock —
	// never runs on a scrape and cannot stall verdict seals.
	fmt.Fprintf(w, "# HELP hmn_gate_audit_integrity_ok Audit chain verifies end-to-end (1) or a mismatch was found (0).\n# TYPE hmn_gate_audit_integrity_ok gauge\nhmn_gate_audit_integrity_ok %d\n", b01(s.integrityOK.Load()))
	fmt.Fprintf(w, "# HELP hmn_gate_audit_witnessed The latest checkpoints carry a valid independent-witness co-signature (1) or not (0).\n# TYPE hmn_gate_audit_witnessed gauge\nhmn_gate_audit_witnessed %d\n", b01(s.auditWitnessed.Load()))
	fmt.Fprintf(w, "# HELP hmn_gate_killswitch Kill switch engaged (1) or not (0).\n# TYPE hmn_gate_killswitch gauge\nhmn_gate_killswitch %d\n", b01(s.killSwitch.Load()))
	fmt.Fprintf(w, "# HELP hmn_gate_monitor Effective global monitor mode (1) or enforcing (0).\n# TYPE hmn_gate_monitor gauge\nhmn_gate_monitor %d\n", b01(s.monitorOn()))
	eff := s.SettingsEffective()
	fmt.Fprintf(w, "# HELP hmn_gate_settings_overlay_active Runtime Settings overlay active (1) or empty/freeze defaults (0).\n# TYPE hmn_gate_settings_overlay_active gauge\nhmn_gate_settings_overlay_active %d\n", b01(!eff.EmptyOverlay))
	fmt.Fprintf(w, "# HELP hmn_gate_settings_overlay_pending_age_seconds Age in seconds of the oldest pending Settings approval (0 when none).\n# TYPE hmn_gate_settings_overlay_pending_age_seconds gauge\nhmn_gate_settings_overlay_pending_age_seconds %.0f\n", s.settingsPendingAgeSeconds())
	fmt.Fprintf(w, "# HELP hmn_gate_settings_apply_total Settings apply outcomes.\n# TYPE hmn_gate_settings_apply_total counter\n")
	fmt.Fprintf(w, "hmn_gate_settings_apply_total{result=\"applied\"} %d\n", s.settingsStats.applied.Load())
	fmt.Fprintf(w, "hmn_gate_settings_apply_total{result=\"rolled_back\"} %d\n", s.settingsStats.rolledBack.Load())
	fmt.Fprintf(w, "hmn_gate_settings_apply_total{result=\"rejected\"} %d\n", s.settingsStats.rejected.Load())
	fmt.Fprintf(w, "hmn_gate_settings_apply_total{result=\"error\"} %d\n", s.settingsStats.errors.Load())
	fmt.Fprintf(w, "# HELP hmn_gate_settings_store_errors_total Settings store load or persistence errors.\n# TYPE hmn_gate_settings_store_errors_total counter\nhmn_gate_settings_store_errors_total %d\n", s.settingsStats.storeErrors.Load())
	fmt.Fprintf(w, "# HELP hmn_gate_config_version Effective signed Settings configuration version.\n# TYPE hmn_gate_config_version gauge\nhmn_gate_config_version{version=\"%s\"} 1\n", prometheusLabel(eff.ConfigVersion))
	fmt.Fprintf(w, "# HELP hmn_gate_goroutines Current goroutine count.\n# TYPE hmn_gate_goroutines gauge\nhmn_gate_goroutines %d\n", runtime.NumGoroutine())
	fmt.Fprintf(w, "# HELP hmn_gate_heap_alloc_bytes Heap bytes allocated and in use.\n# TYPE hmn_gate_heap_alloc_bytes gauge\nhmn_gate_heap_alloc_bytes %d\n", m.HeapAlloc)
	fmt.Fprintf(w, "# HELP hmn_gate_sys_bytes Total memory obtained from the OS.\n# TYPE hmn_gate_sys_bytes gauge\nhmn_gate_sys_bytes %d\n", m.Sys)
}

func (s *Server) adminIntegrity(w http.ResponseWriter) {
	log := s.sink.Log()
	res := log.SelfVerify()
	cps := log.Checkpoints()
	out := map[string]any{"node": s.cfg.NodeID, "ok": res.OK, "class": res.Class, "records": log.Len(), "checkpoints": len(cps)}
	if !res.OK {
		out["divergentSeq"] = res.AtSeq
		out["detail"] = res.Detail
	}
	// Independent witness attestation (SoT-28 WS8): a chain the writer rewrote
	// cannot carry valid witness co-signatures.
	if wpub := log.WitnessPublicKey(); wpub != nil {
		if at, ok := audit.VerifyWitness(cps, wpub); ok {
			out["witnessed"] = true
		} else {
			out["witnessed"] = false
			out["witnessFailAt"] = at
		}
	}
	if n := len(cps); n > 0 {
		out["lastSTH"] = map[string]any{"treeSize": cps[n-1].TreeSize, "root": cps[n-1].Root}
	}
	writeJSON(w, out)
}

// adminKeys publishes STH + witness verification keys (SoT-38 WS2). No HMAC or
// seed material — only what an external auditor needs to verify exported STHs.
func (s *Server) adminKeys(w http.ResponseWriter) {
	log := s.sink.Log()
	out := map[string]any{
		"node_id":            s.cfg.NodeID,
		"key_id":             "k1",
		"sth_public_key":     hex.EncodeToString(log.PublicKey()),
		"witness_public_key": "",
	}
	if wpub := log.WitnessPublicKey(); len(wpub) > 0 {
		out["witness_public_key"] = hex.EncodeToString(wpub)
	}
	writeJSON(w, out)
}

// adminCheckpoints returns the full Signed Tree Head list including writer and
// witness signatures (SoT-38 WS2).
func (s *Server) adminCheckpoints(w http.ResponseWriter) {
	cps := s.sink.Log().Checkpoints()
	writeJSON(w, map[string]any{"checkpoints": cps, "count": len(cps)})
}

// adminAudit serves the audit stream with server-side search, filtering and
// cursor pagination (SoT-28 WS9): verdict, host substring, route_class, rule,
// min risk, and a `before` seq cursor. Records are newest-first.
func (s *Server) adminAudit(w http.ResponseWriter, r *http.Request) {
	q := r.URL.Query()
	limit := 50
	if v := q.Get("limit"); v != "" {
		if n, err := strconv.Atoi(v); err == nil && n > 0 && n <= 500 {
			limit = n
		}
	}
	vf, host, route, rule := q.Get("verdict"), strings.ToLower(q.Get("host")), q.Get("route"), q.Get("rule")
	// subject scopes the stream to one pseudonymous subject (GDPR Art. 15 access request):
	// the deterministic session/id pseudonym, matched against session_pseudonym or id_pseudonym.
	subject := q.Get("subject")
	minRisk, _ := strconv.Atoi(q.Get("minRisk"))
	before, _ := strconv.ParseUint(q.Get("before"), 10, 64) // pagination cursor (0 = newest)

	recs := s.sink.Log().Records()
	out := make([]audit.Record, 0, limit)
	var nextCursor uint64
	for i := len(recs) - 1; i >= 0 && len(out) < limit; i-- {
		rec := recs[i]
		if before != 0 && rec.Seq >= before {
			continue
		}
		if vf != "" && rec.Verdict != vf {
			continue
		}
		if host != "" && !strings.Contains(strings.ToLower(rec.Host), host) {
			continue
		}
		if route != "" && rec.RouteClass != route {
			continue
		}
		if minRisk > 0 && rec.RiskScore < minRisk {
			continue
		}
		if rule != "" && !hasRule(rec.Rules, rule) {
			continue
		}
		if subject != "" && rec.SessionPsn != subject && rec.Actor.IDPsn != subject {
			continue
		}
		rec.Incident = s.incidentHandle(rec.Seq) // opaque handle for the console (WS4)
		out = append(out, rec)
		nextCursor = rec.Seq
	}
	writeJSON(w, map[string]any{"records": out, "count": len(out), "nextBefore": nextCursor})
}

func hasRule(rules []string, want string) bool {
	for _, r := range rules {
		if r == want {
			return true
		}
	}
	return false
}

// adminIncident looks up a record by its OPAQUE incident handle (SoT-28 WS4).
// The monotonic seq is never accepted here — that would allow enumeration. A
// per-operator lookup cap turns a trawl into a 429 + audited alert.
func (s *Server) adminIncident(w http.ResponseWriter, ref string, op Operator) {
	if lvl := s.incidentLimiter.Level(s.incidentLimiter.Observe("inc:"+op.ID, s.nowFn())); lvl == 2 {
		s.sink.Emit(audit.Record{
			EventType: audit.EventReconProbing, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
			TenantID: s.cfg.NodeID, RouteClass: "control", FailReason: "incident enumeration cap exceeded (bulk-read)", KeyID: "k1",
		})
		http.Error(w, "too many incident lookups", http.StatusTooManyRequests)
		return
	}
	rec, ok := s.resolveIncident(ref)
	if !ok {
		http.Error(w, "incident not found", http.StatusNotFound)
		return
	}
	writeJSON(w, rec)
}

func (s *Server) adminListBans(w http.ResponseWriter) {
	now := s.nowFn()
	out := make([]banView, 0)
	for _, b := range s.bans.List() {
		v := banView{Key: b.Key, Reason: b.Reason, Source: b.Source, Incident: b.Incident, Strike: b.Strike, Permanent: b.Permanent()}
		if !b.Permanent() {
			if d := int(b.Until.Sub(now).Seconds()); d > 0 {
				v.ExpiresIn = d
			}
		}
		out = append(out, v)
	}
	writeJSON(w, map[string]any{"bans": out, "count": len(out)})
}

// maxBanDurationSec (~400 days) is the ceiling for a single-operator temporary ban. At or
// above it a ban is treated as effectively permanent and routed to dual-control; it also
// keeps time.Duration(sec)*time.Second well within int64 so a huge value cannot wrap
// negative and be silently reinterpreted as a permanent ban (deep-review).
const maxBanDurationSec = 400 * 24 * 3600

// adminAddBan applies a temporary ban directly (authenticated Operator) or, for
// a permanent/CIDR/broad ban, creates a PENDING action a distinct Approver must
// commit (SoT-27 §4 / SoT-28 §7). The actor is the authenticated operator.
func (s *Server) adminAddBan(w http.ResponseWriter, r *http.Request, op Operator) {
	var req struct {
		Key, Reason, Incident string
		DurationSec           int
	}
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 1<<16)).Decode(&req); err != nil || req.Key == "" {
		http.Error(w, "bad request", http.StatusBadRequest)
		return
	}
	if !validBanKey(req.Key) {
		http.Error(w, "ban key must be ip:<addr>, fp:<hash>, or cidr:<addr/mask>", http.StatusBadRequest)
		return
	}
	if req.DurationSec < 0 {
		http.Error(w, "durationSec must be >= 0", http.StatusBadRequest)
		return
	}
	// Permanent (0), effectively-permanent (>= cap), OR broad (CIDR) bans need dual-control
	// regardless of duration. The >= cap check ALSO closes the overflow bypass where a huge
	// DurationSec wrapped time.Duration(sec)*time.Second negative -> dur<=0 -> a PERMANENT ban
	// committed by ONE operator, defeating dual-control and the no-lockout constraint
	// (deep-review). Below the cap the multiply cannot overflow int64.
	if req.DurationSec == 0 || req.DurationSec >= maxBanDurationSec || isBroadKey(req.Key) {
		p := s.approvals.Create("ban", map[string]string{
			"key": req.Key, "reason": req.Reason, "incident": req.Incident, "durationSec": strconv.Itoa(req.DurationSec),
		}, op.ID, RoleApprover)
		s.sink.Emit(audit.Record{
			EventType: audit.EventApprovalRequested, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
			TenantID: s.cfg.NodeID, Mode: "enforce", FailReason: "ban pending approval: " + req.Key, KeyID: "k1",
		})
		writeJSON(w, map[string]any{"pending": true, "approvalId": p.ID, "needsRole": RoleApprover})
		return
	}
	entry := s.commitBan(req.Key, req.Reason, req.Incident, time.Duration(req.DurationSec)*time.Second, op.ID, "")
	writeJSON(w, map[string]any{"ok": true, "permanent": entry.Permanent()})
}

// adminBulkBan applies a temporary ban to many keys at once (SoT-28 WS9), e.g.
// banning all IPs behind a rotating scraper. Permanent/CIDR keys are rejected
// here — those must go through dual-control (SoT-27 §4).
func (s *Server) adminBulkBan(w http.ResponseWriter, r *http.Request, op Operator) {
	var req struct {
		Keys        []string
		Reason      string
		DurationSec int
	}
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 1<<18)).Decode(&req); err != nil || len(req.Keys) == 0 {
		http.Error(w, "keys required", http.StatusBadRequest)
		return
	}
	if req.DurationSec <= 0 || req.DurationSec >= maxBanDurationSec {
		http.Error(w, "bulk bans must be temporary and bounded (0/negative/over-cap needs dual-control)", http.StatusBadRequest)
		return
	}
	applied, skipped := 0, 0
	for _, k := range req.Keys {
		if !validBanKey(k) || isBroadKey(k) {
			skipped++
			continue
		}
		s.commitBan(k, req.Reason, "bulk", time.Duration(req.DurationSec)*time.Second, op.ID, "")
		applied++
	}
	writeJSON(w, map[string]any{"ok": true, "applied": applied, "skipped": skipped})
}

// adminLiftBan removes a ban. Authenticated Operator only — an unauthenticated
// (e.g. auto-banned) caller never reaches here (404 at the auth gate), which
// closes the self-lift attack (SoT-28 §7).
func (s *Server) adminLiftBan(w http.ResponseWriter, r *http.Request, op Operator) {
	key := r.URL.Query().Get("key")
	if key == "" {
		http.Error(w, "key required", http.StatusBadRequest)
		return
	}
	lifted := s.bans.Lift(key)
	s.sink.Emit(audit.Record{
		EventType: audit.EventBanLifted, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
		TenantID: s.cfg.NodeID, Action: "pass", Mode: "enforce", FailReason: "manual unblock: " + key, KeyID: "k1",
	})
	writeJSON(w, map[string]any{"ok": true, "lifted": lifted})
}

// adminListErasures lists erasures in their cancellable hold window (SoT-28 WS3).
func (s *Server) adminListErasures(w http.ResponseWriter) {
	now := s.nowFn()
	out := make([]map[string]any, 0)
	for _, e := range s.shreds.List() {
		out = append(out, map[string]any{"id": e.ID, "legalBasis": e.LegalBasis,
			"requester": e.Requester, "approver": e.Approver, "executesInSec": int(e.ExecuteAt.Sub(now).Seconds())})
	}
	writeJSON(w, map[string]any{"scheduled": out, "count": len(out)})
}

// adminCancelErasure cancels a scheduled erasure during its hold window.
func (s *Server) adminCancelErasure(w http.ResponseWriter, id string, op Operator) {
	e, ok := s.shreds.Cancel(id)
	if !ok {
		http.Error(w, "no such scheduled erasure", http.StatusNotFound)
		return
	}
	s.sink.Emit(audit.Record{
		EventType: "erasure.cancelled", Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
		TenantID: s.cfg.NodeID, Mode: "enforce", FailReason: "erasure cancelled in hold window; id=" + e.ID, KeyID: "k1",
	})
	writeJSON(w, map[string]any{"ok": true, "cancelled": e.ID})
}

func (s *Server) adminListApprovals(w http.ResponseWriter) {
	pend := s.approvals.Pending()
	out := make([]map[string]any, 0, len(pend))
	for _, p := range pend {
		out = append(out, map[string]any{"id": p.ID, "kind": p.Kind, "params": p.Params, "needsRole": p.NeedsRole})
	}
	writeJSON(w, map[string]any{"pending": out, "count": len(out)})
}

// adminApprove commits a pending action by a DISTINCT second approver (SoT-28 §4).
func (s *Server) adminApprove(w http.ResponseWriter, id string, op Operator) {
	p, ticket, err := s.approvals.Approve(id, op)
	if err != nil {
		http.Error(w, err.Error(), http.StatusForbidden)
		return
	}
	// Erasure additionally requires the approver to be a DPO (SoT-28 §5).
	if p.Kind == "erasure" && !op.canApproveErasure() {
		http.Error(w, "erasure approval requires the DPO role", http.StatusForbidden)
		return
	}
	s.sink.Emit(audit.Record{
		EventType: audit.EventApprovalGranted, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
		TenantID: s.cfg.NodeID, Mode: "enforce", FailReason: p.Kind + " approved by " + op.ID + " (requester " + p.Requester + "); ticket=" + ticket[:12], KeyID: "k1",
	})
	switch p.Kind {
	case "ban":
		dur, _ := strconv.Atoi(p.Params["durationSec"])
		// 0 or an at/above-cap value => a PERMANENT ban (both approved by two principals);
		// otherwise an exact, overflow-safe duration. Never rely on multiply overflow.
		var d time.Duration
		if dur > 0 && dur < maxBanDurationSec {
			d = time.Duration(dur) * time.Second
		}
		entry := s.commitBan(p.Params["key"], p.Params["reason"], p.Params["incident"], d, p.Requester, op.ID)
		writeJSON(w, map[string]any{"ok": true, "committed": "ban", "permanent": entry.Permanent()})
	case "erasure":
		// Do NOT shred immediately — SCHEDULE with a cancellable hold window; the
		// shred + signed certificate happen when the window elapses (SoT-28 §5/§7).
		sched := s.shreds.Schedule(p.Params["subject"], p.Requester, op.ID, p.Params["legalBasis"])
		s.sink.Emit(audit.Record{
			EventType: audit.EventApprovalGranted, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
			TenantID: s.cfg.NodeID, Mode: "enforce", KeyID: "k1",
			FailReason: "erasure scheduled (cancellable hold); id=" + sched.ID + " legal_basis=" + p.Params["legalBasis"],
		})
		writeJSON(w, map[string]any{"ok": true, "committed": "erasure", "scheduled": true, "shredId": sched.ID, "executeAtUnix": sched.ExecuteAt.Unix()})
	case "killswitch":
		on := p.Params["on"] == "true"
		s.SetKillSwitch(on)
		s.sink.Emit(audit.Record{
			EventType: audit.EventConfigChanged, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(op.ID, op.ID)},
			TenantID: s.cfg.NodeID, Mode: "enforce", FailReason: "kill switch " + map[bool]string{true: "ENGAGED (global monitor)", false: "released"}[on] + " by " + p.Requester + "→" + op.ID, KeyID: "k1",
		})
		writeJSON(w, map[string]any{"ok": true, "committed": "killswitch", "monitorOn": s.monitorOn()})
	case "settings.overlay":
		if err := s.commitSettingsOverlay(p, op); err != nil {
			http.Error(w, err.Error(), http.StatusConflict)
			return
		}
		writeJSON(w, map[string]any{
			"ok": true, "committed": "settings.overlay",
			"configVersion": s.SettingsEffective().ConfigVersion,
			"overlayId":     p.Params["overlayId"],
		})
	case "settings.overlay.rollback":
		if err := s.clearOverlayCAS(p.Params["parentConfigVersion"], p.Requester, op.ID); err != nil {
			http.Error(w, err.Error(), http.StatusConflict)
			return
		}
		writeJSON(w, map[string]any{
			"ok": true, "committed": "settings.overlay.rollback",
			"configVersion": s.SettingsEffective().ConfigVersion,
		})
	default:
		http.Error(w, "unknown action kind", http.StatusInternalServerError)
	}
}

// commitBan applies a ban and audits it with the server-derived requester (and
// approver, when dual-controlled).
func (s *Server) commitBan(key, reason, incident string, dur time.Duration, requester, approver string) BanEntry {
	entry := s.bans.Add(key, reason, s.pseudonym(requester, requester), incident, dur)
	fr := banReason(entry)
	if approver != "" {
		fr += " (dual-control: " + requester + "→" + approver + ")"
	}
	s.sink.Emit(audit.Record{
		EventType: audit.EventBanApplied, Actor: audit.Actor{Kind: "operator", IDPsn: s.pseudonym(requester, requester)},
		TenantID: s.cfg.NodeID, Verdict: string(VerdictDeny), Action: "block", Mode: "enforce", FailReason: fr, KeyID: "k1",
	})
	return entry
}

// validBanKey enforces the ip:/fp:/cidr: key format (SoT-28 §7) with real
// address material — a bare "ip:" / "fp:" / "cidr:" prefix is not enough.
// Control characters and non-printable payloads are rejected so ban keys cannot
// smuggle framing or log-injection bytes into the ban store / audit trail.
func validBanKey(k string) bool {
	if k == "" || strings.ContainsAny(k, "\r\n\x00") {
		return false
	}
	for _, r := range k {
		if r < 32 || r == 127 {
			return false
		}
	}
	switch {
	case strings.HasPrefix(k, "ip:"):
		ip := strings.TrimSpace(k[len("ip:"):])
		// ParseIP rejects empty, garbage, and CIDR-suffix forms (use cidr: for ranges).
		return ip != "" && net.ParseIP(ip) != nil
	case strings.HasPrefix(k, "fp:"):
		fp := k[len("fp:"):]
		if len(fp) < 4 || len(fp) > 128 {
			return false
		}
		for _, r := range fp {
			if r < 33 || r > 126 {
				return false
			}
		}
		return true
	case strings.HasPrefix(k, "cidr:"):
		c := strings.TrimSpace(k[len("cidr:"):])
		if c == "" {
			return false
		}
		_, _, err := net.ParseCIDR(c)
		return err == nil
	default:
		return false
	}
}

// isBroadKey reports a wide-blast-radius key (CIDR / range) that always needs
// dual-control.
func isBroadKey(k string) bool {
	return strings.HasPrefix(k, "cidr:") || strings.Contains(k, "/")
}

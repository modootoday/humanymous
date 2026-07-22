// Package audit implements the enterprise tamper-evident audit log (SoT-18): an
// append-only, hash-chained, per-record-HMAC event store with asymmetric
// (Ed25519) Signed-Tree-Head checkpoints, per-subject linkage-key pseudonymization
// for real GDPR/PIPA crypto-shred, and a frozen canonical form so an offline
// verifier reproduces record hashes byte-for-byte. It is the substrate every
// proxy enforcement decision emits into BEFORE the action takes effect.
package audit

// record.go defines the canonical audit record and the event taxonomy (SoT-18
// §2, §7). Fields mirror the SoT schema; identifiers are already pseudonymized
// by the caller (see pseudonym.go) — no raw PII ever reaches a Record.

// Verdict / action / mode / fail enums kept as plain strings to stay JSON-stable.
type (
	// Record is one immutable audit event. seq/prevHash/recordHash/hmac are
	// filled by the Log at append time; callers set the semantic fields only.
	Record struct {
		EventID     string    `json:"event_id"`      // UUIDv7 (display/correlation, NOT the dedupe key)
		Seq         uint64    `json:"seq"`           // per-node monotonic sequence; a gap is an integrity violation
		NodeID      string    `json:"node_id"`       // proxy instance; each node owns its own chain
		TS          string    `json:"ts"`            // RFC3339 nanos UTC; observational only, NOT the order of record
		EventType   string    `json:"event_type"`    // taxonomy value (see Event* constants)
		EventVer    string    `json:"event_version"` // schema semver of this event type
		Actor       Actor     `json:"actor"`
		TenantID    string    `json:"tenant_id"` // tenant isolation boundary (SoT-18 §11)
		SessionPsn  string    `json:"session_pseudonym,omitempty"`
		RequestID   string    `json:"request_id,omitempty"`
		Correlation string    `json:"correlation_id,omitempty"`
		Host        string    `json:"host,omitempty"`
		RouteClass  string    `json:"route_class,omitempty"` // html|api|upgrade|control|static
		Verdict     string    `json:"verdict,omitempty"`     // allow|challenge|deny|none
		RiskScore   int       `json:"risk_score"`
		Rules       []string  `json:"triggered_rules"` // HR-xx, sorted (canonical)
		TopSignals  []Signal  `json:"top_signals"`     // id/verdict/conf only, sorted by id
		Action      string    `json:"enforcement_action,omitempty"`
		Mode        string    `json:"enforcement_mode,omitempty"` // monitor|shadow|enforce
		FailMode    string    `json:"fail_mode,omitempty"`        // none|fail_open|fail_closed|degraded
		FailReason  string    `json:"fail_reason,omitempty"`
		LatencyUS   uint32    `json:"latency_us"`
		Upstream    *Upstream `json:"upstream,omitempty"`
		TLS         *TLSPsn   `json:"tls,omitempty"`
		ConfigVer   string    `json:"config_version,omitempty"` // bound to an APPROVED signed config
		KeyID       string    `json:"key_id"`                   // signing/pepper key version
		DataClass   string    `json:"data_class,omitempty"`

		// Integrity fields — set by the Log, excluded from the canonical body.
		PrevHash   string `json:"prev_hash"`
		RecordHash string `json:"record_hash"`
		HMAC       string `json:"hmac"`

		// Incident is an opaque, non-enumerable handle computed at READ time only
		// (never stored, excluded from the canonical body) so the console never
		// exposes the monotonic seq (SoT-28 WS4).
		Incident string `json:"incident,omitempty"`
	}

	Actor struct {
		Kind  string `json:"kind"` // system|subject|operator
		IDPsn string `json:"id_pseudonym,omitempty"`
	}
	Signal struct {
		ID      string  `json:"id"`
		Verdict string  `json:"verdict"`
		Conf    float64 `json:"conf"`
	}
	Upstream struct {
		Status     int    `json:"status"`
		ErrorClass string `json:"error_class,omitempty"`
	}
	TLSPsn struct {
		JA4Psn  string `json:"ja4_pseudonym,omitempty"`
		H2FPPsn string `json:"h2fp_pseudonym,omitempty"`
		SNIPsn  string `json:"sni_pseudonym,omitempty"`
	}
)

// Event taxonomy (SoT-18 §7). Only the codes referenced by the proxy layer are
// enumerated as constants; the taxonomy is versioned and additions are audited.
const (
	EventSessionBind          = "session.bind"
	EventTLSCaptured          = "tls.fingerprint.captured"
	EventH2Captured           = "h2.fingerprint.captured"
	EventInjectApplied        = "html.injection.applied"
	EventInjectSkipped        = "html.injection.skipped"
	EventInjectFailed         = "html.injection.failed"
	EventScoringEvaluated     = "scoring.evaluated"
	EventRuleTriggered        = "rule.hard.triggered"
	EventVerdictAssigned      = "verdict.assigned"
	EventVerdictChanged       = "verdict.changed"
	EventEnfAllow             = "enforcement.allow"
	EventEnfChallengeIssued   = "enforcement.challenge.issued"
	EventEnfDeny              = "enforcement.deny"
	EventEnfTarpit            = "enforcement.tarpit"
	EventEnfRateLimit         = "enforcement.ratelimit.applied"
	EventTokenIssued          = "token.issued"
	EventTokenBindingMismatch = "token.binding_mismatch"
	EventTokenBadSig          = "token.bad_sig"
	EventBeaconReplay         = "beacon.replay"
	EventSmuggle              = "net.smuggle"
	EventOriginDirectHit      = "origin.direct_hit"
	EventOriginLeakStripped   = "origin.leak_stripped"
	EventUpgradeNoVerdict     = "upgrade.no_prior_verdict"
	EventInjectNoCallback     = "inject.no_callback"
	EventHdrSpoofedForwarded  = "hdr.spoofed_forwarded"
	EventHdrInternalIngress   = "hdr.internal_header_ingress"
	EventReconProbing         = "recon.decision_probing"
	EventAbuseH2DoS           = "abuse.h2dos.detected"
	EventUpstreamForwarded    = "upstream.request.forwarded"
	EventUpstreamError        = "upstream.error"              // origin unreachable/timeout → 502 (SoT-19; PLAN-07 R16)
	EventAgentVerified        = "agent.signature.verified"    // valid Web Bot Auth sig from an allowlisted key (PLAN-08 R3)
	EventAgentForged          = "agent.signature.forged"      // claims an allowlisted key but fails verification
	EventAgentUnknown         = "agent.signature.unknown"     // signed by an unknown/unsupported key (neutral)
	EventPATVerified          = "privacypass.token.verified"  // valid RFC 9578 Private Access Token (PLAN-08 R2)
	EventWebAuthnVerified     = "webauthn.assertion.verified" // valid WebAuthn possession assertion (PLAN-08 R2)
	EventResponseEgress       = "response.egress"
	EventFailOpen             = "failopen.triggered"
	EventFailClosed           = "failclosed.triggered"
	EventAdminAccess          = "admin.access"       // meta-audit of an admin read (SoT-28 §9)
	EventAdminAuthFail        = "admin.auth_fail"    // rejected admin credential
	EventApprovalRequested    = "approval.requested" // dual-control pending created (SoT-28 §4)
	EventApprovalGranted      = "approval.granted"   // dual-control committed by second principal
	EventBanApplied           = "ban.applied"
	EventBanEnforced          = "ban.enforced"
	EventBanExpired           = "ban.expired"
	EventBanLifted            = "ban.lifted"
	EventRateSoftExceeded     = "ratelimit.soft_exceeded"
	EventRateHardExceeded     = "ratelimit.hard_exceeded"
	EventConfigChanged        = "config.changed"
	EventKeyRotated           = "key.rotated"
	EventOperatorOverride     = "operator.override"
	EventErasureRequested     = "erasure.requested"
	EventErasureCompleted     = "erasure.completed"
	EventIntegrityPassed      = "audit.integrity.check.passed"
	EventIntegrityViolation   = "audit.integrity.violation.detected"
	EventInstanceStartup      = "instance.startup"
	EventInstanceShutdown     = "instance.shutdown"
)

// EventSchemaVersion is the current taxonomy schema version.
const EventSchemaVersion = "1.0.0"

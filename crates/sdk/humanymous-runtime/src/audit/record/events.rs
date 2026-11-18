//! Event taxonomy (SoT-18 section 7), ported from Go `internal/audit/record.go`
//! at oracle 981d25c. Only the codes referenced by the proxy layer are
//! enumerated; the taxonomy is versioned and additions are audited. Names and
//! string values are frozen: they are hashed into sealed records.

/// Go `EventSessionBind`.
pub const EVENT_SESSION_BIND: &str = "session.bind";
/// Go `EventTLSCaptured`.
pub const EVENT_TLS_CAPTURED: &str = "tls.fingerprint.captured";
/// Go `EventH2Captured`.
pub const EVENT_H2_CAPTURED: &str = "h2.fingerprint.captured";
/// Go `EventInjectApplied`.
pub const EVENT_INJECT_APPLIED: &str = "html.injection.applied";
/// Go `EventInjectSkipped`.
pub const EVENT_INJECT_SKIPPED: &str = "html.injection.skipped";
/// Go `EventInjectFailed`.
pub const EVENT_INJECT_FAILED: &str = "html.injection.failed";
/// Go `EventScoringEvaluated`.
pub const EVENT_SCORING_EVALUATED: &str = "scoring.evaluated";
/// Go `EventRuleTriggered`.
pub const EVENT_RULE_TRIGGERED: &str = "rule.hard.triggered";
/// Go `EventVerdictAssigned`.
pub const EVENT_VERDICT_ASSIGNED: &str = "verdict.assigned";
/// Go `EventVerdictChanged`.
pub const EVENT_VERDICT_CHANGED: &str = "verdict.changed";
/// Go `EventEnfAllow`.
pub const EVENT_ENF_ALLOW: &str = "enforcement.allow";
/// Go `EventEnfChallengeIssued`.
pub const EVENT_ENF_CHALLENGE_ISSUED: &str = "enforcement.challenge.issued";
/// Go `EventEnfDeny`.
pub const EVENT_ENF_DENY: &str = "enforcement.deny";
/// Go `EventEnfTarpit`.
pub const EVENT_ENF_TARPIT: &str = "enforcement.tarpit";
/// Go `EventEnfRateLimit`.
pub const EVENT_ENF_RATE_LIMIT: &str = "enforcement.ratelimit.applied";
/// Go `EventTokenIssued`.
pub const EVENT_TOKEN_ISSUED: &str = "token.issued";
/// Go `EventTokenBindingMismatch`.
pub const EVENT_TOKEN_BINDING_MISMATCH: &str = "token.binding_mismatch";
/// Go `EventTokenBadSig`.
pub const EVENT_TOKEN_BAD_SIG: &str = "token.bad_sig";
/// Go `EventBeaconReplay`.
pub const EVENT_BEACON_REPLAY: &str = "beacon.replay";
/// Go `EventSmuggle`.
pub const EVENT_SMUGGLE: &str = "net.smuggle";
/// Go `EventOriginDirectHit`.
pub const EVENT_ORIGIN_DIRECT_HIT: &str = "origin.direct_hit";
/// Go `EventOriginLeakStripped`.
pub const EVENT_ORIGIN_LEAK_STRIPPED: &str = "origin.leak_stripped";
/// Go `EventUpgradeNoVerdict`.
pub const EVENT_UPGRADE_NO_VERDICT: &str = "upgrade.no_prior_verdict";
/// Go `EventInjectNoCallback`.
pub const EVENT_INJECT_NO_CALLBACK: &str = "inject.no_callback";
/// Go `EventHdrSpoofedForwarded`.
pub const EVENT_HDR_SPOOFED_FORWARDED: &str = "hdr.spoofed_forwarded";
/// Go `EventHdrInternalIngress`.
pub const EVENT_HDR_INTERNAL_INGRESS: &str = "hdr.internal_header_ingress";
// Network plane residuals (score-exempt). Operators detect them in the audit
// stream and block through bans; they do not move the risk score.
/// Go `EventNetProxyHop`.
pub const EVENT_NET_PROXY_HOP: &str = "net.proxy.hop";
/// Go `EventNetClientIPSpoof`.
pub const EVENT_NET_CLIENT_IP_SPOOF: &str = "net.proxy.client_ip_spoof";
/// Go `EventNetXFFMultiHop`.
pub const EVENT_NET_XFF_MULTI_HOP: &str = "net.proxy.xff_multi_hop";
/// Go `EventNetForwardedPrivate`.
pub const EVENT_NET_FORWARDED_PRIVATE: &str = "net.proxy.forwarded_private";
/// Go `EventNetAnonChain`.
pub const EVENT_NET_ANON_CHAIN: &str = "net.proxy.anon_chain";
/// Go `EventNetVPNWebRTC`.
pub const EVENT_NET_VPN_WEBRTC: &str = "net.vpn.webrtc_leak";
/// Go `EventNetTorCircuit`.
pub const EVENT_NET_TOR_CIRCUIT: &str = "net.tor.circuit";
/// Go `EventNetTorExit`.
pub const EVENT_NET_TOR_EXIT: &str = "net.tor.exit";
/// Go `EventNetIPDatacenter`.
pub const EVENT_NET_IP_DATACENTER: &str = "net.ip.datacenter";
/// Go `EventNetIPProxyVPN`.
pub const EVENT_NET_IP_PROXY_VPN: &str = "net.ip.proxy_vpn";
/// Go `EventNetTCPNotObserved`.
pub const EVENT_NET_TCP_NOT_OBSERVED: &str = "net.tcp.not_observed";
/// Go `EventNetTCPFingerprint`.
pub const EVENT_NET_TCP_FINGERPRINT: &str = "net.tcp.fingerprint";
/// Go `EventNetTCPTTLOS`.
pub const EVENT_NET_TCP_TTL_OS: &str = "net.tcp.ttl_os_mismatch";
/// Go `EventNetProxyRotation`.
pub const EVENT_NET_PROXY_ROTATION: &str = "net.correlation.proxy_rotation";
/// Go `EventNetFPChurn`.
pub const EVENT_NET_FP_CHURN: &str = "net.correlation.fp_churn";
/// Go `EventNetSharedFP`.
pub const EVENT_NET_SHARED_FP: &str = "net.correlation.shared_fingerprint";
/// Go `EventNetIPHop`.
pub const EVENT_NET_IP_HOP: &str = "net.traffic.ip_hop";
/// Go `EventReconProbing`.
pub const EVENT_RECON_PROBING: &str = "recon.decision_probing";
/// Go `EventAbuseH2DoS`.
pub const EVENT_ABUSE_H2_DOS: &str = "abuse.h2dos.detected";
/// Go `EventUpstreamForwarded`.
pub const EVENT_UPSTREAM_FORWARDED: &str = "upstream.request.forwarded";
/// Go `EventUpstreamError`: origin unreachable/timeout -> 502 (SoT-19; PLAN-07 R16)
pub const EVENT_UPSTREAM_ERROR: &str = "upstream.error";
/// Go `EventAgentVerified`: valid Web Bot Auth sig from an allowlisted key (PLAN-08 R3)
pub const EVENT_AGENT_VERIFIED: &str = "agent.signature.verified";
/// Go `EventAgentForged`: claims an allowlisted key but fails verification
pub const EVENT_AGENT_FORGED: &str = "agent.signature.forged";
/// Go `EventAgentUnknown`: signed by an unknown/unsupported key (neutral)
pub const EVENT_AGENT_UNKNOWN: &str = "agent.signature.unknown";
/// Go `EventPATVerified`: valid RFC 9578 Private Access Token (PLAN-08 R2)
pub const EVENT_PAT_VERIFIED: &str = "privacypass.token.verified";
/// Go `EventWebAuthnVerified`: valid WebAuthn possession assertion (PLAN-08 R2)
pub const EVENT_WEBAUTHN_VERIFIED: &str = "webauthn.assertion.verified";
/// Go `EventResponseEgress`.
pub const EVENT_RESPONSE_EGRESS: &str = "response.egress";
/// Go `EventFailOpen`.
pub const EVENT_FAIL_OPEN: &str = "failopen.triggered";
/// Go `EventFailClosed`.
pub const EVENT_FAIL_CLOSED: &str = "failclosed.triggered";
/// Go `EventAdminAccess`: meta-audit of an admin read (SoT-28 §9)
pub const EVENT_ADMIN_ACCESS: &str = "admin.access";
/// Go `EventAdminAuthFail`: rejected admin credential
pub const EVENT_ADMIN_AUTH_FAIL: &str = "admin.auth_fail";
/// Go `EventApprovalRequested`: dual-control pending created (SoT-28 §4)
pub const EVENT_APPROVAL_REQUESTED: &str = "approval.requested";
/// Go `EventApprovalGranted`: dual-control committed by second principal
pub const EVENT_APPROVAL_GRANTED: &str = "approval.granted";
/// Go `EventBanApplied`.
pub const EVENT_BAN_APPLIED: &str = "ban.applied";
/// Go `EventBanEnforced`.
pub const EVENT_BAN_ENFORCED: &str = "ban.enforced";
/// Go `EventBanExpired`.
pub const EVENT_BAN_EXPIRED: &str = "ban.expired";
/// Go `EventBanLifted`.
pub const EVENT_BAN_LIFTED: &str = "ban.lifted";
/// Go `EventRateSoftExceeded`.
pub const EVENT_RATE_SOFT_EXCEEDED: &str = "ratelimit.soft_exceeded";
/// Go `EventRateHardExceeded`.
pub const EVENT_RATE_HARD_EXCEEDED: &str = "ratelimit.hard_exceeded";
/// Go `EventConfigChanged`.
pub const EVENT_CONFIG_CHANGED: &str = "config.changed";
/// Go `EventKeyRotated`.
pub const EVENT_KEY_ROTATED: &str = "key.rotated";
/// Go `EventOperatorOverride`.
pub const EVENT_OPERATOR_OVERRIDE: &str = "operator.override";
/// Go `EventErasureRequested`.
pub const EVENT_ERASURE_REQUESTED: &str = "erasure.requested";
/// Go `EventErasureCompleted`.
pub const EVENT_ERASURE_COMPLETED: &str = "erasure.completed";
/// Go `EventIntegrityPassed`.
pub const EVENT_INTEGRITY_PASSED: &str = "audit.integrity.check.passed";
/// Go `EventIntegrityViolation`.
pub const EVENT_INTEGRITY_VIOLATION: &str = "audit.integrity.violation.detected";
/// Go `EventInstanceStartup`.
pub const EVENT_INSTANCE_STARTUP: &str = "instance.startup";
/// Go `EventInstanceShutdown`.
pub const EVENT_INSTANCE_SHUTDOWN: &str = "instance.shutdown";

/// Go `EventSchemaVersion`: the current taxonomy schema version stamped on
/// records that do not carry one.
pub const EVENT_SCHEMA_VERSION: &str = "1.0.0";

#[cfg(test)]
mod tests;

use super::*;
use std::collections::BTreeSet;

/// Every Go `Event*` constant at oracle 981d25c, as (Go name, Rust value,
/// frozen Go string). The literals are copied from the Go source, not from
/// the Rust constants, so a drifted value fails here.
const GO_TAXONOMY: [(&str, &str, &str); 74] = [
    ("EventSessionBind", EVENT_SESSION_BIND, "session.bind"),
    (
        "EventTLSCaptured",
        EVENT_TLS_CAPTURED,
        "tls.fingerprint.captured",
    ),
    (
        "EventH2Captured",
        EVENT_H2_CAPTURED,
        "h2.fingerprint.captured",
    ),
    (
        "EventInjectApplied",
        EVENT_INJECT_APPLIED,
        "html.injection.applied",
    ),
    (
        "EventInjectSkipped",
        EVENT_INJECT_SKIPPED,
        "html.injection.skipped",
    ),
    (
        "EventInjectFailed",
        EVENT_INJECT_FAILED,
        "html.injection.failed",
    ),
    (
        "EventScoringEvaluated",
        EVENT_SCORING_EVALUATED,
        "scoring.evaluated",
    ),
    (
        "EventRuleTriggered",
        EVENT_RULE_TRIGGERED,
        "rule.hard.triggered",
    ),
    (
        "EventVerdictAssigned",
        EVENT_VERDICT_ASSIGNED,
        "verdict.assigned",
    ),
    (
        "EventVerdictChanged",
        EVENT_VERDICT_CHANGED,
        "verdict.changed",
    ),
    ("EventEnfAllow", EVENT_ENF_ALLOW, "enforcement.allow"),
    (
        "EventEnfChallengeIssued",
        EVENT_ENF_CHALLENGE_ISSUED,
        "enforcement.challenge.issued",
    ),
    ("EventEnfDeny", EVENT_ENF_DENY, "enforcement.deny"),
    ("EventEnfTarpit", EVENT_ENF_TARPIT, "enforcement.tarpit"),
    (
        "EventEnfRateLimit",
        EVENT_ENF_RATE_LIMIT,
        "enforcement.ratelimit.applied",
    ),
    ("EventTokenIssued", EVENT_TOKEN_ISSUED, "token.issued"),
    (
        "EventTokenBindingMismatch",
        EVENT_TOKEN_BINDING_MISMATCH,
        "token.binding_mismatch",
    ),
    ("EventTokenBadSig", EVENT_TOKEN_BAD_SIG, "token.bad_sig"),
    ("EventBeaconReplay", EVENT_BEACON_REPLAY, "beacon.replay"),
    ("EventSmuggle", EVENT_SMUGGLE, "net.smuggle"),
    (
        "EventOriginDirectHit",
        EVENT_ORIGIN_DIRECT_HIT,
        "origin.direct_hit",
    ),
    (
        "EventOriginLeakStripped",
        EVENT_ORIGIN_LEAK_STRIPPED,
        "origin.leak_stripped",
    ),
    (
        "EventUpgradeNoVerdict",
        EVENT_UPGRADE_NO_VERDICT,
        "upgrade.no_prior_verdict",
    ),
    (
        "EventInjectNoCallback",
        EVENT_INJECT_NO_CALLBACK,
        "inject.no_callback",
    ),
    (
        "EventHdrSpoofedForwarded",
        EVENT_HDR_SPOOFED_FORWARDED,
        "hdr.spoofed_forwarded",
    ),
    (
        "EventHdrInternalIngress",
        EVENT_HDR_INTERNAL_INGRESS,
        "hdr.internal_header_ingress",
    ),
    ("EventNetProxyHop", EVENT_NET_PROXY_HOP, "net.proxy.hop"),
    (
        "EventNetClientIPSpoof",
        EVENT_NET_CLIENT_IP_SPOOF,
        "net.proxy.client_ip_spoof",
    ),
    (
        "EventNetXFFMultiHop",
        EVENT_NET_XFF_MULTI_HOP,
        "net.proxy.xff_multi_hop",
    ),
    (
        "EventNetForwardedPrivate",
        EVENT_NET_FORWARDED_PRIVATE,
        "net.proxy.forwarded_private",
    ),
    (
        "EventNetAnonChain",
        EVENT_NET_ANON_CHAIN,
        "net.proxy.anon_chain",
    ),
    (
        "EventNetVPNWebRTC",
        EVENT_NET_VPN_WEBRTC,
        "net.vpn.webrtc_leak",
    ),
    (
        "EventNetTorCircuit",
        EVENT_NET_TOR_CIRCUIT,
        "net.tor.circuit",
    ),
    ("EventNetTorExit", EVENT_NET_TOR_EXIT, "net.tor.exit"),
    (
        "EventNetIPDatacenter",
        EVENT_NET_IP_DATACENTER,
        "net.ip.datacenter",
    ),
    (
        "EventNetIPProxyVPN",
        EVENT_NET_IP_PROXY_VPN,
        "net.ip.proxy_vpn",
    ),
    (
        "EventNetTCPNotObserved",
        EVENT_NET_TCP_NOT_OBSERVED,
        "net.tcp.not_observed",
    ),
    (
        "EventNetTCPFingerprint",
        EVENT_NET_TCP_FINGERPRINT,
        "net.tcp.fingerprint",
    ),
    (
        "EventNetTCPTTLOS",
        EVENT_NET_TCP_TTL_OS,
        "net.tcp.ttl_os_mismatch",
    ),
    (
        "EventNetProxyRotation",
        EVENT_NET_PROXY_ROTATION,
        "net.correlation.proxy_rotation",
    ),
    (
        "EventNetFPChurn",
        EVENT_NET_FP_CHURN,
        "net.correlation.fp_churn",
    ),
    (
        "EventNetSharedFP",
        EVENT_NET_SHARED_FP,
        "net.correlation.shared_fingerprint",
    ),
    ("EventNetIPHop", EVENT_NET_IP_HOP, "net.traffic.ip_hop"),
    (
        "EventReconProbing",
        EVENT_RECON_PROBING,
        "recon.decision_probing",
    ),
    (
        "EventAbuseH2DoS",
        EVENT_ABUSE_H2_DOS,
        "abuse.h2dos.detected",
    ),
    (
        "EventUpstreamForwarded",
        EVENT_UPSTREAM_FORWARDED,
        "upstream.request.forwarded",
    ),
    ("EventUpstreamError", EVENT_UPSTREAM_ERROR, "upstream.error"),
    (
        "EventAgentVerified",
        EVENT_AGENT_VERIFIED,
        "agent.signature.verified",
    ),
    (
        "EventAgentForged",
        EVENT_AGENT_FORGED,
        "agent.signature.forged",
    ),
    (
        "EventAgentUnknown",
        EVENT_AGENT_UNKNOWN,
        "agent.signature.unknown",
    ),
    (
        "EventPATVerified",
        EVENT_PAT_VERIFIED,
        "privacypass.token.verified",
    ),
    (
        "EventWebAuthnVerified",
        EVENT_WEBAUTHN_VERIFIED,
        "webauthn.assertion.verified",
    ),
    (
        "EventResponseEgress",
        EVENT_RESPONSE_EGRESS,
        "response.egress",
    ),
    ("EventFailOpen", EVENT_FAIL_OPEN, "failopen.triggered"),
    ("EventFailClosed", EVENT_FAIL_CLOSED, "failclosed.triggered"),
    ("EventAdminAccess", EVENT_ADMIN_ACCESS, "admin.access"),
    (
        "EventAdminAuthFail",
        EVENT_ADMIN_AUTH_FAIL,
        "admin.auth_fail",
    ),
    (
        "EventApprovalRequested",
        EVENT_APPROVAL_REQUESTED,
        "approval.requested",
    ),
    (
        "EventApprovalGranted",
        EVENT_APPROVAL_GRANTED,
        "approval.granted",
    ),
    ("EventBanApplied", EVENT_BAN_APPLIED, "ban.applied"),
    ("EventBanEnforced", EVENT_BAN_ENFORCED, "ban.enforced"),
    ("EventBanExpired", EVENT_BAN_EXPIRED, "ban.expired"),
    ("EventBanLifted", EVENT_BAN_LIFTED, "ban.lifted"),
    (
        "EventRateSoftExceeded",
        EVENT_RATE_SOFT_EXCEEDED,
        "ratelimit.soft_exceeded",
    ),
    (
        "EventRateHardExceeded",
        EVENT_RATE_HARD_EXCEEDED,
        "ratelimit.hard_exceeded",
    ),
    ("EventConfigChanged", EVENT_CONFIG_CHANGED, "config.changed"),
    ("EventKeyRotated", EVENT_KEY_ROTATED, "key.rotated"),
    (
        "EventOperatorOverride",
        EVENT_OPERATOR_OVERRIDE,
        "operator.override",
    ),
    (
        "EventErasureRequested",
        EVENT_ERASURE_REQUESTED,
        "erasure.requested",
    ),
    (
        "EventErasureCompleted",
        EVENT_ERASURE_COMPLETED,
        "erasure.completed",
    ),
    (
        "EventIntegrityPassed",
        EVENT_INTEGRITY_PASSED,
        "audit.integrity.check.passed",
    ),
    (
        "EventIntegrityViolation",
        EVENT_INTEGRITY_VIOLATION,
        "audit.integrity.violation.detected",
    ),
    (
        "EventInstanceStartup",
        EVENT_INSTANCE_STARTUP,
        "instance.startup",
    ),
    (
        "EventInstanceShutdown",
        EVENT_INSTANCE_SHUTDOWN,
        "instance.shutdown",
    ),
];

#[test]
fn event_taxonomy_matches_go_names_and_values() {
    let mut names = BTreeSet::new();
    let mut values = BTreeSet::new();
    for (go, rust, frozen) in GO_TAXONOMY {
        assert_eq!(rust, frozen, "{go} drifted from the Go oracle");
        assert!(go.starts_with("Event"), "{go}");
        assert!(names.insert(go), "duplicate Go name {go}");
        assert!(values.insert(rust), "duplicate event type {rust}");
        assert!(
            !rust.is_empty()
                && rust
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._".contains(&b)),
            "{go} is not a dotted lowercase code: {rust}"
        );
    }
    assert_eq!(values.len(), 74);
    assert_eq!(EVENT_SCHEMA_VERSION, "1.0.0");
}

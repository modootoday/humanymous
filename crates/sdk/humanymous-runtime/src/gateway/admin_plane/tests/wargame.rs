//! Ports of the admin HTTP rounds of legacy
//! internal/gate/wargame_escalation_r81_test.go (R111, R112, R114-R116) and
//! wargame_escalation_r121_test.go (R133-R140, R142-R146) at baseline 981d25c,
//! through the same served loopback stack as the parent module.
//!
//! Not ported here: R113 (the keys route is not served), R131 (audit witness
//! key projection), R132 (an approved erasure answers 409 instead of
//! scheduling, so there is no hold queue to cancel) and R141 (the policy route
//! is not served).
use super::*;

/// Go `maxBanDurationSec` (400 days).
const MAX_BAN_DURATION_SEC: u64 = 400 * 24 * 3600;

// --- wargame_escalation_r81_test.go ----------------------------------------

#[tokio::test]
async fn r111_unauth_admin_whoami_404() {
    let s = stack().await;
    let w = s.admin_do("", "GET", "whoami", "").await;
    assert_eq!(w.status, 404, "unauth whoami: {}", w.body);
    assert!(s.journal.has("admin.auth_fail"));
}

#[tokio::test]
async fn r112_auditor_whoami_ok() {
    let s = stack().await;
    let w = s.admin_do(s.token("auditor"), "GET", "whoami", "").await;
    assert_eq!(w.status, 200, "{}", w.body);
    assert!(w.body.contains("auditor"), "{}", w.body);
}

#[tokio::test]
async fn r114_checkpoints_auth_required() {
    let s = stack().await;
    assert_eq!(s.admin_do("", "GET", "checkpoints", "").await.status, 404);
}

#[tokio::test]
async fn r115_metrics_auth_required() {
    let s = stack().await;
    assert_eq!(s.admin_do("", "GET", "metrics", "").await.status, 404);
}

#[tokio::test]
async fn r116_operator_cannot_list_as_unscoped() {
    let s = stack().await;
    let w = s.admin_do("not-a-real-token", "GET", "bans", "").await;
    assert_eq!(w.status, 404, "bad token: {}", w.body);
    assert!(!s.journal.has("admin.access"));
}

// --- wargame_escalation_r121_test.go ---------------------------------------

#[tokio::test]
async fn r133_auditor_cannot_erasure_request() {
    let s = stack().await;
    let w = s
        .admin_do(
            s.token("auditor"),
            "POST",
            "erasure",
            r#"{"Subject":"x","LegalBasis":"y"}"#,
        )
        .await;
    assert_eq!(w.status, 403, "{}", w.body);
    assert!(s.plane.approvals.pending(s.now()).is_empty());
}

#[tokio::test]
async fn r134_auditor_cannot_killswitch() {
    let s = stack().await;
    let w = s
        .admin_do(s.token("auditor"), "POST", "killswitch", r#"{"On":true}"#)
        .await;
    assert_eq!(w.status, 403, "{}", w.body);
    assert!(s.plane.approvals.pending(s.now()).is_empty());
}

#[tokio::test]
async fn r135_auditor_cannot_bulk_ban() {
    let s = stack().await;
    let w = s
        .admin_do(
            s.token("auditor"),
            "POST",
            "bans/bulk",
            r#"{"Keys":["ip:203.0.113.135"],"DurationSec":60}"#,
        )
        .await;
    assert_eq!(w.status, 403, "{}", w.body);
    assert!(s.banned("ip:203.0.113.135").is_none());
}

#[tokio::test]
async fn r136_bulk_rejects_permanent_duration() {
    let s = stack().await;
    let w = s
        .admin_do(
            s.token("operator"),
            "POST",
            "bans/bulk",
            r#"{"Keys":["ip:203.0.113.136"],"DurationSec":0}"#,
        )
        .await;
    assert_eq!(w.status, 400, "{}", w.body);
    assert!(s.banned("ip:203.0.113.136").is_none());
}

#[tokio::test]
async fn r137_bulk_rejects_over_cap_duration() {
    let s = stack().await;
    let body = format!(r#"{{"Keys":["ip:203.0.113.137"],"DurationSec":{MAX_BAN_DURATION_SEC}}}"#);
    let w = s
        .admin_do(s.token("operator"), "POST", "bans/bulk", &body)
        .await;
    assert_eq!(w.status, 400, "{}", w.body);
    assert!(s.banned("ip:203.0.113.137").is_none());
}

#[tokio::test]
async fn r138_bulk_skips_cidr_keys() {
    let s = stack().await;
    let body = r#"{"Keys":["cidr:198.51.100.0/24","ip:203.0.113.138"],"DurationSec":3600,"Reason":"r138"}"#;
    let w = s
        .admin_do(s.token("operator"), "POST", "bans/bulk", body)
        .await;
    assert_eq!(w.status, 200, "{}", w.body);
    assert!(
        s.banned("cidr:198.51.100.0/24").is_none(),
        "CIDR must be skipped in bulk (needs dual-control path)"
    );
    assert!(
        s.banned("ip:203.0.113.138").is_some(),
        "valid IP must apply"
    );
}

#[tokio::test]
async fn r139_single_cidr_still_dual_control_http() {
    let s = stack().await;
    let w = s
        .admin_do(
            s.token("operator"),
            "POST",
            "bans",
            r#"{"Key":"cidr:198.51.100.0/24","DurationSec":60,"Reason":"r139"}"#,
        )
        .await;
    assert_eq!(w.status, 200, "{}", w.body);
    assert!(w.body.contains("pending"), "{}", w.body);
    assert!(s.banned("cidr:198.51.100.0/24").is_none());
}

#[tokio::test]
async fn r140_ipv6_ban_http_accepts() {
    let s = stack().await;
    let w = s
        .admin_do(
            s.token("operator"),
            "POST",
            "bans",
            r#"{"Key":"ip:2001:db8::9","DurationSec":60,"Reason":"r140"}"#,
        )
        .await;
    assert_eq!(w.status, 200, "IPv6 ban: {}", w.body);
    assert!(s.banned("ip:2001:db8::9").is_some());
}

#[tokio::test]
async fn r142_policy_has_no_write_verb() {
    let s = stack().await;
    let w = s
        .admin_do(s.token("operator"), "POST", "policy", r#"{"routes":{}}"#)
        .await;
    assert_eq!(w.status, 404, "POST policy: {}", w.body);
}

#[tokio::test]
async fn r143_proof_endpoint_auth_required() {
    let s = stack().await;
    assert_eq!(s.admin_do("", "GET", "proof", "").await.status, 404);
}

#[tokio::test]
async fn r144_incidents_auth_required() {
    let s = stack().await;
    assert_eq!(s.admin_do("", "GET", "incidents/x", "").await.status, 404);
}

#[tokio::test]
async fn r145_approvals_list_auth_required() {
    let s = stack().await;
    assert_eq!(s.admin_do("", "GET", "approvals", "").await.status, 404);
}

/// Go accepts 404 or 200 here because its console page is served without
/// authentication; this plane serves no console, so it is 404.
#[tokio::test]
async fn r146_console_unauth_404() {
    let s = stack().await;
    let w = s.admin_do("", "GET", "console", "").await;
    assert!(matches!(w.status, 404 | 200), "unexpected {}", w.status);
    assert_eq!(w.status, 404);
}

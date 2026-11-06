//! One mapping authority for score-exempt network residuals and audit events.
use humanymous_signals::observation::Signal;

const EVENTS: &[(&str, &str)] = &[
    ("l5.header.proxy_hop", "net.proxy.hop"),
    ("l5.header.client_ip_spoof", "net.proxy.client_ip_spoof"),
    ("l5.header.xff_multi_hop", "net.proxy.xff_multi_hop"),
    ("l5.header.forwarded_private", "net.proxy.forwarded_private"),
    ("l5.proxy.vpn_webrtc_leak", "net.vpn.webrtc_leak"),
    ("l5.proxy.tor_circuit", "net.tor.circuit"),
    ("l5.proxy.anon_chain", "net.proxy.anon_chain"),
    ("l5.ip.datacenter_asn", "net.ip.datacenter"),
    ("l5.ip.proxy_vpn_tor", "net.ip.proxy_vpn"),
    ("l5.ip.tor_exit", "net.tor.exit"),
    ("l5.tcp.not_observed", "net.tcp.not_observed"),
    ("l5.tcp.ttl", "net.tcp.fingerprint"),
    ("l5.tcp.mss", "net.tcp.fingerprint"),
    ("l5.tcp.window", "net.tcp.fingerprint"),
    ("l5.tcp.ttl_os", "net.tcp.ttl_os_mismatch"),
    (
        "l5.correlation.proxy_rotation",
        "net.correlation.proxy_rotation",
    ),
    ("l5.correlation.fp_churn_proxy", "net.correlation.fp_churn"),
    (
        "l5.correlation.shared_fingerprint",
        "net.correlation.shared_fingerprint",
    ),
    ("l5.traffic.ip_hop", "net.traffic.ip_hop"),
];

/// Administrative audit event, or empty for a non-residual signal.
pub fn audit_event_for(id: &str) -> &'static str {
    EVENTS
        .iter()
        .find(|entry| entry.0 == id)
        .map(|entry| entry.1)
        .unwrap_or("")
}

/// Whether the frozen network policy classifies the ID as score-exempt.
pub fn is_score_exempt(id: &str) -> bool {
    !audit_event_for(id).is_empty()
}

/// Preserve residual input order without copying individual observations.
pub fn signals(input: &[Signal]) -> impl Iterator<Item = &Signal> {
    input.iter().filter(|signal| is_score_exempt(&signal.id))
}

/// Whether an admin-relevant residual is BOT or SUSPICIOUS.
pub fn has_actionable_residual(input: &[Signal]) -> bool {
    signals(input).any(|signal| matches!(signal.verdict.as_str(), "BOT" | "SUSPICIOUS"))
}

/// Minimal audit payload, independent of storage/chain implementation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AuditRecord {
    /// Stable event type.
    pub event_type: String,
    /// Source signal ID.
    pub signal_id: String,
    /// Wire verdict, including unknown observation markers.
    pub verdict: String,
    /// Notes, falling back to the signal ID.
    pub notes: String,
}

/// One event per mapped residual, including UNKNOWN dark-plane observations.
pub fn audit_records(input: &[Signal]) -> Vec<AuditRecord> {
    signals(input)
        .map(|signal| AuditRecord {
            event_type: audit_event_for(&signal.id).into(),
            signal_id: signal.id.clone(),
            verdict: signal.verdict.as_str().into(),
            notes: if signal.notes.is_empty() {
                signal.id.clone()
            } else {
                signal.notes.clone()
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use humanymous_signals::observation::{Source, Verdict};
    #[test]
    fn observations_are_audited_without_becoming_actionable_or_scored() {
        let signal = Signal::new(
            "l5.tcp.not_observed",
            serde_json::json!(true),
            Verdict::Unknown,
            1.0,
            Source::Server,
            "",
        );
        assert!(!has_actionable_residual(std::slice::from_ref(&signal)));
        let records = audit_records(&[signal]);
        assert_eq!(records[0].notes, "l5.tcp.not_observed");
        assert_eq!(records[0].event_type, "net.tcp.not_observed");
        assert_eq!(audit_event_for("l5.rit.absent"), "");
        assert_eq!(EVENTS.len(), 19);
    }
}

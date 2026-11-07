//! Optional TCP/SYN observations remain score-exempt administrative evidence.
use humanymous_core::signals::{
    observation::{Signal, Source, Verdict},
    wire::simple_lowercase,
};
use serde::{Deserialize, Serialize};

/// Connection-level TCP evidence, when a real capture source is wired.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Observation {
    /// A SYN/PROXY TLV or equivalent capture source supplied evidence.
    pub observed: bool,
    /// Observed IP TTL.
    pub ttl: u8,
    /// MSS option, zero if unknown.
    pub mss: u16,
    /// Initial window, zero if unknown.
    pub window: u16,
    /// Window scale, -1 for absent when captured.
    pub window_scale: i8,
    /// SACK permitted.
    pub sack: bool,
    /// Coarse explicitly supplied OS family or empty for TTL fallback.
    pub os_family: String,
}

/// Frozen best-effort TTL classification, not an OS identity assertion.
pub fn guess_os_from_ttl(ttl: u8) -> &'static str {
    match ttl {
        0 => "unknown",
        1..=64 => "linux",
        65..=128 => "windows",
        _ => "unknown",
    }
}

fn ua_os_family(ua: &str) -> &'static str {
    let ua = simple_lowercase(ua);
    if ua.contains("windows") {
        "windows"
    } else if ua.contains("android") {
        "linux"
    } else if ["iphone", "ipad", "mac os"]
        .iter()
        .any(|token| ua.contains(token))
    {
        "macos"
    } else if ua.contains("linux") || ua.contains("cros") {
        "linux"
    } else {
        "unknown"
    }
}

/// Emit the frozen observation markers; these do not add risk score.
pub fn signals(observation: &Observation, user_agent: &str) -> Vec<Signal> {
    let mut out = Vec::new();
    let mut add = |id, value, verdict, notes| {
        out.push(Signal::new(id, value, verdict, 1.0, Source::Server, notes))
    };
    if !observation.observed {
        add(
            "l5.tcp.not_observed",
            serde_json::json!(true),
            Verdict::Unknown,
            "no TCP SYN/PROXY-TLV capture — L4 plane inactive (wire eBPF/PROXY TLV for full residual)",
        );
        return out;
    }
    let os = if observation.os_family.is_empty() && observation.ttl > 0 {
        guess_os_from_ttl(observation.ttl)
    } else {
        &observation.os_family
    };
    add(
        "l5.tcp.ttl",
        serde_json::json!(observation.ttl),
        Verdict::Unknown,
        "observed IP TTL",
    );
    if observation.mss > 0 {
        add(
            "l5.tcp.mss",
            serde_json::json!(observation.mss),
            Verdict::Unknown,
            "TCP MSS option",
        );
    }
    if observation.window > 0 {
        add(
            "l5.tcp.window",
            serde_json::json!(observation.window),
            Verdict::Unknown,
            "TCP initial window",
        );
    }
    let claimed = ua_os_family(user_agent);
    if !os.is_empty() && os != "unknown" && claimed != "unknown" && os != claimed {
        add(
            "l5.tcp.ttl_os",
            serde_json::json!({"tcp":os,"ua":claimed}),
            Verdict::Suspicious,
            "TCP TTL/OS family vs User-Agent OS mismatch",
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_absence_and_os_mismatch_remain_zero_score() {
        let absent = signals(&Observation::default(), "Windows");
        assert_eq!(absent[0].id, "l5.tcp.not_observed");
        let observed = signals(
            &Observation {
                observed: true,
                ttl: 64,
                mss: 1460,
                window: 65535,
                ..Default::default()
            },
            "Windows",
        );
        assert!(observed.iter().any(|s| s.id == "l5.tcp.ttl_os"));
        assert!(
            observed
                .iter()
                .all(|s| s.score == 0.0 && crate::residual::is_score_exempt(&s.id))
        );
        assert_eq!(guess_os_from_ttl(0), "unknown");
        assert_eq!(guess_os_from_ttl(128), "windows");
        assert_eq!(guess_os_from_ttl(129), "unknown");
    }
}

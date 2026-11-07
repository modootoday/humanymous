use humanymous_core::signals::wire::simple_lowercase as lowercase;
use serde::{Deserialize, Serialize};

/// Request header evidence supplied by the capture/HTTP adapter. Reliability
/// flags must describe actual capture provenance, not inferred browser identity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HeaderInfo {
    /// Request method.
    pub method: String,
    /// HTTP version as 10, 11, 20 or 30.
    pub version: String,
    /// HTTP/2 or later semantics.
    pub is_h2: bool,
    /// Captured names; wire order only if order_reliable.
    pub names: Vec<String>,
    /// Original casing survived raw capture.
    pub casing_reliable: bool,
    /// Original order survived raw capture.
    pub order_reliable: bool,
    /// Cookie header presence.
    pub has_cookie: bool,
    /// Referer header presence.
    pub has_referer: bool,
    /// Accept-Language value.
    pub accept_language: String,
    /// Accept-Encoding value.
    pub accept_encoding: String,
    /// Observed cookie names.
    pub cookie_names: Vec<String>,
    /// Claimed User-Agent.
    pub user_agent: String,
    /// Via value.
    pub via: String,
    /// Proxy-Connection value.
    pub proxy_connection: String,
    /// X-Cache request value.
    pub x_cache: String,
    /// X-Squid-Error value.
    pub x_squid_error: String,
    /// Full X-Forwarded-For value, never a trusted identity by itself.
    pub x_forwarded_for: String,
    /// RFC 7239 Forwarded value.
    pub forwarded: String,
    /// Untrusted CF-Connecting-IP value.
    pub cf_connecting_ip: String,
    /// Untrusted True-Client-IP value.
    pub true_client_ip: String,
    /// Untrusted X-Client-IP value.
    pub x_client_ip: String,
    /// Untrusted X-Original-Forwarded-For value.
    pub x_original_forwarded_for: String,
    /// X-BlueCoat-Via value.
    pub x_blue_coat_via: String,
}

// Unicode White_Space recognized by the frozen Go strings.TrimSpace.
fn trim_space(value: &str) -> &str {
    value.trim_matches(|c| matches!(c, '\u{9}'..='\u{d}' | ' ' | '\u{85}' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}'))
}

impl HeaderInfo {
    /// Any sec-fetch-* name, with the frozen simple Unicode lowercase behavior.
    pub fn sec_fetch_present(&self) -> bool {
        self.names
            .iter()
            .any(|name| lowercase(name).starts_with("sec-fetch-"))
    }

    /// sec-ch-ua presence with Go EqualFold semantics, including long-s folding.
    pub fn sec_ch_ua_present(&self) -> bool {
        self.names.iter().any(|name| {
            name.chars()
                .map(|c| {
                    if c == '\u{17f}' {
                        's'
                    } else {
                        c.to_ascii_lowercase()
                    }
                })
                .eq("sec-ch-ua".chars())
        })
    }

    /// Uppercase anomaly only when HTTP/2 and raw casing provenance are known.
    pub fn has_uppercase_in_h2(&self) -> bool {
        self.is_h2 && self.casing_reliable && self.names.iter().any(|name| *name != lowercase(name))
    }

    /// Legacy substring observation for modern Chrome compression capability.
    pub fn chrome_accept_encoding_ok(&self) -> bool {
        lowercase(&self.accept_encoding).contains("zstd")
    }

    /// Lowercased names in observed order; callers must check order_reliable
    /// before treating this as on-wire ordering evidence.
    pub fn order(&self) -> Vec<String> {
        self.names.iter().map(|name| lowercase(name)).collect()
    }

    /// First matching forward-proxy residual label, preserving precedence.
    pub fn proxy_hop_kind(&self) -> &'static str {
        let via = lowercase(&self.via);
        if !via.is_empty() {
            return if via.contains("squid") {
                "via-squid"
            } else {
                "via"
            };
        }
        for (value, label) in [
            (&self.x_squid_error, "x-squid-error"),
            (&self.x_blue_coat_via, "bluecoat-via"),
            (&self.x_cache, "x-cache-request"),
            (&self.proxy_connection, "proxy-connection"),
        ] {
            if !value.is_empty() {
                return label;
            }
        }
        let forwarded = lowercase(&self.forwarded);
        if !forwarded.is_empty()
            && (forwarded.matches("for=").count() >= 2 || forwarded.contains("by="))
        {
            "forwarded-multi"
        } else {
            ""
        }
    }

    /// First nonblank client identity laundering header. These values never
    /// establish trusted proxy identity or local management access.
    pub fn client_ip_spoof_kind(&self) -> &'static str {
        for (value, label) in [
            (&self.cf_connecting_ip, "cf-connecting-ip"),
            (&self.true_client_ip, "true-client-ip"),
            (&self.x_client_ip, "x-client-ip"),
            (&self.x_original_forwarded_for, "x-original-forwarded-for"),
        ] {
            if !trim_space(value).is_empty() {
                return label;
            }
        }
        ""
    }

    /// Comma-separated hop count, preserving empty chain members as observed.
    pub fn xff_hop_count(&self) -> usize {
        let value = trim_space(&self.x_forwarded_for);
        if value.is_empty() {
            0
        } else {
            1 + value.bytes().filter(|&b| b == b',').count()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalized_maps_cannot_supply_casing_evidence_or_trusted_identity() {
        let mut headers = HeaderInfo {
            is_h2: true,
            names: vec![
                "User-Agent".into(),
                "SEC-FETCH-SITE".into(),
                "ſec-ch-ua".into(),
            ],
            cf_connecting_ip: "\u{a0}".into(),
            true_client_ip: "127.0.0.1".into(),
            ..Default::default()
        };
        assert!(!headers.has_uppercase_in_h2());
        assert!(headers.sec_fetch_present());
        assert!(headers.sec_ch_ua_present());
        assert_eq!(headers.client_ip_spoof_kind(), "true-client-ip");
        headers.casing_reliable = true;
        assert!(headers.has_uppercase_in_h2());
    }
    #[test]
    fn proxy_precedence_and_empty_chain_members_are_preserved() {
        let mut headers = HeaderInfo {
            via: "1.1 SQUID".into(),
            x_squid_error: "ERR".into(),
            x_forwarded_for: " , , ".into(),
            ..Default::default()
        };
        assert_eq!(headers.proxy_hop_kind(), "via-squid");
        assert_eq!(headers.xff_hop_count(), 3);
        headers.via.clear();
        assert_eq!(headers.proxy_hop_kind(), "x-squid-error");
    }
}

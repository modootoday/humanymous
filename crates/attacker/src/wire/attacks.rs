//! Defense-only red-team attack catalog and `-attack` selector (Go parity).
//!
//! This is the Rust port of the Go `cmd/redteam` `-attack` list. Every entry
//! targets ONLY this deployment's owned lab runtime; there is no arbitrary
//! target field and no stealth capability beyond what the Go oracle used to
//! test its own detector. Entries are descriptors: a transport (which wire
//! builder presents the fingerprint) plus the residual tell the Blue engine is
//! expected to catch. They never embed or alter detection logic — the expected
//! tells are documentation of the attack intent, not scoring predicates.
use crate::wire::{clienthello, h2};
use serde_json::{Value, json};

/// Which wire builder presents the attack's fingerprint to the owned runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transport {
    /// A constructed TLS ClientHello (optionally a single-extension mutation).
    ClientHello(clienthello::Profile, Option<clienthello::Mutation>),
    /// A raw HTTP/2 client with a controlled frame profile.
    RawH2(h2::H2Profile),
    /// An HTTP/1.1 request plan (headers/body/RIT), no TLS/h2 control.
    Http1,
}

/// One catalog entry. `bounded_requests` is the explicit cap for volume
/// attacks so a flood/distributed run can never be unbounded or non-lab.
#[derive(Clone, Copy, Debug)]
pub struct AttackSpec {
    /// Stable `-attack` selector name (matches the Go CLI).
    pub name: &'static str,
    /// Cost ladder / control tier.
    pub tier: &'static str,
    /// Attack family grouping.
    pub class: &'static str,
    /// Wire transport that carries the fingerprint.
    pub transport: Transport,
    /// The residual tell / hard rule the Blue engine must catch (intent only).
    pub expected: &'static str,
    /// Explicit request cap for bounded volume attacks (lab-only).
    pub bounded_requests: Option<u32>,
    /// Short provenance note tying the entry to the Go oracle behaviour.
    pub note: &'static str,
}

use Transport::{ClientHello, Http1, RawH2};
use clienthello::{Mutation, Profile};
use h2::H2Profile;

/// The full catalog, one entry per Go `-attack` name.
pub const CATALOG: &[AttackSpec] = &[
    // --- TLS parrot / rotation family (ClientHello builder) ---
    AttackSpec {
        name: "tls-static",
        tier: "T1",
        class: "tls",
        transport: ClientHello(Profile::Chrome, None),
        expected: "l5.traffic.tls_no_permutation -> HR-14",
        bounded_requests: Some(4),
        note: "static Chrome parrot: identical ClientHello every request",
    },
    AttackSpec {
        name: "tls-rotate",
        tier: "T1",
        class: "tls",
        transport: ClientHello(Profile::Chrome, None),
        expected: "l5.traffic.engine_rotation -> HR-14",
        bounded_requests: Some(2),
        note: "two TLS stacks (Chrome then Firefox) in one session",
    },
    AttackSpec {
        name: "ja4-churn",
        tier: "T1",
        class: "tls",
        transport: ClientHello(Profile::Chrome, None),
        expected: "l5.traffic.ja4_rotation -> HR-14",
        bounded_requests: Some(3),
        note: "three distinct TLS fingerprints in one session",
    },
    AttackSpec {
        name: "multi-axis-rotate",
        tier: "T1",
        class: "tls",
        transport: ClientHello(Profile::Chrome, None),
        expected: "ua_rotation + ja4_rotation -> HR-15",
        bounded_requests: Some(2),
        note: "rotate User-Agent AND TLS engine together",
    },
    AttackSpec {
        name: "ua-rotate",
        tier: "T1",
        class: "tls",
        transport: ClientHello(Profile::Chrome, None),
        expected: "l5.traffic.ua_rotation",
        bounded_requests: Some(2),
        note: "two User-Agents, same TLS stack",
    },
    AttackSpec {
        name: "grease-absent-js",
        tier: "T0",
        class: "tls",
        transport: ClientHello(Profile::Stock, None),
        expected: "l5.tls.grease_absent + x.ua_vs_ja4",
        bounded_requests: None,
        note: "no-GREASE Go stack under a Chrome UA that runs JS",
    },
    AttackSpec {
        name: "alps-absent",
        tier: "runtime",
        class: "tls",
        transport: ClientHello(Profile::Chrome, Some(Mutation::StripAlps)),
        expected: "l5.tls.alps_absent -> HR-24",
        bounded_requests: None,
        note: "Chrome hello minus ALPS over h2",
    },
    AttackSpec {
        name: "ech-absent",
        tier: "runtime",
        class: "tls",
        transport: ClientHello(Profile::Chrome, Some(Mutation::StripEch)),
        expected: "l5.tls.ech_absent -> HR-24",
        bounded_requests: None,
        note: "pre-ECH parrot under a Chrome UA",
    },
    AttackSpec {
        name: "cert-compression-absent",
        tier: "runtime",
        class: "tls",
        transport: ClientHello(Profile::Chrome, Some(Mutation::StripCertCompression)),
        expected: "l5.tls.cert_compression_absent -> HR-24",
        bounded_requests: None,
        note: "Chrome hello minus compress_certificate (ext 27)",
    },
    AttackSpec {
        name: "pq-absent",
        tier: "runtime",
        class: "tls",
        transport: ClientHello(Profile::Chrome, Some(Mutation::PreQuantum)),
        expected: "l5.tls.pq_keyshare -> HR-24",
        bounded_requests: None,
        note: "pre-PQ groups while the UA claims Chrome 131",
    },
    // --- Raw HTTP/2 protocol-split family (RawH2 builder) ---
    AttackSpec {
        name: "h2-settings-split",
        tier: "runtime",
        class: "h2",
        transport: RawH2(H2Profile::SettingsSplit),
        expected: "l5.http2.browser_settings_atypical",
        bounded_requests: None,
        note: "Chrome pseudo-order, library SETTINGS, no window",
    },
    AttackSpec {
        name: "h2-flow-control",
        tier: "runtime",
        class: "h2",
        transport: RawH2(H2Profile::FlowControlSplit),
        expected: "l5.http2.flow_control_atypical",
        bounded_requests: None,
        note: "coherent SETTINGS but a 1 GiB connection window",
    },
    AttackSpec {
        name: "h2-priority-absent",
        tier: "runtime",
        class: "h2",
        transport: RawH2(H2Profile::PriorityAbsent),
        expected: "l5.http2.priority_absent",
        bounded_requests: None,
        note: "coherent SETTINGS + real window, no HEADERS priority",
    },
    AttackSpec {
        name: "h2-protocol-split",
        tier: "runtime",
        class: "h2",
        transport: RawH2(H2Profile::SettingsSplit),
        expected: "x.ua_vs_h2 / l5.http2.engine_mismatch",
        bounded_requests: None,
        note: "TLS/JA4 Chrome but a library HTTP/2 frame layout",
    },
    AttackSpec {
        name: "coherent-ceiling",
        tier: "T4",
        class: "ceiling",
        transport: RawH2(H2Profile::Chrome),
        expected: "ALLOW (intentional detection ceiling)",
        bounded_requests: None,
        note: "full coherent Chrome h2 fingerprint; honest ALLOW",
    },
    // --- Header-order / H1 cross-layer family ---
    AttackSpec {
        name: "browser-tls-h1",
        tier: "runtime",
        class: "http",
        transport: Http1,
        expected: "l5.http.browser_tls_over_h1 -> HR-24",
        bounded_requests: None,
        note: "coherent Chrome TLS delivered over HTTP/1.1",
    },
    AttackSpec {
        name: "header-order-split",
        tier: "runtime",
        class: "http",
        transport: Http1,
        expected: "l5.header.order -> HR-24",
        bounded_requests: None,
        note: "user-agent before the sec-ch-ua cluster (non-browser order)",
    },
    // --- Header / RIT cross-check family (HTTP/1.1 plan) ---
    AttackSpec {
        name: "nonbrowser-ua",
        tier: "T0",
        class: "header",
        transport: Http1,
        expected: "x.non_browser_ua + HR-10",
        bounded_requests: None,
        note: "bare HTTP library UA, no browser headers, no JS",
    },
    AttackSpec {
        name: "sec-chua-absent",
        tier: "T0",
        class: "header",
        transport: Http1,
        expected: "x.uach_present -> CHALLENGE",
        bounded_requests: None,
        note: "Chromium UA that omits Sec-CH-UA client hints",
    },
    AttackSpec {
        name: "sec-fetch-absent",
        tier: "T0",
        class: "header",
        transport: Http1,
        expected: "l5.header.sec_fetch_missing",
        bounded_requests: None,
        note: "Chrome UA that omits Sec-Fetch-* metadata",
    },
    AttackSpec {
        name: "rit-replay",
        tier: "T0",
        class: "rit",
        transport: Http1,
        expected: "l5.rit.stale_replay",
        bounded_requests: Some(2),
        note: "replay a previously valid RIT token/counter",
    },
    AttackSpec {
        name: "rit-tamper",
        tier: "T0",
        class: "rit",
        transport: Http1,
        expected: "l5.rit.header_tampered",
        bounded_requests: None,
        note: "sign one body, send a different body",
    },
    AttackSpec {
        name: "rit-absent",
        tier: "T0",
        class: "rit",
        transport: Http1,
        expected: "l5.rit.absent -> HR-17",
        bounded_requests: Some(2),
        note: "never present an anti-tamper token",
    },
    AttackSpec {
        name: "signal-forgery",
        tier: "T2",
        class: "provenance",
        transport: Http1,
        expected: "forged l7.pass/pow.solved inert; must not ALLOW",
        bounded_requests: None,
        note: "client forges server-only trust-upgrade signals",
    },
    // --- Proxy / VPN / Tor / CDN header-spoof family (HR-24 / HR-19) ---
    AttackSpec {
        name: "xff-spoof",
        tier: "T0",
        class: "proxy",
        transport: Http1,
        expected: "l5.header.forwarded_private",
        bounded_requests: None,
        note: "forged private X-Forwarded-For to fake a LAN client",
    },
    AttackSpec {
        name: "squid-forward",
        tier: "T0",
        class: "proxy",
        transport: Http1,
        expected: "l5.header.proxy_hop -> HR-24",
        bounded_requests: None,
        note: "Squid forward-proxy Via / X-Cache residual",
    },
    AttackSpec {
        name: "openvpn-exit",
        tier: "T0",
        class: "proxy",
        transport: Http1,
        expected: "l5.proxy.vpn_webrtc_leak -> HR-24",
        bounded_requests: None,
        note: "VPN exit with a WebRTC public-IP leak",
    },
    AttackSpec {
        name: "wireguard-hop",
        tier: "T0",
        class: "proxy",
        transport: Http1,
        expected: "ip_hop + xff_multi_hop -> HR-24",
        bounded_requests: Some(2),
        note: "WireGuard exit rotation in one session",
    },
    AttackSpec {
        name: "tor-exit",
        tier: "T0",
        class: "proxy",
        transport: Http1,
        expected: "l5.proxy.tor_circuit -> HR-24 / HR-19",
        bounded_requests: Some(4),
        note: "Tor circuit multi-hop + exit-subnet churn",
    },
    AttackSpec {
        name: "anon-proxy-chain",
        tier: "T0",
        class: "proxy",
        transport: Http1,
        expected: "l5.proxy.anon_chain -> HR-24",
        bounded_requests: None,
        note: "4+ hop open-proxy chain with Proxy-Connection",
    },
    AttackSpec {
        name: "elite-anon-proxy",
        tier: "T0",
        class: "proxy",
        transport: Http1,
        expected: "l5.header.proxy_hop (forwarded-multi) -> HR-24",
        bounded_requests: None,
        note: "RFC 7239 Forwarded multi-for, Via stripped",
    },
    AttackSpec {
        name: "cdn-ip-spoof",
        tier: "T0",
        class: "proxy",
        transport: Http1,
        expected: "l5.header.client_ip_spoof -> HR-24",
        bounded_requests: None,
        note: "forged CF-Connecting-IP / True-Client-IP",
    },
    AttackSpec {
        name: "proxy-ua-rotate",
        tier: "T0",
        class: "proxy",
        transport: Http1,
        expected: "ua_rotation + ip_hop -> HR-15 / HR-19",
        bounded_requests: Some(3),
        note: "residential exit rotation + mid-session UA rotate",
    },
    AttackSpec {
        name: "fp-churn-proxy",
        tier: "T0",
        class: "proxy",
        transport: Http1,
        expected: "l5.correlation.fp_churn_proxy -> HR-19",
        bounded_requests: Some(3),
        note: "rotate fingerprintId per exit to dodge HR-19",
    },
    AttackSpec {
        name: "stacked-proxy-vpn",
        tier: "T0",
        class: "proxy",
        transport: Http1,
        expected: "proxy_hop + vpn_webrtc_leak -> HR-24",
        bounded_requests: None,
        note: "Squid hop over a VPN exit with WebRTC leak",
    },
    AttackSpec {
        name: "socks-exit-hop",
        tier: "T0",
        class: "proxy",
        transport: Http1,
        expected: "ip_hop + xff_multi_hop -> HR-24",
        bounded_requests: Some(2),
        note: "SOCKS5/VPN L4 exit hop + multi-hop XFF",
    },
    // --- Bounded volume family (explicit lab caps) ---
    AttackSpec {
        name: "flood",
        tier: "T0",
        class: "volume",
        transport: Http1,
        expected: "l5.abuse.flood -> CHALLENGE + ban ladder",
        bounded_requests: Some(90),
        note: "application-layer flood from one fingerprint (capped)",
    },
    AttackSpec {
        name: "distributed",
        tier: "T0",
        class: "volume",
        transport: Http1,
        expected: "l5.correlation.proxy_rotation -> HR-19",
        bounded_requests: Some(4),
        note: "one fingerprint across rotated proxy subnets (capped)",
    },
    AttackSpec {
        name: "privacy-evasion",
        tier: "T0",
        class: "volume",
        transport: Http1,
        expected: "HR-19 (privacy flag must not disarm)",
        bounded_requests: Some(4),
        note: "distributed rotation that also claims ad-block/GPC",
    },
    // --- Proof-of-work active-defense family ---
    AttackSpec {
        name: "pow-fast-solve",
        tier: "T2",
        class: "pow",
        transport: Http1,
        expected: "l7.pow.too_fast -> HR-19",
        bounded_requests: None,
        note: "native PoW solve faster than any browser JS solver",
    },
    AttackSpec {
        name: "pow-launder",
        tier: "T2",
        class: "pow",
        transport: Http1,
        expected: "too-fast tell must persist; no laundered ALLOW",
        bounded_requests: None,
        note: "delayed resubmit of a natively-solved nonce",
    },
];

/// Resolve a `-attack` selector name to its catalog entry.
pub fn select(name: &str) -> Option<&'static AttackSpec> {
    CATALOG.iter().find(|spec| spec.name == name)
}

impl AttackSpec {
    /// Emit the defense-only plan artifact: what this attack would present on
    /// the wire to the owned runtime, with the fingerprint computed by the
    /// project's own observers where a transport carries one.
    pub fn plan_json(&self) -> Value {
        let mut plan = json!({
            "attack": self.name,
            "tier": self.tier,
            "class": self.class,
            "expected": self.expected,
            "note": self.note,
            "target": "owned-lab-runtime-only",
        });
        if let Some(cap) = self.bounded_requests {
            plan["boundedRequests"] = json!(cap);
        }
        match self.transport {
            Transport::ClientHello(profile, mutation) => {
                let mut builder = clienthello::ClientHelloBuilder::new(profile);
                if let Some(m) = mutation {
                    builder = builder.mutate(m);
                }
                let hello =
                    humanymous_runtime::network::tls::parse_client_hello(&builder.handshake())
                        .expect("builder emits a parseable ClientHello");
                let (ja3_hash, _) = humanymous_runtime::network::tls::ja3(&hello);
                plan["transport"] = json!("tls-clienthello");
                plan["ja3"] = json!(ja3_hash);
                plan["tlsEngine"] = json!(
                    humanymous_runtime::network::tls::engine_from_client_hello(Some(&hello))
                );
            }
            Transport::RawH2(profile) => {
                let mut capture = humanymous_runtime::capture::http2::Capture::default();
                let request =
                    h2::H2Request::new(profile, "GET", "owned.test", "/api/session").encode();
                capture
                    .feed(&request)
                    .expect("runtime H2 capture accepts the frames");
                let fp = capture.request(1).expect("stream 1 request").fingerprint();
                plan["transport"] = json!("raw-http2");
                plan["h2Akamai"] = json!(fp.akamai());
                plan["h2Engine"] = json!(fp.engine());
            }
            Transport::Http1 => {
                plan["transport"] = json!("http1-request-plan");
            }
        }
        plan
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Every Go `cmd/redteam` `-attack` selector name must be present.
    const GO_ATTACK_NAMES: &[&str] = &[
        "flood",
        "distributed",
        "privacy-evasion",
        "signal-forgery",
        "pow-fast-solve",
        "pow-launder",
        "h2-protocol-split",
        "h2-settings-split",
        "header-order-split",
        "pq-absent",
        "alps-absent",
        "h2-flow-control",
        "h2-priority-absent",
        "browser-tls-h1",
        "cert-compression-absent",
        "ech-absent",
        "nonbrowser-ua",
        "sec-chua-absent",
        "sec-fetch-absent",
        "rit-absent",
        "ja4-churn",
        "multi-axis-rotate",
        "grease-absent-js",
        "coherent-ceiling",
        "xff-spoof",
        "squid-forward",
        "openvpn-exit",
        "wireguard-hop",
        "tor-exit",
        "anon-proxy-chain",
        "elite-anon-proxy",
        "cdn-ip-spoof",
        "proxy-ua-rotate",
        "fp-churn-proxy",
        "stacked-proxy-vpn",
        "socks-exit-hop",
        "tls-static",
        "tls-rotate",
        "ua-rotate",
        "rit-replay",
        "rit-tamper",
    ];

    #[test]
    fn catalog_covers_every_go_attack_name_with_unique_entries() {
        let names: BTreeSet<_> = CATALOG.iter().map(|s| s.name).collect();
        assert_eq!(names.len(), CATALOG.len(), "duplicate attack names");
        for go in GO_ATTACK_NAMES {
            assert!(select(go).is_some(), "missing Go attack: {go}");
        }
        assert_eq!(names.len(), GO_ATTACK_NAMES.len());
    }

    #[test]
    fn selector_rejects_unknown_and_arbitrary_targets() {
        assert!(select("not-an-attack").is_none());
        assert!(select("https://example.invalid").is_none());
    }

    #[test]
    fn volume_attacks_are_always_bounded() {
        for spec in CATALOG {
            if spec.class == "volume" {
                assert!(
                    spec.bounded_requests.is_some(),
                    "{} must be capped",
                    spec.name
                );
            }
        }
    }

    #[test]
    fn every_tls_and_h2_plan_emits_a_wire_fingerprint() {
        for spec in CATALOG {
            let plan = spec.plan_json();
            assert_eq!(plan["target"], "owned-lab-runtime-only");
            match spec.transport {
                Transport::ClientHello(..) => {
                    assert!(plan["ja3"].is_string(), "{} has no ja3", spec.name);
                }
                Transport::RawH2(..) => {
                    assert!(plan["h2Akamai"].is_string(), "{} has no h2 fp", spec.name);
                }
                Transport::Http1 => assert_eq!(plan["transport"], "http1-request-plan"),
            }
        }
    }

    #[test]
    fn mutation_plans_report_the_targeted_tls_residual() {
        // The ALPS/ECH/cert-comp/PQ mutations must each resolve to a chrome-ish
        // engine (the hello is still Chrome-shaped; only one field is dropped).
        for name in [
            "alps-absent",
            "ech-absent",
            "cert-compression-absent",
            "pq-absent",
        ] {
            let plan = select(name).unwrap().plan_json();
            assert_eq!(plan["tlsEngine"], "chrome", "{name}");
        }
    }
}

//! Defense-only red-team attack catalog and `-attack` selector (Go parity).
//!
//! This is the Rust port of the Go `cmd/redteam` `-attack` list (oracle
//! 981d25c). Every entry targets ONLY this deployment's owned lab runtime;
//! there is no arbitrary target field and no stealth capability beyond what
//! the Go oracle used to test its own detector.
//!
//! Each entry is a request plan ported from the Go attack function: the
//! ordered steps (session bootstrap, collects, PoW submits) with the TLS
//! ClientHello preset each step's connection used, the HTTP version and h2
//! frame profile, method and path, the request headers and body the Go code
//! composed, the RIT signing intent and repeat count. Each step's ClientHello
//! and h2 frames are built and run through the runtime's own observers, so the
//! plan carries the fingerprint the detector would see. Nothing here sends a
//! request: values only the live server can supply (session cookie, RIT seed
//! and token, time bucket, PoW challenge and nonce) are `{placeholders}`.
//!
//! Recorded differences from the oracle: Go composed most header sets as Go
//! maps, so its on-wire header order was randomised per request except for
//! `header-order-split` (exact order) and the raw h2 framer (Chrome order);
//! plans list the composition order instead. The PoW native solve loop and
//! the live round-trips are not ported (see the ledger row gaps).
//!
//! The expected tells are documentation of the attack intent, not scoring
//! predicates; nothing here embeds or alters detection logic.
use crate::wire::clienthello::{ClientHelloBuilder, Mutation, Profile};
use crate::wire::h2::{self, H2Profile};
use humanymous_runtime::network::tls::{
    engine_from_client_hello, extension_order_hash, ja3, ja4, parse_client_hello,
};
use serde_json::{Value, json};

/// The SNI / Host / `:authority` every plan step names (reserved `.test`).
pub const LAB_HOST: &str = "owned.test";

/// HTTP framing of one step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Http {
    /// HTTP/1.1 over a hello with ALPN forced to `http/1.1` (Go `do`,
    /// `doStock`); Go wrote these headers in map order.
    H1,
    /// HTTP/1.1 with headers in exactly the listed order (Go `doOrderedH1`).
    H1Ordered,
    /// HTTP/2 with the given frame profile; the hello offers `h2, http/1.1`.
    H2(H2Profile),
}

/// RIT anti-tamper intent of one step. Counters are offsets from the `ritN`
/// the session bootstrap returned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rit {
    /// No RIT headers.
    None,
    /// Sign the sent body with counter `n + k` under the current seed.
    Sign(u64),
    /// Resend the exact token/counter of an earlier `Sign(k)` step.
    Replay(u64),
    /// Sign a different body with counter `n + k`, then send this step's body.
    SignOtherBody(u64, String),
}

/// One request of an attack plan.
#[derive(Clone, Debug)]
pub struct Step {
    /// `session`, `collect` or `pow-submit`.
    pub purpose: &'static str,
    /// ClientHello preset of this step's (fresh) connection.
    pub hello: Profile,
    /// Single-extension deviation from the preset, if any.
    pub mutation: Option<Mutation>,
    /// HTTP framing.
    pub http: Http,
    /// Request method.
    pub method: &'static str,
    /// Request path (oracle `/api/*` contract).
    pub path: &'static str,
    /// Request headers in plan order (see the module note on Go map order).
    pub headers: Vec<(String, String)>,
    /// Request body (may contain `{placeholders}` for server-supplied values).
    pub body: String,
    /// RIT intent.
    pub rit: Rit,
    /// How many times the oracle sent this step (fresh connection each time).
    pub repeat: u32,
    /// Oracle behaviour that is described rather than reproduced.
    pub note: Option<&'static str>,
}

/// One catalog entry. `bounded_requests` is the explicit cap for multi-request
/// attacks (it equals the number of collect requests the plan sends) so a
/// flood/distributed run can never be unbounded or non-lab.
#[derive(Clone, Copy, Debug)]
pub struct AttackSpec {
    /// Stable `-attack` selector name (matches the Go CLI).
    pub name: &'static str,
    /// Cost ladder / control tier.
    pub tier: &'static str,
    /// Attack family grouping.
    pub class: &'static str,
    /// The residual tell / hard rule the Blue engine must catch (intent only).
    pub expected: &'static str,
    /// Explicit request cap for bounded multi-request attacks (lab-only).
    pub bounded_requests: Option<u32>,
    /// Short provenance note tying the entry to the Go oracle behaviour.
    pub note: &'static str,
    /// Oracle function the plan is ported from (981d25c).
    pub oracle: &'static str,
    /// The ported request plan.
    pub steps: fn() -> Vec<Step>,
}

mod plans;
use plans::*;

macro_rules! attack {
    ($name:literal, $tier:literal, $class:literal, $expected:literal, $cap:expr, $note:literal, $oracle:literal, $steps:path) => {
        AttackSpec {
            name: $name,
            tier: $tier,
            class: $class,
            expected: $expected,
            bounded_requests: $cap,
            note: $note,
            oracle: $oracle,
            steps: $steps,
        }
    };
}

/// The full catalog, one entry per Go `-attack` name.
pub const CATALOG: &[AttackSpec] = &[
    // --- TLS parrot / rotation family ---
    attack!(
        "tls-static",
        "T1",
        "tls",
        "l5.traffic.tls_no_permutation -> HR-14",
        Some(4),
        "static HelloChrome_100 parrot: identical ClientHello on every collect",
        "cmd/redteam/attacks.go::tlsStatic",
        tls_static
    ),
    attack!(
        "tls-rotate",
        "T1",
        "tls",
        "l5.traffic.engine_rotation -> HR-14",
        Some(2),
        "two TLS stacks (Chrome_Auto then Firefox_Auto) in one session",
        "cmd/redteam/attacks.go::tlsRotate",
        tls_rotate
    ),
    attack!(
        "ja4-churn",
        "T1",
        "tls",
        "l5.traffic.ja4_rotation -> HR-14",
        Some(3),
        "Chrome_100, Firefox_Auto, Chrome_Auto: three JA4s in one session",
        "cmd/redteam/attacks.go::ja4Churn",
        ja4_churn
    ),
    attack!(
        "multi-axis-rotate",
        "T1",
        "tls",
        "ua_rotation + ja4_rotation -> HR-15",
        Some(2),
        "rotate User-Agent AND TLS engine together",
        "cmd/redteam/attacks.go::multiAxisRotate",
        multi_axis_rotate
    ),
    attack!(
        "ua-rotate",
        "T1",
        "tls",
        "l5.traffic.ua_rotation",
        Some(2),
        "two User-Agents, same TLS preset",
        "cmd/redteam/attacks.go::uaRotate",
        ua_rotate
    ),
    attack!(
        "grease-absent-js",
        "T0",
        "tls",
        "l5.tls.grease_absent + x.ua_vs_ja4",
        None,
        "no-GREASE Go stock stack under a Chrome UA that runs JS",
        "cmd/redteam/attacks.go::greaseAbsentJS",
        grease_absent_js
    ),
    attack!(
        "alps-absent",
        "runtime",
        "tls",
        "l5.tls.alps_absent -> HR-24",
        None,
        "Chrome_Auto hello minus ALPS over h2",
        "cmd/redteam/attacks.go::alpsAbsent",
        alps_absent
    ),
    attack!(
        "ech-absent",
        "runtime",
        "tls",
        "l5.tls.ech_absent -> HR-24",
        None,
        "Chrome_Auto hello minus ECH GREASE over h2",
        "cmd/redteam/attacks.go::echAbsent",
        ech_absent
    ),
    attack!(
        "cert-compression-absent",
        "runtime",
        "tls",
        "l5.tls.cert_compression_absent -> HR-24",
        None,
        "Chrome_Auto hello minus compress_certificate (ext 27) over h2",
        "cmd/redteam/attacks.go::certCompressionAbsent",
        cert_compression_absent
    ),
    attack!(
        "pq-absent",
        "runtime",
        "tls",
        "l5.tls.pq_keyshare -> HR-24",
        None,
        "pre-PQ HelloChrome_100 while the UA claims Chrome 131",
        "cmd/redteam/attacks.go::pqAbsent",
        pq_absent
    ),
    // --- Raw HTTP/2 protocol-split family ---
    attack!(
        "h2-settings-split",
        "runtime",
        "h2",
        "l5.http2.browser_settings_atypical",
        Some(2),
        "Chrome pseudo-order, library SETTINGS, no window",
        "cmd/redteam/h2raw.go::h2SettingsSplit",
        h2_settings_split
    ),
    attack!(
        "h2-flow-control",
        "runtime",
        "h2",
        "l5.http2.flow_control_atypical",
        Some(2),
        "coherent SETTINGS but a 1 GiB connection window",
        "cmd/redteam/h2raw.go::h2FlowControlSplit",
        h2_flow_control
    ),
    attack!(
        "h2-priority-absent",
        "runtime",
        "h2",
        "l5.http2.priority_atypical -> HR-24",
        Some(2),
        "coherent SETTINGS + real window, no HEADERS priority",
        "cmd/redteam/h2raw.go::h2PriorityAbsent",
        h2_priority_absent
    ),
    attack!(
        "h2-protocol-split",
        "runtime",
        "h2",
        "x.ua_vs_h2 / l5.http2.engine_mismatch",
        Some(2),
        "TLS/JA4 Chrome but the Go x/net/http2 Transport frame layout",
        "cmd/redteam/attacks.go::h2ProtocolSplit",
        h2_protocol_split
    ),
    attack!(
        "coherent-ceiling",
        "T4",
        "ceiling",
        "ALLOW (intentional detection ceiling)",
        Some(2),
        "full coherent Chrome h2 fingerprint; honest ALLOW",
        "cmd/redteam/h2raw.go::coherentBrowserH2",
        coherent_ceiling
    ),
    // --- Header-order / H1 cross-layer family ---
    attack!(
        "browser-tls-h1",
        "runtime",
        "http",
        "l5.http.browser_tls_over_h1 -> HR-24",
        None,
        "coherent Chrome TLS delivered over HTTP/1.1",
        "cmd/redteam/attacks.go::browserTLSOverH1",
        browser_tls_h1
    ),
    attack!(
        "header-order-split",
        "runtime",
        "http",
        "l5.header.order -> HR-24",
        None,
        "user-agent before the sec-ch-ua cluster (exact wire order)",
        "cmd/redteam/attacks.go::headerOrderSplit",
        header_order_split
    ),
    // --- Header / RIT cross-check family ---
    attack!(
        "nonbrowser-ua",
        "T0",
        "header",
        "x.non_browser_ua + HR-10",
        None,
        "bare HTTP library UA over Go stock TLS, no browser headers, no JS",
        "cmd/redteam/attacks.go::nonBrowserUA",
        nonbrowser_ua
    ),
    attack!(
        "sec-chua-absent",
        "T0",
        "header",
        "x.uach_present -> CHALLENGE",
        None,
        "Chromium UA that omits Sec-CH-UA client hints",
        "cmd/redteam/attacks.go::secCHUAAbsent",
        sec_chua_absent
    ),
    attack!(
        "sec-fetch-absent",
        "T0",
        "header",
        "l5.header.sec_fetch_missing",
        None,
        "Chrome UA that omits Sec-Fetch-* metadata",
        "cmd/redteam/attacks.go::secFetchAbsent",
        sec_fetch_absent
    ),
    attack!(
        "rit-replay",
        "T0",
        "rit",
        "l5.rit.stale_replay",
        Some(2),
        "replay a previously valid RIT token/counter",
        "cmd/redteam/attacks.go::ritReplay",
        rit_replay
    ),
    attack!(
        "rit-tamper",
        "T0",
        "rit",
        "l5.rit.header_tampered",
        None,
        "sign one body, send a different body",
        "cmd/redteam/attacks.go::ritTamper",
        rit_tamper
    ),
    attack!(
        "rit-absent",
        "T0",
        "rit",
        "l5.rit.absent -> HR-17",
        Some(2),
        "never present an anti-tamper token",
        "cmd/redteam/attacks.go::ritAbsent",
        rit_absent
    ),
    attack!(
        "signal-forgery",
        "T2",
        "provenance",
        "forged l7.pass/pow.solved inert; must not ALLOW",
        None,
        "client forges server-only trust-upgrade signals",
        "cmd/redteam/attacks.go::signalForgery",
        signal_forgery
    ),
    // --- Proxy / VPN / Tor / CDN header-spoof family (HR-24 / HR-19) ---
    attack!(
        "xff-spoof",
        "T0",
        "proxy",
        "l5.header.forwarded_private",
        None,
        "forged private X-Forwarded-For to fake a LAN client",
        "cmd/redteam/attacks.go::xffSpoof",
        xff_spoof
    ),
    attack!(
        "squid-forward",
        "T0",
        "proxy",
        "l5.header.proxy_hop -> HR-24",
        None,
        "Squid forward-proxy Via / X-Cache residual",
        "cmd/redteam/attacks.go::squidForward",
        squid_forward
    ),
    attack!(
        "openvpn-exit",
        "T0",
        "proxy",
        "l5.proxy.vpn_webrtc_leak -> HR-24",
        None,
        "VPN exit with a WebRTC public-IP leak",
        "cmd/redteam/attacks.go::openvpnExit",
        openvpn_exit
    ),
    attack!(
        "wireguard-hop",
        "T0",
        "proxy",
        "ip_hop + xff_multi_hop -> HR-24",
        Some(2),
        "WireGuard exit rotation in one session",
        "cmd/redteam/attacks.go::wireguardHop",
        wireguard_hop
    ),
    attack!(
        "tor-exit",
        "T0",
        "proxy",
        "l5.proxy.tor_circuit -> HR-24 / HR-19",
        Some(4),
        "Tor circuit multi-hop + exit-subnet churn",
        "cmd/redteam/attacks.go::torExit",
        tor_exit
    ),
    attack!(
        "anon-proxy-chain",
        "T0",
        "proxy",
        "l5.proxy.anon_chain -> HR-24",
        None,
        "4+ hop open-proxy chain with Proxy-Connection",
        "cmd/redteam/attacks.go::anonProxyChain",
        anon_proxy_chain
    ),
    attack!(
        "elite-anon-proxy",
        "T0",
        "proxy",
        "l5.header.proxy_hop (forwarded-multi) -> HR-24",
        None,
        "RFC 7239 Forwarded multi-for, Via stripped",
        "cmd/redteam/attacks.go::eliteAnonProxy",
        elite_anon_proxy
    ),
    attack!(
        "cdn-ip-spoof",
        "T0",
        "proxy",
        "l5.header.client_ip_spoof -> HR-24",
        None,
        "forged CF-Connecting-IP / True-Client-IP",
        "cmd/redteam/attacks.go::cdnIPSpoof",
        cdn_ip_spoof
    ),
    attack!(
        "proxy-ua-rotate",
        "T0",
        "proxy",
        "ua_rotation + ip_hop -> HR-15 / HR-19",
        Some(3),
        "residential exit rotation + mid-session UA rotate",
        "cmd/redteam/attacks.go::proxyUARotate",
        proxy_ua_rotate
    ),
    attack!(
        "fp-churn-proxy",
        "T0",
        "proxy",
        "l5.correlation.fp_churn_proxy -> HR-19",
        Some(3),
        "rotate fingerprintId per exit to dodge HR-19",
        "cmd/redteam/attacks.go::fpChurnProxy",
        fp_churn_proxy
    ),
    attack!(
        "stacked-proxy-vpn",
        "T0",
        "proxy",
        "proxy_hop + vpn_webrtc_leak -> HR-24",
        None,
        "Squid hop over a VPN exit with WebRTC leak",
        "cmd/redteam/attacks.go::stackedProxyVPN",
        stacked_proxy_vpn
    ),
    attack!(
        "socks-exit-hop",
        "T0",
        "proxy",
        "ip_hop + xff_multi_hop -> HR-24",
        Some(2),
        "SOCKS5/VPN L4 exit hop + multi-hop XFF",
        "cmd/redteam/attacks.go::socksExitHop",
        socks_exit_hop
    ),
    // --- Bounded volume family (explicit lab caps) ---
    attack!(
        "flood",
        "T0",
        "volume",
        "l5.abuse.flood -> CHALLENGE + ban ladder",
        Some(90),
        "application-layer flood from one fingerprint (capped)",
        "cmd/redteam/attacks.go::flood",
        flood
    ),
    attack!(
        "distributed",
        "T0",
        "volume",
        "l5.correlation.proxy_rotation -> HR-19",
        Some(4),
        "one fingerprint across rotated proxy subnets (capped)",
        "cmd/redteam/attacks.go::distributed",
        distributed
    ),
    attack!(
        "privacy-evasion",
        "T0",
        "volume",
        "HR-19 (privacy flag must not disarm)",
        Some(4),
        "distributed rotation that also claims ad-block/GPC",
        "cmd/redteam/attacks.go::privacyEvasion",
        privacy_evasion
    ),
    // --- Proof-of-work active-defense family ---
    attack!(
        "pow-fast-solve",
        "T2",
        "pow",
        "l7.pow.too_fast -> HR-19",
        None,
        "native PoW solve faster than any browser JS solver",
        "cmd/redteam/attacks.go::powFastSolve",
        pow_fast_solve
    ),
    attack!(
        "pow-launder",
        "T2",
        "pow",
        "too-fast tell must persist; no laundered ALLOW",
        None,
        "delayed resubmit of a natively-solved nonce",
        "cmd/redteam/attacks.go::powLaunder",
        pow_launder
    ),
];

/// Resolve a `-attack` selector name to its catalog entry.
pub fn select(name: &str) -> Option<&'static AttackSpec> {
    CATALOG.iter().find(|spec| spec.name == name)
}

fn mutation_name(mutation: Mutation) -> &'static str {
    match mutation {
        Mutation::StripAlps => "strip-alps",
        Mutation::StripEch => "strip-ech",
        Mutation::StripCertCompression => "strip-cert-compression",
    }
}

fn h2_profile_name(profile: H2Profile) -> &'static str {
    match profile {
        H2Profile::Chrome => "raw-chrome",
        H2Profile::SettingsSplit => "raw-settings-split",
        H2Profile::FlowControlSplit => "raw-flow-control-split",
        H2Profile::PriorityAbsent => "raw-priority-absent",
        H2Profile::GoTransport => "go-x-net-http2-transport",
    }
}

impl Step {
    /// The ClientHello this step's connection presents. `connection` seeds the
    /// per-connection extension shuffle of shuffling presets.
    pub fn client_hello(&self, connection: u64) -> ClientHelloBuilder {
        let mut builder = ClientHelloBuilder::new(self.hello).permute(connection);
        if let Some(m) = self.mutation {
            builder = builder.mutate(m);
        }
        if !matches!(self.http, Http::H2(_)) {
            builder = builder.force_http11();
        }
        builder
    }

    fn tls_json(&self, connection: u64) -> Value {
        let builder = self.client_hello(connection);
        let hello = parse_client_hello(&builder.handshake())
            .expect("builder emits a parseable ClientHello");
        let alpn: Vec<String> = hello
            .alpn
            .iter()
            .map(|a| String::from_utf8_lossy(a).into_owned())
            .collect();
        json!({
            "preset": self.hello.oracle_preset(),
            "mutation": self.mutation.map(mutation_name),
            "alpn": alpn,
            "ja3": ja3(&hello).0,
            "ja4": ja4(&hello).0,
            "extensionOrder": extension_order_hash(Some(&hello)),
            "engine": engine_from_client_hello(Some(&hello)),
        })
    }

    fn h2_json(&self, profile: H2Profile) -> Value {
        let mut request = h2::H2Request::new(profile, self.method, LAB_HOST, self.path)
            .expect("plan authority is the reserved owned-lab name");
        for (name, value) in &self.headers {
            request = request.header(name, value);
        }
        let request = request.body(self.body.clone().into_bytes());
        let mut capture = humanymous_runtime::capture::http2::Capture::default();
        capture
            .feed(&request.encode())
            .expect("runtime H2 capture accepts the frames");
        let fp = capture.request(1).expect("stream 1 request").fingerprint();
        json!({
            "profile": h2_profile_name(profile),
            "akamai": fp.akamai(),
            "pseudoOrder": fp.pseudo_string(),
            "engine": fp.engine(),
        })
    }

    fn rit_json(&self) -> Value {
        match &self.rit {
            Rit::None => Value::Null,
            Rit::Sign(k) => json!({"sign": format!("n+{k}"), "body": "sent body"}),
            Rit::Replay(k) => {
                json!({"replay": format!("n+{k}"), "body": "same token and counter as the earlier signed step"})
            }
            Rit::SignOtherBody(k, signed) => {
                json!({"sign": format!("n+{k}"), "signedBody": signed, "body": "different sent body"})
            }
        }
    }

    fn json(&self, index: usize) -> Value {
        let connection = index as u64 + 1;
        let mut step = json!({
            "step": index + 1,
            "purpose": self.purpose,
            "tls": self.tls_json(connection),
            "http": if matches!(self.http, Http::H2(_)) { "h2" } else { "http/1.1" },
            "method": self.method,
            "path": self.path,
            "headers": self.headers.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(),
            "body": self.body,
            "rit": self.rit_json(),
        });
        match self.http {
            Http::H1 => {
                step["headerOrder"] = json!("composition order; Go wrote a map (random order)")
            }
            Http::H1Ordered => step["headerOrder"] = json!("exact wire order"),
            Http::H2(profile) => {
                step["headerOrder"] = json!(if profile == H2Profile::GoTransport {
                    "composition order; Go x/net/http2 wrote a map (random order)"
                } else {
                    "Chrome order of the raw framer, then remaining headers (Go map order), cookie last"
                });
                step["h2"] = self.h2_json(profile);
            }
        }
        if self.repeat > 1 {
            step["repeat"] = json!(self.repeat);
        }
        if let Some(note) = self.note {
            step["note"] = json!(note);
        }
        step
    }
}

impl AttackSpec {
    /// The ported request plan of this attack.
    pub fn plan(&self) -> Vec<Step> {
        (self.steps)()
    }

    /// Emit the defense-only plan artifact: the ordered requests this attack
    /// would send to the owned runtime, with each step's fingerprint computed
    /// by the project's own observers. Nothing is sent.
    pub fn plan_json(&self) -> Value {
        let steps: Vec<Value> = self
            .plan()
            .iter()
            .enumerate()
            .map(|(i, step)| step.json(i))
            .collect();
        let mut plan = json!({
            "attack": self.name,
            "tier": self.tier,
            "class": self.class,
            "expected": self.expected,
            "note": self.note,
            "oracle": format!("981d25c {}", self.oracle),
            "target": "owned-lab-runtime-only",
            "executed": false,
            "steps": steps,
        });
        if let Some(cap) = self.bounded_requests {
            plan["boundedRequests"] = json!(cap);
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

    fn collects(name: &str) -> Vec<Value> {
        let plan = select(name).unwrap().plan_json();
        plan["steps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["purpose"] == "collect")
            .cloned()
            .collect()
    }

    fn header<'a>(step: &'a Value, name: &str) -> Option<&'a str> {
        step["headers"].as_array().unwrap().iter().find_map(|pair| {
            pair[0]
                .as_str()
                .filter(|k| k.eq_ignore_ascii_case(name))
                .and(pair[1].as_str())
        })
    }

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
    fn bounded_caps_equal_the_collect_requests_each_plan_sends() {
        for spec in CATALOG {
            let collects: u32 = spec
                .plan()
                .iter()
                .filter(|s| s.purpose == "collect")
                .map(|s| s.repeat)
                .sum();
            match spec.bounded_requests {
                Some(cap) => assert_eq!(collects, cap, "{}", spec.name),
                None => assert!(collects <= 1, "{} is multi-request but uncapped", spec.name),
            }
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
    fn every_step_carries_content_and_an_observed_wire_fingerprint() {
        for spec in CATALOG {
            let plan = spec.plan_json();
            assert_eq!(plan["target"], "owned-lab-runtime-only");
            assert_eq!(plan["executed"], false);
            let steps = plan["steps"].as_array().unwrap();
            assert!(steps.len() >= 2, "{} has no session + request", spec.name);
            assert_eq!(steps[0]["purpose"], "session", "{}", spec.name);
            for step in steps {
                assert_eq!(step["tls"]["ja3"].as_str().unwrap().len(), 32);
                assert!(step["tls"]["ja4"].as_str().unwrap().starts_with("t13d"));
                assert!(!step["headers"].as_array().unwrap().is_empty());
                if step["http"] == "h2" {
                    assert!(step["h2"]["akamai"].is_string(), "{}", spec.name);
                    assert_eq!(step["tls"]["alpn"], json!(["h2", "http/1.1"]));
                } else {
                    assert_eq!(header(step, "Host"), Some(LAB_HOST));
                    assert_eq!(step["tls"]["alpn"], json!(["http/1.1"]));
                }
                if step["purpose"] != "session" {
                    assert!(!step["body"].as_str().unwrap().is_empty(), "{}", spec.name);
                }
            }
        }
    }

    #[test]
    fn rotation_attacks_present_a_distinct_ja3_and_ja4_on_every_collect() {
        for name in ["tls-rotate", "ja4-churn", "multi-axis-rotate"] {
            let steps = collects(name);
            assert!(steps.len() >= 2);
            for i in 0..steps.len() {
                for j in i + 1..steps.len() {
                    assert_ne!(steps[i]["tls"]["ja3"], steps[j]["tls"]["ja3"], "{name}");
                    assert_ne!(steps[i]["tls"]["ja4"], steps[j]["tls"]["ja4"], "{name}");
                }
            }
        }
        // ja4-churn walks Chrome_100 -> Firefox_Auto -> Chrome_Auto like Go.
        let presets: Vec<_> = collects("ja4-churn")
            .iter()
            .map(|s| s["tls"]["preset"].as_str().unwrap().to_owned())
            .collect();
        assert!(presets[0].contains("HelloChrome_100"));
        assert!(presets[1].contains("HelloFirefox_Auto"));
        assert!(presets[2].contains("HelloChrome_Auto"));
    }

    #[test]
    fn tls_static_repeats_one_unpermuted_chrome_100_hello() {
        let steps = collects("tls-static");
        assert_eq!(steps.len(), 4);
        for step in &steps {
            assert!(
                step["tls"]["preset"]
                    .as_str()
                    .unwrap()
                    .contains("HelloChrome_100")
            );
            assert_eq!(step["tls"]["ja3"], steps[0]["tls"]["ja3"]);
            assert_eq!(
                step["tls"]["extensionOrder"],
                steps[0]["tls"]["extensionOrder"]
            );
        }
        // Chrome_Auto collects reshuffle per connection: same JA4, new order.
        let ua = collects("ua-rotate");
        assert_eq!(ua[0]["tls"]["ja4"], ua[1]["tls"]["ja4"]);
        assert_ne!(
            ua[0]["tls"]["extensionOrder"],
            ua[1]["tls"]["extensionOrder"]
        );
        assert_ne!(header(&ua[0], "User-Agent"), header(&ua[1], "User-Agent"));
    }

    #[test]
    fn mutation_and_pq_plans_report_the_targeted_tls_residual() {
        for name in ["alps-absent", "ech-absent", "cert-compression-absent"] {
            let step = &collects(name)[0];
            assert_eq!(step["tls"]["engine"], "chrome", "{name}");
            assert!(step["tls"]["mutation"].is_string(), "{name}");
            assert_eq!(step["http"], "h2");
            assert_eq!(step["h2"]["pseudoOrder"], "a,m,p,s");
        }
        let pq = &collects("pq-absent")[0];
        assert!(
            pq["tls"]["preset"]
                .as_str()
                .unwrap()
                .contains("HelloChrome_100")
        );
        assert!(pq["tls"]["mutation"].is_null());
        assert_eq!(header(pq, "sec-ch-ua"), Some(SEC_CH_UA_131));
        assert_eq!(header(pq, "User-Agent"), Some(CHROME_PQ_UA));
        let hello = parse_client_hello(
            &ClientHelloBuilder::new(Profile::Chrome100)
                .force_http11()
                .handshake(),
        )
        .unwrap();
        assert!(
            !hello
                .curves
                .contains(&crate::wire::clienthello::GROUP_X25519_MLKEM768)
        );
    }

    #[test]
    fn proxy_volume_and_header_families_carry_the_oracle_request_content() {
        let squid = &collects("squid-forward")[0];
        assert_eq!(header(squid, "Via"), Some("1.1 squid.lab (squid/5.7)"));
        assert_eq!(header(squid, "X-Forwarded-For"), Some("203.0.113.40"));
        assert!(
            squid["body"]
                .as_str()
                .unwrap()
                .contains("squ1df0rward000001squ1")
        );

        let tor = collects("tor-exit");
        let exits: Vec<_> = tor
            .iter()
            .map(|s| header(s, "X-Forwarded-For").unwrap())
            .collect();
        assert_eq!(
            exits,
            vec![
                "185.220.101.1, 185.220.102.2, 203.0.113.50",
                "198.51.100.11",
                "203.0.113.22",
                "192.0.2.33"
            ]
        );
        assert!(
            tor.iter()
                .all(|s| s["tls"]["preset"].as_str().unwrap().contains("Firefox"))
        );

        let cdn = &collects("cdn-ip-spoof")[0];
        assert_eq!(header(cdn, "CF-Connecting-IP"), Some("8.8.8.8"));
        assert_eq!(header(cdn, "True-Client-IP"), Some("1.1.1.1"));

        let distributed = collects("distributed");
        let ips: BTreeSet<_> = distributed
            .iter()
            .map(|s| header(s, "X-Forwarded-For").unwrap())
            .collect();
        assert_eq!(ips.len(), 4);
        assert!(distributed.iter().all(|s| {
            s["body"]
                .as_str()
                .unwrap()
                .contains("deadbeefcafe0001deadbeef")
        }));
        let privacy = &collects("privacy-evasion")[0];
        assert!(
            privacy["body"]
                .as_str()
                .unwrap()
                .contains(r#""adBlock":true"#)
        );

        let flood = &collects("flood")[0];
        assert_eq!(flood["repeat"], 90);

        let sec = &collects("sec-chua-absent")[0];
        assert_eq!(header(sec, "sec-ch-ua"), None);
        assert!(header(sec, "sec-fetch-site").is_some());

        let library = &collects("nonbrowser-ua")[0];
        assert_eq!(header(library, "User-Agent"), Some(PYTHON_UA));
        assert_eq!(library["tls"]["engine"], "go");

        let order: Vec<_> = collects("header-order-split")[0]["headers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p[0].as_str().unwrap().to_owned())
            .collect();
        let ua = order.iter().position(|k| k == "user-agent").unwrap();
        let ch = order.iter().position(|k| k == "sec-ch-ua").unwrap();
        assert!(ua < ch);

        let pow = select("pow-launder").unwrap().plan();
        assert_eq!(pow.iter().filter(|s| s.purpose == "pow-submit").count(), 2);
        assert!(pow.iter().all(|s| s.path.starts_with("/api/")));
    }

    #[test]
    fn rit_families_sign_replay_and_tamper_as_the_oracle_did() {
        let replay = select("rit-replay").unwrap().plan();
        assert_eq!(replay[1].rit, Rit::Sign(1));
        assert_eq!(replay[2].rit, Rit::Replay(1));
        assert_eq!(replay[1].headers, replay[2].headers);

        let tamper = select("rit-tamper").unwrap().plan();
        assert_eq!(tamper[1].rit, Rit::SignOtherBody(1, COLLECT_BODY.into()));
        assert_eq!(tamper[1].body, TAMPERED_BODY);

        let absent = select("rit-absent").unwrap().plan();
        assert!(absent.iter().all(|s| s.rit == Rit::None));
        assert!(
            absent
                .iter()
                .flat_map(|s| &s.headers)
                .all(|(k, _)| !k.eq_ignore_ascii_case("x-hm-token"))
        );
    }

    #[test]
    fn h2_plans_use_the_oracle_frame_layouts() {
        let split = &collects("h2-protocol-split")[0];
        assert_eq!(split["h2"]["profile"], "go-x-net-http2-transport");
        assert_eq!(split["h2"]["pseudoOrder"], "a,m,p,s");
        let ceiling = &collects("coherent-ceiling")[0];
        assert_eq!(ceiling["h2"]["engine"], "chrome");
        // Raw framer: Chrome header order, cookie last.
        let names: Vec<_> = ceiling["headers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p[0].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(names.first().map(String::as_str), Some("sec-ch-ua"));
        assert_eq!(names.last().map(String::as_str), Some("cookie"));
    }
}

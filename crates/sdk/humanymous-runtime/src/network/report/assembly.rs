use super::{Observation, claims};
use crate::network::{
    tcp,
    tls::{engine_from_client_hello, ja3, ja4},
};
use humanymous_core::signals::{
    observation::{Signal, Source, Verdict},
    report::NetworkReport,
};
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr};

fn browser_engine(engine: &str) -> bool {
    matches!(engine, "chrome" | "firefox" | "safari")
}
fn private_v4(ip: Ipv4Addr) -> bool {
    ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.is_unspecified()
}
fn private_forwarded(value: &str) -> bool {
    match value.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => private_v4(ip),
        Ok(IpAddr::V6(ip)) => {
            if let Some(v4) = ip.to_ipv4_mapped() {
                return private_v4(v4);
            }
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.segments()[0] & 0xfe00 == 0xfc00
                || ip.segments()[0] & 0xffc0 == 0xfe80
        }
        Err(_) => false,
    }
}

// Go encoding/json replaces each invalid UTF-8 byte, including every byte of a
// truncated sequence. Opaque TLS bytes stay intact in the capture itself.
fn go_string(bytes: &[u8]) -> String {
    let mut rest = bytes;
    let mut out = String::new();
    while !rest.is_empty() {
        match std::str::from_utf8(rest) {
            Ok(valid) => {
                out.push_str(valid);
                break;
            }
            Err(error) => {
                let valid = error.valid_up_to();
                out.push_str(std::str::from_utf8(&rest[..valid]).expect("valid prefix"));
                out.push('\u{fffd}');
                rest = &rest[valid + 1..];
            }
        }
    }
    out
}

/// Derive the frozen ordered L5 signal/report output. Missing capture emits
/// explicit dark-plane observations and never fabricated client fingerprints.
pub fn build(observation: &Observation) -> NetworkReport {
    let mut report = NetworkReport::default();
    let mut signals = Vec::new();
    let mut add = |id, value, verdict, notes: &str| {
        signals.push(Signal::new(id, value, verdict, 1.0, Source::Server, notes))
    };
    let header = &observation.header;
    if let Some(hello) = &observation.hello {
        (report.ja3, report.ja3_text) = ja3(hello);
        (report.ja4, report.ja4_raw) = ja4(hello);
        report.ja4_engine = engine_from_client_hello(Some(hello)).into();
        if let Some(alpn) = hello.alpn.first() {
            report.negotiated_alpn = go_string(alpn);
        }
        add("l5.tls.ja3", json!(report.ja3), Verdict::Unknown, "compat");
        if !hello
            .cipher_suites
            .iter()
            .chain(&hello.extensions)
            .any(|&value| crate::network::tls::is_grease(value))
        {
            add(
                "l5.tls.grease_absent",
                json!(true),
                Verdict::Bot,
                "no GREASE (non-browser TLS stack)",
            );
        }
        if claims::pq_browser(&header.user_agent) && !hello.curves.contains(&0x11ec) {
            add(
                "l5.tls.pq_keyshare",
                json!(true),
                Verdict::Suspicious,
                "UA claims a PQ-era browser but the TLS ClientHello omits X25519MLKEM768",
            );
        }
        let offers_h2 = hello.alpn.iter().any(|name| name == b"h2");
        if claims::chromium(&header.user_agent)
            && offers_h2
            && !hello
                .extensions
                .iter()
                .any(|ext| matches!(ext, 17513 | 17613))
        {
            add(
                "l5.tls.alps_absent",
                json!(true),
                Verdict::Suspicious,
                "Chromium UA offering h2 but the TLS ClientHello omits the ALPS extension",
            );
        }
        if claims::browser(&header.user_agent) && offers_h2 && !hello.extensions.contains(&27) {
            add(
                "l5.tls.cert_compression_absent",
                json!(true),
                Verdict::Suspicious,
                "browser UA offering h2 but the TLS ClientHello omits certificate compression",
            );
        }
        if claims::ech_browser(&header.user_agent)
            && offers_h2
            && !hello.extensions.contains(&0xfe0d)
        {
            add(
                "l5.tls.ech_absent",
                json!(true),
                Verdict::Suspicious,
                "UA claims an ECH-era browser but the TLS ClientHello omits Encrypted Client Hello",
            );
        }
    } else {
        add(
            "l5.tls.not_observed",
            json!(true),
            Verdict::Unknown,
            "no ClientHello captured — TLS fingerprint plane inactive (gate or TLS-terminating CDN/L7-LB in front)",
        );
    }
    if let Some(h2) = &observation.h2 {
        report.akamai_h2 = h2.akamai();
        report.pseudo_order = Some(h2.pseudo_order.clone());
        report.h2_engine = h2.engine().into();
        report.h2_settings = Some(
            h2.settings
                .iter()
                .map(|setting| (setting.id.to_string(), setting.value))
                .collect(),
        );
        if browser_engine(&report.h2_engine) && !h2.has_setting(1) {
            add(
                "l5.http2.browser_settings_atypical",
                json!(true),
                Verdict::Suspicious,
                "browser HTTP/2 pseudo-order with a non-browser SETTINGS profile (no HEADER_TABLE_SIZE)",
            );
        }
        if browser_engine(&report.h2_engine) && h2.window_update >= 64 << 20 {
            add(
                "l5.http2.flow_control_atypical",
                json!(true),
                Verdict::Suspicious,
                "browser HTTP/2 pseudo-order with a gigabyte connection flow-control window",
            );
        }
        if matches!(report.h2_engine.as_str(), "chrome" | "firefox")
            && h2.headers_priority.is_empty()
            && h2.priorities.is_empty()
        {
            add(
                "l5.http2.priority_atypical",
                json!(true),
                Verdict::Suspicious,
                "browser HTTP/2 pseudo-order with no priority signal (no HEADERS priority, no PRIORITY frame)",
            );
        }
    }
    report.header_order = Some(header.order());
    report.sec_fetch_present = header.sec_fetch_present();
    report.sec_ch_ua_present = header.sec_ch_ua_present();
    if observation.hello.is_some() && browser_engine(&report.ja4_engine) && !header.is_h2 {
        add(
            "l5.http.browser_tls_over_h1",
            json!(true),
            Verdict::Suspicious,
            "browser TLS fingerprint delivered over HTTP/1.1 (real browsers speak h2)",
        );
    }
    if observation.h2.is_some()
        && report.h2_engine == "unknown"
        && claims::browser(&header.user_agent)
    {
        add(
            "l5.http2.unknown_under_browser",
            json!(true),
            Verdict::Suspicious,
            "browser UA over an unclassifiable HTTP/2 profile (protocol-split residual)",
        );
    }
    if header.order_reliable && claims::chrome_order_anomaly(header) {
        add(
            "l5.header.order",
            json!(true),
            Verdict::Suspicious,
            "Chrome UA but user-agent precedes sec-ch-ua on the wire (non-browser header order)",
        );
    }
    if header.has_uppercase_in_h2() {
        add(
            "l5.header.h2_uppercase",
            json!(true),
            Verdict::Bot,
            "uppercase header in HTTP/2 (malformed)",
        );
    }
    if claims::chrome(&header.user_agent) && !header.sec_fetch_present() {
        add(
            "l5.header.sec_fetch_missing",
            json!(true),
            Verdict::Bot,
            "Chrome UA but no sec-fetch-*",
        );
    }
    if claims::chrome(&header.user_agent)
        && !header.accept_encoding.is_empty()
        && !header.chrome_accept_encoding_ok()
    {
        add(
            "l5.header.accept_encoding",
            json!(header.accept_encoding),
            Verdict::Suspicious,
            "Chrome UA but accept-encoding lacks zstd",
        );
    }
    let kind = header.proxy_hop_kind();
    if !kind.is_empty() {
        add(
            "l5.header.proxy_hop",
            json!(kind),
            Verdict::Bot,
            &format!("forward-proxy hop header residual ({kind})"),
        );
    }
    let kind = header.client_ip_spoof_kind();
    if !kind.is_empty() {
        add(
            "l5.header.client_ip_spoof",
            json!(kind),
            Verdict::Bot,
            &format!("forged CDN/edge client-IP header ({kind})"),
        );
    }
    let hops = header.xff_hop_count();
    if hops >= 2 {
        add(
            "l5.header.xff_multi_hop",
            json!(hops),
            Verdict::Suspicious,
            "multi-hop X-Forwarded-For chain",
        );
    }
    if hops >= 3 {
        add(
            "l5.proxy.tor_circuit",
            json!(hops),
            Verdict::Suspicious,
            "≥3-hop XFF circuit residual (Tor-class path)",
        );
    }
    if hops >= 4 {
        add(
            "l5.proxy.anon_chain",
            json!(hops),
            Verdict::Bot,
            "≥4-hop XFF anonymous open-proxy chain",
        );
    }
    if observation.is_datacenter_ip {
        add(
            "l5.ip.datacenter_asn",
            json!(true),
            Verdict::Suspicious,
            "datacenter/hosting ASN",
        );
    }
    if observation.is_proxy {
        add(
            "l5.ip.proxy_vpn_tor",
            json!(true),
            Verdict::Suspicious,
            "proxy/vpn exit",
        );
    }
    if observation.is_tor_exit {
        add(
            "l5.ip.tor_exit",
            json!(true),
            Verdict::Suspicious,
            "Tor exit relay",
        );
    }
    if private_forwarded(&observation.client_forwarded_ip) {
        add(
            "l5.header.forwarded_private",
            json!(observation.client_forwarded_ip),
            Verdict::Bot,
            "forwarded client IP is private/reserved (spoofed source)",
        );
    }
    signals.extend(tcp::signals(&observation.tcp, &header.user_agent));
    report.signals = Some(signals);
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forwarded_address_scope_matches_go_including_ipv4_mapped_v6() {
        for address in [
            "10.1.2.3",
            "127.0.0.1",
            "169.254.1.2",
            "0.0.0.0",
            "::",
            "::1",
            "fd00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(private_forwarded(address), "{address}");
        }
        for address in [
            "8.8.8.8",
            "100.64.0.1",
            "192.0.2.1",
            "ff02::1",
            "::127.0.0.1",
            "127.1",
            "010.1.2.3",
            "127.0.0.1:80",
            "",
        ] {
            assert!(!private_forwarded(address), "{address}");
        }
    }
    #[test]
    fn json_alpn_replacement_is_per_invalid_go_byte() {
        assert_eq!(go_string(&[b'a', 0xf0, 0x9f]), "a��");
        assert_eq!(go_string("h2🦀".as_bytes()), "h2🦀");
    }
}

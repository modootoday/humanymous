package network

import (
	"net"
	"strings"

	"github.com/modootoday/humanymous/internal/signals"
)

// report.go assembles a signals.NetworkReport from captured TLS/H2/header
// observations. It is the seam between the network package (fingerprint math)
// and the shared signal schema. Cross-checks (L6) are done in internal/scoring;
// here we only emit direct L5 signals and the resolved engine families.

// Observation bundles everything captured for one request/connection.
type Observation struct {
	Hello  *ClientHello   // nil if not captured (e.g. plain HTTP demo)
	H2     *H2Fingerprint // nil if not HTTP/2
	Header HeaderInfo
	// IsDatacenterIP / IsProxy / IsTorExit come from IP intel (operator CIDR feeds).
	IsDatacenterIP bool
	IsProxy        bool // commercial VPN / open-proxy ranges
	IsTorExit      bool // Tor exit-relay ranges (distinct from generic proxy/VPN)
	// ClientForwardedIP is the IP the CLIENT asserted via a forwarding header
	// (X-Forwarded-For left-most / X-Real-IP), captured regardless of proxy trust.
	// A real reverse proxy forwards the client's PUBLIC address; a private/reserved
	// value here is a forged "I'm on your LAN" source (see forwarded_private).
	ClientForwardedIP string
	// TCP is the optional L4 residual (PROXY TLV / eBPF). Zero-value = not observed.
	TCP TCPObservation
}

// Build turns an Observation into a NetworkReport (L5 signals + engine fields).
func Build(obs Observation) signals.NetworkReport {
	var nr signals.NetworkReport
	var sigs []signals.Signal
	add := func(id string, val any, v signals.Verdict, notes string) {
		sigs = append(sigs, signals.New(id, val, v, 1.0, signals.SourceServer, notes))
	}

	// --- TLS ---
	if obs.Hello != nil {
		ja3h, ja3t := JA3(obs.Hello)
		ja4, ja4raw := JA4(obs.Hello)
		nr.JA3, nr.JA3Text = ja3h, ja3t
		nr.JA4, nr.JA4Raw = ja4, ja4raw
		nr.JA4Engine = EngineFromClientHello(obs.Hello)
		if len(obs.Hello.ALPN) > 0 {
			nr.NegotiatedALPN = obs.Hello.ALPN[0]
		}
		add("l5.tls.ja3", ja3h, signals.VerdictUnknown, "compat")
		if !hasGREASE(obs.Hello) {
			add("l5.tls.grease_absent", true, signals.VerdictBot, "no GREASE (non-browser TLS stack)")
		}
	} else {
		// No ClientHello captured — the entire TLS/JA3/JA4 network plane is INACTIVE for this
		// request (the gate never captures it; a TLS-terminating CDN/L7-LB in front strips it
		// from the Core). Emit a weight-0, non-verdict marker so operators SEE the plane is dark
		// in the console/audit instead of mistaking silent absence for coverage (deep-review).
		// It carries no score and no BOT/SUSPICIOUS verdict, so the frozen detection is untouched.
		add("l5.tls.not_observed", true, signals.VerdictUnknown, "no ClientHello captured — TLS fingerprint plane inactive (gate or TLS-terminating CDN/L7-LB in front)")
	}

	// --- HTTP/2 ---
	if obs.H2 != nil {
		nr.AkamaiH2 = obs.H2.Akamai()
		nr.PseudoOrder = obs.H2.PseudoOrder
		nr.H2Engine = EngineFromH2(*obs.H2)
		nr.H2Settings = map[string]uint32{}
		for _, s := range obs.H2.Settings {
			nr.H2Settings[itoa(s.ID)] = s.Value
		}
		// Score-exempt residual: EngineFromH2 keys the browser engines on pseudo-order
		// ALONE, so a library that mimics Chrome/Firefox/Safari's order but not its SETTINGS
		// is accepted as that browser (the 2026 h2 fingerprint is order + SETTINGS + window).
		// Every real browser sends SETTINGS_HEADER_TABLE_SIZE (id 1); Go and many h2 clients
		// do not — so a browser-classified profile missing it is the protocol-split tell.
		if isBrowserEngine(nr.H2Engine) && !obs.H2.hasSetting(1) {
			add("l5.http2.browser_settings_atypical", true, signals.VerdictSuspicious,
				"browser HTTP/2 pseudo-order with a non-browser SETTINGS profile (no HEADER_TABLE_SIZE)")
		}
	}

	// --- Headers ---
	h := obs.Header
	nr.HeaderOrder = h.Order()
	nr.SecFetchPresent = h.SecFetchPresent()
	nr.SecCHUAPresent = h.SecCHUAPresent()

	// Score-exempt residual (weight 0): a browser-claiming UA delivered over an HTTP/2
	// profile the engine cannot classify as any known browser (EngineFromH2 == unknown).
	// A real Chrome/Firefox/Safari always presents a KNOWN h2 fingerprint, so this is the
	// 2026 "protocol-split" tell — a real browser TLS/JA4 carrying a library h2 frame layout.
	// Surfaced for Audit/Console/NET-POLICY only; it carries no score and moves no verdict.
	if obs.H2 != nil && nr.H2Engine == EngineUnknown && claimsBrowserUA(h.UserAgent) {
		add("l5.http2.unknown_under_browser", true, signals.VerdictSuspicious,
			"browser UA over an unclassifiable HTTP/2 profile (protocol-split residual)")
	}

	// Score-exempt residual (weight 0): a Chrome-claiming request whose on-wire header order
	// (only when OrderReliable — raw h1 peek / h2 HEADERS frame) places user-agent BEFORE the
	// sec-ch-ua client-hints. Real Chrome always emits the client-hints cluster first (measured
	// against real headless Chromium; SoT-02 / R8). Acted on by HR-24 NET-POLICY only.
	if h.OrderReliable && chromeUAOrderAnomaly(h) {
		add("l5.header.order", true, signals.VerdictSuspicious,
			"Chrome UA but user-agent precedes sec-ch-ua on the wire (non-browser header order)")
	}

	if h.HasUppercaseInH2() {
		add("l5.header.h2_uppercase", true, signals.VerdictBot, "uppercase header in HTTP/2 (malformed)")
	}
	claimChrome := strings.Contains(strings.ToLower(h.UserAgent), "chrome") &&
		!strings.Contains(strings.ToLower(h.UserAgent), "edg/")
	if claimChrome && !h.SecFetchPresent() {
		add("l5.header.sec_fetch_missing", true, signals.VerdictBot, "Chrome UA but no sec-fetch-*")
	}
	if claimChrome && h.AcceptEncoding != "" && !h.ChromeAcceptEncodingOK() {
		add("l5.header.accept_encoding", h.AcceptEncoding, signals.VerdictSuspicious, "Chrome UA but accept-encoding lacks zstd")
	}

	// --- Forward proxy / hop residual (Squid, open HTTP proxies, elite Forwarded) ---
	// A direct browser→origin request does not carry Via / Proxy-Connection /
	// Squid X-Cache on the request. Residual = traffic crossed an HTTP forward proxy.
	if kind := h.ProxyHopKind(); kind != "" {
		add("l5.header.proxy_hop", kind, signals.VerdictBot, "forward-proxy hop header residual ("+kind+")")
	}
	// CDN/edge client-identity headers forged by scrapers (anonymous-proxy laundering).
	if kind := h.ClientIPSpoofKind(); kind != "" {
		add("l5.header.client_ip_spoof", kind, signals.VerdictBot, "forged CDN/edge client-IP header ("+kind+")")
	}
	// Multi-hop XFF (≥2) is common on open-proxy chains and some VPN→proxy stacks.
	// Single-hop XFF is also set by legitimate reverse proxies (Gate); only multi-hop
	// is scored here. Soft signal — HR-24 needs a second tell for challenge.
	if n := h.XFFHopCount(); n >= 2 {
		add("l5.header.xff_multi_hop", n, signals.VerdictSuspicious, "multi-hop X-Forwarded-For chain")
	}
	// ≥3 XFF hops is rare for a single reverse proxy; typical of Tor-class circuit
	// residual or stacked open proxies. Soft — HR-24 promotes with browser claim.
	if n := h.XFFHopCount(); n >= 3 {
		add("l5.proxy.tor_circuit", n, signals.VerdictSuspicious, "≥3-hop XFF circuit residual (Tor-class path)")
	}
	// Free/anonymous open-proxy chains often stack 4+ hops (elite lists). Strong residual.
	if n := h.XFFHopCount(); n >= 4 {
		add("l5.proxy.anon_chain", n, signals.VerdictBot, "≥4-hop XFF anonymous open-proxy chain")
	}

	// --- IP intel ---
	if obs.IsDatacenterIP {
		add("l5.ip.datacenter_asn", true, signals.VerdictSuspicious, "datacenter/hosting ASN")
	}
	if obs.IsProxy {
		add("l5.ip.proxy_vpn_tor", true, signals.VerdictSuspicious, "proxy/vpn exit")
	}
	if obs.IsTorExit {
		add("l5.ip.tor_exit", true, signals.VerdictSuspicious, "Tor exit relay")
	}
	// A client-asserted forwarded IP that is private/loopback/link-local is a
	// forged source: a genuine reverse proxy forwards the client's PUBLIC address,
	// never a LAN one. This is the spoof used to shed datacenter/IP-intel signals.
	if ip := net.ParseIP(obs.ClientForwardedIP); ip != nil &&
		(ip.IsPrivate() || ip.IsLoopback() || ip.IsLinkLocalUnicast() || ip.IsUnspecified()) {
		add("l5.header.forwarded_private", obs.ClientForwardedIP, signals.VerdictBot, "forwarded client IP is private/reserved (spoofed source)")
	}

	// TCP/L4 residual plane — always observational (weight 0). Score-exempt; Audit owns it.
	sigs = append(sigs, TCPSignals(obs.TCP, h.UserAgent)...)

	nr.Signals = sigs
	return nr
}

// itoa avoids strconv import churn for small uint16 setting ids.
// claimsBrowserUA reports whether the UA claims a mainstream browser (Chrome, Firefox,
// or Safari). A real browser produces a KNOWN HTTP/2 fingerprint, so pairing such a claim
// with an unclassifiable h2 profile is the protocol-split residual. Library UAs
// (python-requests, Go-http-client, curl) return false and are covered by x.non_browser_ua.
// chromeUAOrderAnomaly reports whether a Chrome-claiming request placed user-agent BEFORE
// the sec-ch-ua client-hints on the wire. Real Chrome always sends the sec-ch-ua cluster
// before user-agent (client-hints-first, stable across versions and request types — nav and
// fetch); a header-spoofing library that appends the client hints after user-agent inverts
// it. FP-safe: fires ONLY when both headers are present AND the order is inverted, so a real
// browser (sec-ch-ua first, or no sec-ch-ua) never trips it. Names must be wire order
// (OrderReliable) — the caller gates on that.
func chromeUAOrderAnomaly(h HeaderInfo) bool {
	l := strings.ToLower(h.UserAgent)
	if !strings.Contains(l, "chrome") || strings.Contains(l, "edg/") {
		return false
	}
	uaPos, chPos := -1, -1
	for i, n := range h.Names {
		switch strings.ToLower(n) {
		case "user-agent":
			if uaPos < 0 {
				uaPos = i
			}
		case "sec-ch-ua":
			if chPos < 0 {
				chPos = i
			}
		}
	}
	return uaPos >= 0 && chPos >= 0 && uaPos < chPos
}

func claimsBrowserUA(ua string) bool {
	l := strings.ToLower(ua)
	if !strings.Contains(l, "mozilla/") {
		return false
	}
	return strings.Contains(l, "chrome") || strings.Contains(l, "firefox") ||
		strings.Contains(l, "safari")
}

func itoa(v uint16) string {
	if v == 0 {
		return "0"
	}
	var buf [8]byte
	i := len(buf)
	for v > 0 {
		i--
		buf[i] = byte('0' + v%10)
		v /= 10
	}
	return string(buf[i:])
}

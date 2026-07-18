package network

import (
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
	// IsDatacenterIP / IsProxy come from IP intel (internal/network/ipintel.go).
	IsDatacenterIP bool
	IsProxy        bool
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
	}

	// --- Headers ---
	h := obs.Header
	nr.HeaderOrder = h.Order()
	nr.SecFetchPresent = h.SecFetchPresent()
	nr.SecCHUAPresent = h.SecCHUAPresent()

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

	// --- IP intel ---
	if obs.IsDatacenterIP {
		add("l5.ip.datacenter_asn", true, signals.VerdictSuspicious, "datacenter/hosting ASN")
	}
	if obs.IsProxy {
		add("l5.ip.proxy_vpn_tor", true, signals.VerdictSuspicious, "proxy/vpn/tor exit")
	}

	nr.Signals = sigs
	return nr
}

// itoa avoids strconv import churn for small uint16 setting ids.
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

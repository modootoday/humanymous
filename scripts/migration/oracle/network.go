package main

import (
	"encoding/json"
	"github.com/modootoday/humanymous/internal/network"
	"os"
	"strings"
)

func networkInput(obs network.Observation) map[string]any {
	h := obs.Header
	header := map[string]any{
		"method": h.Method, "version": h.Version, "is_h2": h.IsH2, "names": append([]string{}, h.Names...),
		"casing_reliable": h.CasingReliable, "order_reliable": h.OrderReliable, "has_cookie": h.HasCookie, "has_referer": h.HasReferer,
		"accept_language": h.AcceptLanguage, "accept_encoding": h.AcceptEncoding, "cookie_names": append([]string{}, h.CookieNames...), "user_agent": h.UserAgent,
		"via": h.Via, "proxy_connection": h.ProxyConnection, "x_cache": h.XCache, "x_squid_error": h.XSquidError,
		"x_forwarded_for": h.XForwardedFor, "forwarded": h.Forwarded, "cf_connecting_ip": h.CFConnectingIP,
		"true_client_ip": h.TrueClientIP, "x_client_ip": h.XClientIP, "x_original_forwarded_for": h.XOriginalForwardedFor, "x_blue_coat_via": h.XBlueCoatVia,
	}
	var hello any
	if ch := obs.Hello; ch != nil {
		protocols := [][]uint16{}
		for _, name := range ch.ALPN {
			bytes := []uint16{}
			for _, b := range []byte(name) {
				bytes = append(bytes, uint16(b))
			}
			protocols = append(protocols, bytes)
		}
		hello = map[string]any{"legacy_version": ch.LegacyVersion, "supported_versions": append([]uint16{}, ch.SupportedVers...), "cipher_suites": append([]uint16{}, ch.CipherSuites...), "extensions": append([]uint16{}, ch.Extensions...), "curves": append([]uint16{}, ch.Curves...), "point_formats": append([]uint16{}, ch.PointFormats...), "signature_algorithms": append([]uint16{}, ch.SigAlgs...), "alpn": protocols, "sni_present": ch.SNIPresent}
	}
	var h2 any
	if f := obs.H2; f != nil {
		settings := []map[string]any{}
		for _, s := range f.Settings {
			settings = append(settings, map[string]any{"id": s.ID, "value": s.Value})
		}
		h2 = map[string]any{"settings": settings, "window_update": f.WindowUpdate, "priorities": append([]string{}, f.Priorities...), "headers_priority": f.HeadersPrio, "pseudo_order": append([]string{}, f.PseudoOrder...), "header_order": append([]string{}, f.HeaderOrder...)}
	}
	tcp := map[string]any{"observed": obs.TCP.Observed, "ttl": obs.TCP.TTL, "mss": obs.TCP.MSS, "window": obs.TCP.Window, "window_scale": obs.TCP.WScale, "sack": obs.TCP.SACK, "os_family": obs.TCP.OSFamily}
	return map[string]any{"hello": hello, "h2": h2, "header": header, "tcp": tcp, "is_datacenter_ip": obs.IsDatacenterIP, "is_proxy": obs.IsProxy, "is_tor_exit": obs.IsTorExit, "client_forwarded_ip": obs.ClientForwardedIP}
}

func exportNetwork() {
	type vector struct {
		Input        any                 `json:"input"`
		Expected     any                 `json:"expected"`
		Audit        []map[string]string `json:"audit"`
		Actionable   bool                `json:"actionable"`
		HeaderChecks any                 `json:"header_checks"`
	}
	var vectors []vector
	uas := []string{"", "curl/8", "Mozilla/5.0 Chrome/116", "Mozilla/5.0 Chrome/117", "Mozilla/5.0 Chrome/130", "Mozilla/5.0 Chrome/131", "Mozilla/5.0 Firefox/117", "Mozilla/5.0 Firefox/118", "Mozilla/5.0 Firefox/131", "Mozilla/5.0 Firefox/132", "Mozilla/5.0 Safari/17", "Mozilla/5.0 Chrome/149 Edg/149", "Chrome/9999999999999999999999999", "Mozilla/5.0 Windows Chrome/149", "Mozilla/5.0 Mac OS Firefox/149", "Mozilla/5.0 Android Chrome/149"}
	for index, ua := range uas {
		for mode := 0; mode < 64; mode++ {
			bit := func(position uint) bool { return mode&(1<<position) != 0 }
			obs := network.Observation{IsDatacenterIP: bit(0), IsProxy: bit(1), IsTorExit: bit(2)}
			obs.Header = network.HeaderInfo{Method: "POST", Version: "20", IsH2: bit(0), CasingReliable: bit(1), OrderReliable: bit(2), UserAgent: ua, Names: []string{"User-Agent", "sec-ch-ua", "sec-fetch-site"}, AcceptEncoding: []string{"", "gzip, br", "gzip, ZSTD"}[mode%3]}
			switch mode % 5 {
			case 1:
				obs.Header.Names = []string{"sec-ch-ua", "user-agent"}
			case 2:
				obs.Header.Names = []string{"USER-AGENT", "ſec-ch-ua"}
			case 3:
				obs.Header.Names = nil
			case 4:
				obs.Header.Names = []string{"sec-fetch-site", "sec-ch-ua", "user-agent"}
			}
			switch mode % 8 {
			case 1:
				obs.Header.Via = "1.1 SQUID"
			case 2:
				obs.Header.XSquidError = "ERR"
			case 3:
				obs.Header.XBlueCoatVia = "proxy"
			case 4:
				obs.Header.XCache = "hit"
			case 5:
				obs.Header.ProxyConnection = "keep-alive"
			case 6:
				obs.Header.Forwarded = "for=a;by=b"
			case 7:
				obs.Header.Forwarded = "for=a, for=b"
			}
			obs.Header.XForwardedFor = strings.Repeat(",", mode%5)
			obs.Header.CFConnectingIP = []string{"", "\u00a0", "127.0.0.1"}[mode%3]
			obs.Header.TrueClientIP = []string{"", "1.1.1.1"}[mode%2]
			obs.Header.XClientIP = []string{"", "\u3000", "10.0.0.1"}[(mode/3)%3]
			obs.Header.XOriginalForwardedFor = []string{"", "original"}[(mode/2)%2]
			obs.ClientForwardedIP = []string{"", "10.1.2.3", "127.0.0.1", "169.254.1.1", "0.0.0.0", "::1", "fd00::1", "fe80::1", "::ffff:127.0.0.1", "::127.0.0.1", "8.8.8.8", "100.64.0.1", "010.1.2.3", "ff02::1"}[(mode+index)%14]
			obs.TCP = network.TCPObservation{Observed: bit(3), TTL: []uint8{0, 64, 65, 128, 129, 255}[mode%6], MSS: uint16(mode % 2 * 1460), Window: uint16(mode % 3 * 1000), OSFamily: []string{"", "linux", "windows", "macos", "unknown"}[mode%5]}
			if bit(4) {
				obs.Hello = &network.ClientHello{LegacyVersion: 0x303, CipherSuites: []uint16{0x1301, 0x1302}, Extensions: []uint16{0, 10, 13, 16, 43}, ALPN: []string{"h2"}, SigAlgs: []uint16{0x403}, SupportedVers: []uint16{0x304}}
				if bit(0) {
					obs.Hello.CipherSuites = append(obs.Hello.CipherSuites, 0x0a0a)
				}
				if bit(1) {
					obs.Hello.Extensions = append(obs.Hello.Extensions, 27, 0x470d, 0xfe0d)
				}
				if bit(2) {
					obs.Hello.Curves = []uint16{0x11ec}
				}
				if mode%7 == 0 {
					obs.Hello.ALPN = []string{"http/1.1"}
				}
				if mode%11 == 0 {
					obs.Hello.ALPN = []string{string([]byte{0xf0, 0x9f})}
				}
			}
			if bit(5) {
				obs.H2 = &network.H2Fingerprint{PseudoOrder: [][]string{{"m", "a", "s", "p"}, {"m", "p", "a", "s"}, {"m", "s", "p", "a"}, {"m", "a"}}[mode%4], WindowUpdate: []uint32{0, (64 << 20) - 1, 64 << 20, 1073741824}[mode%4]}
				if bit(0) {
					obs.H2.Settings = []network.H2Setting{{ID: 1, Value: 65536}, {ID: 4, Value: 65535}}
				}
				if bit(1) {
					obs.H2.HeadersPrio = "1:0:255"
				}
				if bit(2) {
					obs.H2.Priorities = []string{"3:0:0:41"}
				}
			}
			report := network.Build(obs)
			audit := []map[string]string{}
			for _, r := range network.AuditRecordsFromSignals(report.Signals) {
				audit = append(audit, map[string]string{"event_type": r.EventType, "signal_id": r.SignalID, "verdict": r.Verdict, "notes": r.Notes})
			}
			h := obs.Header
			checks := map[string]any{"order": h.Order(), "sec_fetch": h.SecFetchPresent(), "sec_ch_ua": h.SecCHUAPresent(), "uppercase": h.HasUppercaseInH2(), "encoding": h.ChromeAcceptEncodingOK(), "proxy": h.ProxyHopKind(), "spoof": h.ClientIPSpoofKind(), "hops": h.XFFHopCount()}
			vectors = append(vectors, vector{networkInput(obs), report, audit, network.HasActionableResidual(report.Signals), checks})
		}
	}
	if err := json.NewEncoder(os.Stdout).Encode(vectors); err != nil {
		panic(err)
	}
}

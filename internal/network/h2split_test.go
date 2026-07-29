package network

import "testing"

// A browser-claiming UA delivered over an HTTP/2 profile that classifies as neither
// Chrome, Firefox, nor Safari (EngineFromH2 == unknown) is the 2026 "protocol-split"
// tell: a real browser TLS/JA4 carrying a library HTTP/2 frame layout. It must surface
// as the SCORE-EXEMPT residual l5.http2.unknown_under_browser for Audit/Console/NET-POLICY.
//
// Wargame round R3 (2026-07-27), freeze-safe half: the residual is observability only —
// weight 0, referenced by no hard rule — so the frozen verdict is unchanged.
func TestH2UnknownUnderBrowserResidual(t *testing.T) {
	// Go's http2.Transport layout observed on the wire: pseudo-order a,m,p,s and no
	// MAX_CONCURRENT_STREAMS (id 3), so EngineFromH2 returns "unknown".
	goH2 := &H2Fingerprint{
		Settings:    []H2Setting{{ID: 2, Value: 0}, {ID: 4, Value: 4194304}, {ID: 5, Value: 16384}, {ID: 6, Value: 10485760}},
		PseudoOrder: []string{"a", "m", "p", "s"},
	}
	if EngineFromH2(*goH2) != EngineUnknown {
		t.Fatalf("precondition: Go h2 layout should classify as unknown, got %q", EngineFromH2(*goH2))
	}
	chromeUA := "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"

	// Fires: browser UA + unclassifiable h2.
	if !buildIDs(Observation{H2: goH2, Header: HeaderInfo{UserAgent: chromeUA}})["l5.http2.unknown_under_browser"] {
		t.Error("browser UA over unknown h2 must raise l5.http2.unknown_under_browser")
	}

	// Quiet: a KNOWN browser h2 engine (real Chrome pseudo-order) under a browser UA.
	chromeH2 := &H2Fingerprint{PseudoOrder: []string{"m", "a", "s", "p"}}
	if EngineFromH2(*chromeH2) == EngineUnknown {
		t.Fatal("precondition: m,a,s,p should classify as Chrome, not unknown")
	}
	if buildIDs(Observation{H2: chromeH2, Header: HeaderInfo{UserAgent: chromeUA}})["l5.http2.unknown_under_browser"] {
		t.Error("a known browser h2 engine must NOT raise the residual")
	}

	// Quiet: a non-browser (library) UA — covered by x.non_browser_ua, not this residual.
	if buildIDs(Observation{H2: goH2, Header: HeaderInfo{UserAgent: "python-requests/2.31.0"}})["l5.http2.unknown_under_browser"] {
		t.Error("a library UA must NOT raise the browser-scoped residual")
	}

	// Quiet: no HTTP/2 at all (h1 connection).
	if buildIDs(Observation{Header: HeaderInfo{UserAgent: chromeUA}})["l5.http2.unknown_under_browser"] {
		t.Error("an h1 connection (no h2 fingerprint) must NOT raise the residual")
	}
}

// EngineFromH2 classifies the three browsers on pseudo-order ALONE, so a library that
// mimics Chrome's m,a,s,p but ships a non-browser SETTINGS profile is accepted as Chrome.
// Every real browser sends SETTINGS_HEADER_TABLE_SIZE (id 1); Go and many h2 clients omit
// it — the score-exempt residual l5.http2.browser_settings_atypical flags that split.
// Wargame R6 (2026-07-27).
func TestH2BrowserSettingsAtypicalResidual(t *testing.T) {
	chromeUA := "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"

	// Chrome pseudo-order + Go's SETTINGS (no HEADER_TABLE_SIZE): misclassified Chrome -> residual.
	mimic := &H2Fingerprint{
		PseudoOrder:  []string{"m", "a", "s", "p"},
		Settings:     []H2Setting{{2, 0}, {4, 4194304}, {5, 16384}, {6, 10485760}},
		WindowUpdate: 1073741824,
	}
	if EngineFromH2(*mimic) != EngineChrome {
		t.Fatal("precondition: mimic must classify as Chrome by pseudo-order")
	}
	if !buildIDs(Observation{H2: mimic, Header: HeaderInfo{UserAgent: chromeUA}})["l5.http2.browser_settings_atypical"] {
		t.Error("Chrome pseudo-order without HEADER_TABLE_SIZE must raise the residual")
	}

	// Real Chrome SETTINGS (with HEADER_TABLE_SIZE id 1): quiet.
	realChrome := &H2Fingerprint{
		PseudoOrder:  []string{"m", "a", "s", "p"},
		Settings:     []H2Setting{{1, 65536}, {2, 0}, {4, 6291456}, {6, 262144}},
		WindowUpdate: 15663105,
	}
	if buildIDs(Observation{H2: realChrome, Header: HeaderInfo{UserAgent: chromeUA}})["l5.http2.browser_settings_atypical"] {
		t.Error("a real Chrome SETTINGS profile (with HEADER_TABLE_SIZE) must NOT raise the residual")
	}

	// Non-browser pseudo-order (Go a,m,p,s -> unknown): quiet (covered by unknown_under_browser).
	goPseudo := &H2Fingerprint{PseudoOrder: []string{"a", "m", "p", "s"}, Settings: []H2Setting{{2, 0}, {4, 4194304}}}
	if buildIDs(Observation{H2: goPseudo, Header: HeaderInfo{UserAgent: chromeUA}})["l5.http2.browser_settings_atypical"] {
		t.Error("a non-browser pseudo-order must NOT raise the browser-scoped residual")
	}
}

// A Chrome-UA request whose on-wire header order places user-agent BEFORE sec-ch-ua is a
// non-browser order (real Chrome sends the client-hints cluster first — measured against real
// headless Chromium). The score-exempt residual l5.header.order fires ONLY when the order is
// reliable AND both headers are present AND inverted. Wargame R8 (2026-07-28).
func TestHeaderOrderAnomalyResidual(t *testing.T) {
	chromeUA := "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
	fire := func(order []string, reliable bool, ua string) bool {
		return buildIDs(Observation{Header: HeaderInfo{UserAgent: ua, Names: order, OrderReliable: reliable}})["l5.header.order"]
	}
	// Real Chrome navigation order (measured): sec-ch-ua BEFORE user-agent → quiet.
	realNav := []string{"sec-ch-ua", "sec-ch-ua-mobile", "sec-ch-ua-platform", "upgrade-insecure-requests", "user-agent", "accept", "sec-fetch-site", "accept-encoding"}
	if fire(realNav, true, chromeUA) {
		t.Error("real Chrome nav order (sec-ch-ua first) must NOT fire l5.header.order")
	}
	// Real Chrome fetch order (measured): no sec-ch-ua → quiet.
	realFetch := []string{"host", "connection", "content-type", "user-agent", "accept", "accept-language", "sec-fetch-mode", "accept-encoding", "content-length"}
	if fire(realFetch, true, chromeUA) {
		t.Error("real Chrome fetch order (no sec-ch-ua) must NOT fire l5.header.order")
	}
	// Spoofer: user-agent BEFORE sec-ch-ua → fires.
	spoof := []string{"host", "user-agent", "accept", "sec-ch-ua", "sec-ch-ua-mobile", "content-type"}
	if !fire(spoof, true, chromeUA) {
		t.Error("user-agent before sec-ch-ua under a Chrome UA must fire l5.header.order")
	}
	// Same inverted order but OrderReliable=false (net/http map, unknown order) → quiet (no false accusation).
	if fire(spoof, false, chromeUA) {
		t.Error("an unreliable (sorted map) order must NEVER fire l5.header.order")
	}
	// Non-Chrome UA → quiet (sec-ch-ua invariant is Chrome-specific).
	if fire(spoof, true, "Mozilla/5.0 (X11; Linux x86_64; rv:115.0) Gecko/20100101 Firefox/115.0") {
		t.Error("a non-Chrome UA must NOT fire the Chrome-scoped header-order residual")
	}
}

// A UA claiming a post-quantum-era browser (Chrome >= 131 / Firefox >= 132) whose TLS
// supported_groups omit X25519MLKEM768 (0x11EC) is a scraper pinning a pre-PQ profile. Real
// PQ-era browsers send it by default (measured vs real headless Chromium 149: curves include
// 4588). Version-gated so an older browser is never accused. Wargame R9 (2026-07-28).
func TestPQKeyShareResidual(t *testing.T) {
	fire := func(ua string, curves []uint16) bool {
		return buildIDs(Observation{Hello: &ClientHello{Curves: curves}, Header: HeaderInfo{UserAgent: ua}})["l5.tls.pq_keyshare"]
	}
	chrome149 := "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/149.0.7827.55 Safari/537.36"
	realCurves := []uint16{51914, 0x11EC, 29, 23, 24} // measured real Chromium 149 (has PQ)
	preCurves := []uint16{64250, 29, 23, 24}          // pre-PQ parrot (no 0x11EC)

	// Real Chrome 149 curves (with 0x11EC): quiet.
	if fire(chrome149, realCurves) {
		t.Error("real Chrome 149 (with X25519MLKEM768) must NOT fire l5.tls.pq_keyshare")
	}
	// Chrome/149 UA but pre-PQ curves (the spoof): fires.
	if !fire(chrome149, preCurves) {
		t.Error("Chrome/149 UA without X25519MLKEM768 must fire l5.tls.pq_keyshare")
	}
	// Older Chrome (126, pre-PQ era) without 0x11EC: quiet (version-gated, no false accusation).
	if fire("Mozilla/5.0 (Windows NT 10.0) Chrome/126.0.0.0 Safari/537.36", preCurves) {
		t.Error("Chrome/126 (pre-PQ era) must NOT fire — version gate")
	}
	// Edge (different rollout) claiming 131 without PQ: quiet.
	if fire("Mozilla/5.0 (Windows NT 10.0) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0", preCurves) {
		t.Error("Edge must NOT fire the PQ residual")
	}
	// Firefox 132 without PQ: fires.
	if !fire("Mozilla/5.0 (Windows NT 10.0; rv:132.0) Gecko/20100101 Firefox/132.0", preCurves) {
		t.Error("Firefox/132 without X25519MLKEM768 must fire l5.tls.pq_keyshare")
	}
}

// A Chromium-claiming UA (chrome/ token) offering h2 whose ClientHello omits the ALPS
// application_settings extension (codepoint 17513 or 17613) is a non-Chromium TLS stack wearing
// a Chrome UA — every Chromium build sends ALPS on h2; Firefox/Safari/Go/curl send none. Gated
// on h2-in-ALPN so an http/1.1-only client is never flagged. Wargame R10 (2026-07-28).
func TestALPSAbsentResidual(t *testing.T) {
	fire := func(ua string, alpn []string, exts []uint16) bool {
		return buildIDs(Observation{
			Hello:  &ClientHello{ALPN: alpn, Extensions: exts},
			Header: HeaderInfo{UserAgent: ua},
		})["l5.tls.alps_absent"]
	}
	chromeUA := "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36"
	h2 := []string{"h2", "http/1.1"}
	withALPSNew := []uint16{0x0000, 0x000a, 17613, 0x0010} // Chrome >= ~124
	withALPSOld := []uint16{0x0000, 0x000a, 17513, 0x0010} // Chrome <= ~123
	noALPS := []uint16{0x0000, 0x000a, 0x000d, 0x0010}

	// Chrome UA + h2 + ALPS present (either codepoint): quiet.
	if fire(chromeUA, h2, withALPSNew) {
		t.Error("real Chrome (ALPS 17613) must NOT fire l5.tls.alps_absent")
	}
	if fire(chromeUA, h2, withALPSOld) {
		t.Error("older Chrome (ALPS 17513) must NOT fire l5.tls.alps_absent")
	}
	// Chrome UA + h2 + NO ALPS (the spoof): fires.
	if !fire(chromeUA, h2, noALPS) {
		t.Error("Chrome UA over h2 without ALPS must fire l5.tls.alps_absent")
	}
	// Chrome UA but http/1.1-only ALPN (legitimately omits ALPS): quiet — the h2 gate.
	if fire(chromeUA, []string{"http/1.1"}, noALPS) {
		t.Error("http/1.1-only Chrome must NOT fire — ALPS is only sent when offering h2")
	}
	// Firefox (never sends ALPS): quiet — Chromium-only gate.
	if fire("Mozilla/5.0 (Windows NT 10.0; rv:132.0) Gecko/20100101 Firefox/132.0", h2, noALPS) {
		t.Error("Firefox must NOT fire the ALPS residual (non-Chromium)")
	}
	// Edge (Chromium — sends ALPS; without it, the spoof): fires.
	if !fire("Mozilla/5.0 (Windows NT 10.0) Chrome/133.0.0.0 Safari/537.36 Edg/133.0.0.0", h2, noALPS) {
		t.Error("Edge (Chromium) over h2 without ALPS must fire l5.tls.alps_absent")
	}
}

// A browser-classified h2 profile (Chrome pseudo-order) whose connection-level WINDOW_UPDATE
// opens a gigabyte-scale flow-control window is a library mimicking the pseudo-order but not the
// browser's bounded flow control. Real browsers stay well under 20 MB (Chrome 15663105); Go's
// http2 default is 1 GiB. The flow-control (W) dimension of the h2 fingerprint. Wargame R11.
func TestH2FlowControlResidual(t *testing.T) {
	fire := func(fp *H2Fingerprint, ua string) bool {
		return buildIDs(Observation{H2: fp, Header: HeaderInfo{UserAgent: ua}})["l5.http2.flow_control_atypical"]
	}
	chromeUA := "Mozilla/5.0 (Windows NT 10.0) Chrome/133 Safari/537.36"
	chromeOrder := []string{"m", "a", "s", "p"}
	coherentSettings := []H2Setting{{1, 65536}, {2, 0}, {4, 6291456}, {6, 262144}}

	// Browser pseudo-order + real Chrome window (15663105 ~15 MB): quiet.
	if fire(&H2Fingerprint{PseudoOrder: chromeOrder, Settings: coherentSettings, WindowUpdate: 15663105}, chromeUA) {
		t.Error("real Chrome window (15663105) must NOT fire l5.http2.flow_control_atypical")
	}
	// Browser pseudo-order + no WINDOW_UPDATE (increment 0): quiet.
	if fire(&H2Fingerprint{PseudoOrder: chromeOrder, Settings: coherentSettings, WindowUpdate: 0}, chromeUA) {
		t.Error("absent WINDOW_UPDATE (0) must NOT fire — absence != gigabyte window")
	}
	// Browser pseudo-order + 1 GiB window (the Go/library signature): fires.
	if !fire(&H2Fingerprint{PseudoOrder: chromeOrder, Settings: coherentSettings, WindowUpdate: 1073741824}, chromeUA) {
		t.Error("browser pseudo-order + 1 GiB flow-control window must fire l5.http2.flow_control_atypical")
	}
	// Non-browser h2 profile (unknown engine) + 1 GiB window: quiet — the residual is only for a
	// client that already mimics a browser pseudo-order (isBrowserEngine), not a raw library.
	if fire(&H2Fingerprint{PseudoOrder: []string{"a", "m", "p", "s"}, Settings: coherentSettings, WindowUpdate: 1073741824}, chromeUA) {
		t.Error("non-browser pseudo-order must NOT fire (residual is gated on browser classification)")
	}
}

// A browser-claiming UA offering h2 whose ClientHello omits compress_certificate (RFC 8879, ext
// 27) is a non-browser TLS stack wearing a browser UA — every modern browser (Chrome/Firefox/
// Safari) advertises it. Broader than ALPS (all engines, not just Chromium). Gated on h2-in-ALPN.
// Wargame R12 (2026-07-29).
func TestCertCompressionAbsentResidual(t *testing.T) {
	fire := func(ua string, alpn []string, exts []uint16) bool {
		return buildIDs(Observation{
			Hello:  &ClientHello{ALPN: alpn, Extensions: exts},
			Header: HeaderInfo{UserAgent: ua},
		})["l5.tls.cert_compression_absent"]
	}
	chromeUA := "Mozilla/5.0 (Windows NT 10.0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36"
	firefoxUA := "Mozilla/5.0 (Windows NT 10.0; rv:132.0) Gecko/20100101 Firefox/132.0"
	h2 := []string{"h2", "http/1.1"}
	withCC := []uint16{0x0000, 0x000a, 27, 0x0010}
	noCC := []uint16{0x0000, 0x000a, 0x000d, 0x0010}

	// Chrome UA + h2 + cert compression present: quiet.
	if fire(chromeUA, h2, withCC) {
		t.Error("Chrome with compress_certificate must NOT fire l5.tls.cert_compression_absent")
	}
	// Firefox UA + h2 + present: quiet (all browsers send it).
	if fire(firefoxUA, h2, withCC) {
		t.Error("Firefox with compress_certificate must NOT fire")
	}
	// Chrome UA + h2 + NO cert compression (the spoof): fires.
	if !fire(chromeUA, h2, noCC) {
		t.Error("Chrome UA over h2 without compress_certificate must fire")
	}
	// Firefox UA + h2 + NO cert compression: fires (broader than ALPS — all browser engines).
	if !fire(firefoxUA, h2, noCC) {
		t.Error("Firefox UA over h2 without compress_certificate must fire")
	}
	// Chrome UA but http/1.1-only ALPN: quiet — the h2 gate.
	if fire(chromeUA, []string{"http/1.1"}, noCC) {
		t.Error("http/1.1-only client must NOT fire — h2 gate")
	}
	// Non-browser UA (Go client) without cert compression: quiet — browser-UA gate.
	if fire("Go-http-client/2.0", h2, noCC) {
		t.Error("non-browser UA must NOT fire the cert-compression residual")
	}
}

// A Chrome/Firefox-classified h2 profile (by pseudo-order) that sends NO priority signal —
// neither a HEADERS-frame priority field nor a separate PRIORITY frame — is a raw-framer library
// mimicking the pseudo-order. Measured: real Chrome sets HEADERS priority excl=1/weight=255,
// Firefox excl=0/weight=41 (both always present). Safari excluded (unmeasured). Wargame R14.
func TestH2PriorityAbsentResidual(t *testing.T) {
	fire := func(fp *H2Fingerprint, ua string) bool {
		return buildIDs(Observation{H2: fp, Header: HeaderInfo{UserAgent: ua}})["l5.http2.priority_atypical"]
	}
	chromeUA := "Mozilla/5.0 (Windows NT 10.0) Chrome/133 Safari/537.36"
	chromeOrder := []string{"m", "a", "s", "p"}       // -> EngineChrome
	firefoxOrder := []string{"m", "p", "a", "s"}      // -> EngineFirefox
	settings := []H2Setting{{1, 65536}, {4, 6291456}}

	// Chrome pseudo-order WITH a HEADERS priority: quiet.
	if fire(&H2Fingerprint{PseudoOrder: chromeOrder, Settings: settings, HeadersPrio: "0:1:0:255"}, chromeUA) {
		t.Error("Chrome with HEADERS priority must NOT fire l5.http2.priority_atypical")
	}
	// Chrome pseudo-order with a separate PRIORITY frame (older-style): quiet.
	if fire(&H2Fingerprint{PseudoOrder: chromeOrder, Settings: settings, Priorities: []string{"3:0:0:200"}}, chromeUA) {
		t.Error("a separate PRIORITY frame must NOT fire priority_atypical")
	}
	// Chrome pseudo-order but NO priority signal at all (the raw-framer spoof): fires.
	if !fire(&H2Fingerprint{PseudoOrder: chromeOrder, Settings: settings}, chromeUA) {
		t.Error("Chrome pseudo-order with no priority signal must fire l5.http2.priority_atypical")
	}
	// Firefox pseudo-order but no priority signal: fires (Firefox also always sends one).
	if !fire(&H2Fingerprint{PseudoOrder: firefoxOrder, Settings: settings}, "Mozilla/5.0 Firefox/132.0") {
		t.Error("Firefox pseudo-order with no priority signal must fire")
	}
}

// A browser TLS stack (JA4 engine Chrome/Firefox/Safari) whose request was delivered over
// HTTP/1.1 is a browser-TLS parrot on an h1 client — real browsers always speak h2 to an
// h2-capable server (the JA4↔JA4H cross-layer tell). Requires a captured ClientHello. Wargame R15.
func TestBrowserTLSOverH1Residual(t *testing.T) {
	fire := func(hello *ClientHello, isH2 bool) bool {
		return buildIDs(Observation{Hello: hello, Header: HeaderInfo{UserAgent: "Mozilla/5.0 Chrome/133", IsH2: isH2}})["l5.http.browser_tls_over_h1"]
	}
	// A real Chrome ClientHello (GREASE + compress_certificate -> EngineChrome).
	chromeHello := &ClientHello{
		CipherSuites: []uint16{0x0a0a, 0x1301, 0x1302, 0x1303, 0xc02b, 0xc02f, 0xc02c, 0xc030, 0xcca9, 0xcca8, 0xc013, 0xc014, 0x009c, 0x009d, 0x002f, 0x0035},
		Extensions:   []uint16{0x0a0a, 27, 0x000a, 0x000d}, // GREASE + compress_certificate(27)
	}
	goHello := &ClientHello{CipherSuites: []uint16{0x1301, 0x1302, 0x1303}, Extensions: []uint16{0x000a}} // no GREASE -> EngineGo

	// Chrome JA4 over HTTP/1.1 (the spoof): fires.
	if !fire(chromeHello, false) {
		t.Error("Chrome JA4 over HTTP/1.1 must fire l5.http.browser_tls_over_h1")
	}
	// Chrome JA4 over h2 (a real browser): quiet.
	if fire(chromeHello, true) {
		t.Error("Chrome JA4 over h2 must NOT fire (real browsers speak h2)")
	}
	// Non-browser TLS (Go) over HTTP/1.1: quiet — not a browser JA4.
	if fire(goHello, false) {
		t.Error("non-browser (Go) TLS over h1 must NOT fire the browser-TLS-over-h1 residual")
	}
}

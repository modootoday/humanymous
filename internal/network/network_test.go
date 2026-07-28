package network

import (
	"strings"
	"testing"
)

func TestIsGREASE(t *testing.T) {
	greases := []uint16{0x0a0a, 0x1a1a, 0x2a2a, 0xdada, 0xfafa}
	for _, g := range greases {
		if !isGREASE(g) {
			t.Errorf("expected %#04x to be GREASE", g)
		}
	}
	for _, n := range []uint16{0x1301, 0x0033, 0xc02b, 0x000a} {
		if isGREASE(n) {
			t.Errorf("expected %#04x NOT to be GREASE", n)
		}
	}
}

// chromeLikeHello builds a Chrome-ish ClientHello with GREASE + markers.
func chromeLikeHello(extOrder []uint16) *ClientHello {
	return &ClientHello{
		LegacyVersion: 0x0303,
		SupportedVers: []uint16{0x0a0a, 0x0304, 0x0303}, // GREASE + 1.3 + 1.2
		CipherSuites:  []uint16{0x0a0a, 0x1301, 0x1302, 0x1303, 0xc02b, 0xc02f},
		Extensions:    extOrder,
		Curves:        []uint16{0x0a0a, 0x001d, 0x0017, 0x0018},
		SigAlgs:       []uint16{0x0403, 0x0804, 0x0401},
		ALPN:          []string{"h2", "http/1.1"},
		SNIPresent:    true,
	}
}

func TestJA4_Format(t *testing.T) {
	exts := []uint16{0x0a0a, 0x0000, 0x0010, 0x001b, 0x002b, 0x000d, 0x000a}
	ja4, raw := JA4(chromeLikeHello(exts))
	parts := strings.Split(ja4, "_")
	if len(parts) != 3 {
		t.Fatalf("JA4 should have 3 underscore-separated parts, got %q", ja4)
	}
	a := parts[0]
	if !strings.HasPrefix(a, "t13d") { // TLS over TCP, 1.3, SNI present
		t.Errorf("JA4_a prefix want t13d, got %q", a)
	}
	if !strings.HasSuffix(a, "h2") { // ALPN first value h2
		t.Errorf("JA4_a alpn suffix want h2, got %q", a)
	}
	if len(parts[1]) != 12 || len(parts[2]) != 12 {
		t.Errorf("JA4_b/c must be 12 hex chars, got %q / %q", parts[1], parts[2])
	}
	if !strings.Contains(raw, "_") {
		t.Errorf("raw JA4 should be present: %q", raw)
	}
}

func TestJA4_PermutationInvariant(t *testing.T) {
	// Chrome permutes extension order; JA4 must be identical regardless.
	order1 := []uint16{0x0a0a, 0x0000, 0x0010, 0x001b, 0x002b, 0x000d, 0x000a}
	order2 := []uint16{0x000a, 0x002b, 0x0a0a, 0x000d, 0x001b, 0x0010, 0x0000}
	j1, _ := JA4(chromeLikeHello(order1))
	j2, _ := JA4(chromeLikeHello(order2))
	if j1 != j2 {
		t.Fatalf("JA4 not permutation-invariant:\n %s\n %s", j1, j2)
	}
}

func TestEngineFromClientHello(t *testing.T) {
	chrome := chromeLikeHello([]uint16{0x0a0a, 0x0000, 0x001b, 0x002b}) // has compress_certificate
	if got := EngineFromClientHello(chrome); got != EngineChrome {
		t.Errorf("chrome hello -> %s", got)
	}
	firefox := &ClientHello{
		CipherSuites: []uint16{0x0a0a, 0x1301, 0x1302},
		Extensions:   []uint16{0x0a0a, 0x0000, extRecordSizeLimit, 0x002b},
	}
	if got := EngineFromClientHello(firefox); got != EngineFirefox {
		t.Errorf("firefox hello -> %s", got)
	}
	goHello := &ClientHello{ // no GREASE, small cipher set, no browser exts
		CipherSuites: []uint16{0x1301, 0x1302, 0x1303, 0xc02b, 0xc02f},
		Extensions:   []uint16{0x0000, 0x000a, 0x000d, 0x002b},
	}
	if got := EngineFromClientHello(goHello); got != EngineGo {
		t.Errorf("go hello -> %s", got)
	}
}

func TestEngineFromH2(t *testing.T) {
	cases := map[string]string{
		"m,a,s,p": EngineChrome,
		"m,p,a,s": EngineFirefox,
		"m,s,p,a": EngineSafari,
	}
	for order, want := range cases {
		f := H2Fingerprint{PseudoOrder: strings.Split(order, ",")}
		if got := EngineFromH2(f); got != want {
			t.Errorf("pseudo %q -> %s, want %s", order, got, want)
		}
	}
	// Go library profile (no browser pseudo order, MAX_CONCURRENT_STREAMS + 65535 window).
	goF := H2Fingerprint{Settings: []H2Setting{{3, 100}, {4, 65535}}}
	if got := EngineFromH2(goF); got != EngineGo {
		t.Errorf("go h2 -> %s", got)
	}
}

// EngineFromH2 classifies by pseudo-header ORDER ALONE for the three browsers — a
// client that mimics Chrome's m,a,s,p order but ships a NON-Chrome SETTINGS/
// WINDOW_UPDATE profile (the real Go layout measured in R3: no HEADER_TABLE_SIZE,
// a 1 GiB window update) is still classified as Chrome, so x.ua_vs_h2 stays
// consistent. This is the 2026 "HTTP/2 fingerprint = pseudo-order + SETTINGS +
// WINDOW_UPDATE" gap: pseudo-order is only one of three components. Wargame R6.
func TestEngineFromH2MisclassifiesBySettings(t *testing.T) {
	mimic := H2Fingerprint{
		PseudoOrder:  []string{"m", "a", "s", "p"},                                 // Chrome's order
		Settings:     []H2Setting{{2, 0}, {4, 4194304}, {5, 16384}, {6, 10485760}}, // Go's — no id1
		WindowUpdate: 1073741824,                                                   // 1 GiB — no browser does this
	}
	if got := EngineFromH2(mimic); got != EngineChrome {
		t.Fatalf("precondition: pseudo-order-only classification should call this Chrome, got %s", got)
	}
	// Documents the gap: a library SETTINGS profile is accepted as Chrome purely on
	// pseudo-order. The freeze-safe residual (l5.http2.browser_settings_atypical) flags it.
}

func TestH2Akamai(t *testing.T) {
	f := H2Fingerprint{
		Settings:     []H2Setting{{1, 65536}, {2, 0}, {4, 6291456}, {6, 262144}},
		WindowUpdate: 15663105,
		PseudoOrder:  []string{"m", "a", "s", "p"},
	}
	want := "1:65536;2:0;4:6291456;6:262144|15663105|0|m,a,s,p"
	if got := f.Akamai(); got != want {
		t.Errorf("akamai=%q want %q", got, want)
	}
}

func TestHeaderAnomalies(t *testing.T) {
	h := HeaderInfo{
		Method: "GET", Version: "20", IsH2: true, CasingReliable: true,
		Names:     []string{"user-agent", "Accept"}, // uppercase Accept in h2 = malformed
		UserAgent: "Mozilla/5.0 Chrome/126",
	}
	if !h.HasUppercaseInH2() {
		t.Error("expected uppercase-in-h2 detected")
	}
	if h.SecFetchPresent() {
		t.Error("no sec-fetch expected")
	}
	h2 := HeaderInfo{Names: []string{"sec-fetch-site", "sec-ch-ua"}}
	if !h2.SecFetchPresent() || !h2.SecCHUAPresent() {
		t.Error("expected sec-fetch and sec-ch-ua present")
	}
}

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

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

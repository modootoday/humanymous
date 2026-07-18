package scoring

import (
	"fmt"

	"github.com/modootoday/humanymous/internal/signals"
)

// crosscheck.go generates L6 consistency checks by diffing the claimed identity
// (spoofable UA/UA-CH) against harder-to-spoof observations (TLS/H2/header/JS).
// See SoT-02 §L6. Each CrossCheck carries its own weight; inconsistent ones
// contribute to the risk score as network-layer signals.

// crossWeights are the L6 weights (SoT-02 §L6 table / SoT-05 §3).
var crossWeights = map[string]float64{
	"x.ua_vs_ja4":      50,
	"x.ua_vs_h2":       45,
	"x.uach_present":   40,
	"x.uach_vs_js":     35,
	"x.ua_vs_header":   35,
	"x.non_browser_ua": 45,
	"x.browser_no_js":  45,
}

// CrossChecks builds the L6 CrossCheck list for a session.
func CrossChecks(r *signals.SessionReport) []signals.CrossCheck {
	ua := r.Client.UserAgent
	if ua == "" {
		ua = "" // network-only session (HTTP client); handled below
	}
	claimed := claimedEngine(ua)
	var out []signals.CrossCheck

	// A non-browser UA (library default) is itself a strong tell.
	if r.Client.UserAgent != "" && !isBrowserUA(r.Client.UserAgent) {
		out = append(out, mkCross("x.non_browser_ua",
			[]string{"client.userAgent"},
			"UA looks like a browser", "UA matches a known HTTP library",
			false, fmt.Sprintf("UA=%q not a browser", r.Client.UserAgent)))
	}

	// A browser-claiming UA that delivered ZERO client-side JS evidence never
	// executed as a browser: no WASM L1-L3 signals, no behavior, no advanced or
	// environment probe. This catches a header-spoofing HTTP parrot (curl_cffi /
	// uTLS) that fakes sec-ch-ua/sec-fetch but cannot run our JS/WASM (SoT-14).
	if isBrowserUA(r.Client.UserAgent) {
		noJS := len(r.Client.Signals) == 0 &&
			!r.Client.Advanced.Probed && !r.Client.Environment.Probed &&
			r.Client.Behavior.Mouse.Samples == 0 && r.Client.Behavior.Events.TotalEvents == 0 &&
			r.Client.EngineVersion == ""
		out = append(out, mkCross("x.browser_no_js",
			[]string{"client.userAgent", "client.signals", "client.advanced"},
			"browser UA implies JS/WASM execution", "no client-side evidence delivered",
			!noJS, "browser UA but zero JS execution evidence (HTTP parrot)"))
	}

	// UA-vs-engine checks require a definite browser claim to diff against.
	// An empty or library UA has no "claimed browser engine", so skip them
	// (the non-browser case is covered by x.non_browser_ua above).
	hasBrowserClaim := claimed == "chrome" || claimed == "firefox" || claimed == "safari"

	// UA engine vs observed TLS (JA4) engine.
	if hasBrowserClaim && r.Network.JA4Engine != "" {
		consistent := engineConsistent(claimed, r.Network.JA4Engine)
		out = append(out, mkCross("x.ua_vs_ja4",
			[]string{"client.userAgent", "network.ja4"},
			"UA="+claimed, "JA4 engine="+r.Network.JA4Engine,
			consistent, "TLS stack vs claimed browser"))
	}

	// UA engine vs observed HTTP/2 engine.
	if hasBrowserClaim && r.Network.H2Engine != "" {
		consistent := engineConsistent(claimed, r.Network.H2Engine)
		out = append(out, mkCross("x.ua_vs_h2",
			[]string{"client.userAgent", "network.akamaiH2"},
			"UA="+claimed, "H2 engine="+r.Network.H2Engine,
			consistent, "HTTP/2 profile vs claimed browser"))
	}

	// Chromium UA must send Sec-CH-UA over HTTPS.
	if claimsChromium(ua) {
		out = append(out, mkCross("x.uach_present",
			[]string{"client.userAgent", "network.secChUa"},
			"Chromium always sends Sec-CH-UA", "Sec-CH-UA present",
			r.Network.SecCHUAPresent, "UA-CH presence"))
		out = append(out, mkCross("x.ua_vs_header",
			[]string{"client.userAgent", "network.secFetch"},
			"Chromium sends sec-fetch-*", "sec-fetch present",
			r.Network.SecFetchPresent, "sec-fetch presence"))
	}

	// UA-CH headers vs JS navigator.userAgentData (both surfaces must agree).
	if len(r.Client.UAClientHints.Brands) > 0 && r.Network.SecCHUAPresent {
		consistent := uachMatchesJS(r)
		out = append(out, mkCross("x.uach_vs_js",
			[]string{"network.secChUa", "client.uaClientHints"},
			"header UA-CH", "JS userAgentData",
			consistent, "UA-CH header vs JS"))
	}

	return out
}

// uachMatchesJS is a placeholder consistency check: in this reference build the
// server stores header UA-CH into the same struct, so we compare platform.
// A real deployment would diff brand+version+mobile byte-for-byte.
func uachMatchesJS(r *signals.SessionReport) bool {
	// If the client reported a platform, it must be non-empty and plausible.
	return r.Client.UAClientHints.Platform != ""
}

func mkCross(id string, inputs []string, claim, observed string, consistent bool, notes string) signals.CrossCheck {
	w := crossWeights[id]
	score := 0.0
	if !consistent {
		score = w
	}
	return signals.CrossCheck{
		ID:         id,
		Inputs:     inputs,
		Claim:      claim,
		Observed:   observed,
		Consistent: consistent,
		Weight:     w,
		Score:      score,
		Notes:      notes,
	}
}

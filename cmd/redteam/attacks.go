package main

import (
	"time"

	utls "github.com/refraction-networking/utls"
)

// attacks.go implements the aggressive evasions. Each spoofs browser headers
// (so the L6 header checks stay quiet) and signs RIT correctly where it should,
// isolating the specific bypass the Blue engine must catch (SoT-07/12).

const ritW = 10

func nowTB() uint64 { return uint64(time.Now().Unix() / ritW) }

// signedCollect POSTs a properly RIT-signed /api/collect with the current seed
// and returns the verdict plus the server's rotated seed for the next request.
func signedCollect(hello utls.ClientHelloID, ua, cookie, sid string, seed []byte, n uint64, body string) (map[string]any, []byte, error) {
	hdr := withBrowserHeaders(nil)
	if seed != nil {
		tb := nowTB()
		hdr["X-HM-Token"] = ritToken(seed, sid, n, tb, body)
		hdr["X-HM-N"] = itoa(n)
		hdr["X-HM-TB"] = itoa(tb)
	}
	return collectSeed(hello, ua, cookie, hdr, body)
}

// jsEvidenceBody carries JS-execution + human-shaped behavior so HR-10 (no client) and
// HR-18 (browser-no-js) stay quiet — letting a scenario ISOLATE its specific header / TLS /
// token tell instead of collapsing into the no-JS parrot rule.
const jsEvidenceBody = `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",` +
	`"engineVersion":"wasm-1.0.0","advanced":{"probed":true},"environment":{"probed":true},` +
	`"behavior":{"durationS":3,"mouse":{"samples":30,"velocityStdDev":0.5,"straightLineFrac":0.2,"meanJerk":0.4},` +
	`"key":{"keystrokes":10,"meanFlightMs":120,"flightStdDevMs":35,"dwellStdDevMs":28},` +
	`"events":{"totalEvents":40,"untrustedFrac":0}},"signals":[]}`

// browserHeadersExcept returns the spoofed Chrome header set minus the given keys, so a
// scenario can drop exactly ONE header family (Sec-CH-UA, Sec-Fetch-*) and isolate the
// cross-check that catches its absence.
func browserHeadersExcept(drop ...string) map[string]string {
	h := browserHeaders()
	for _, k := range drop {
		delete(h, k)
	}
	return h
}

// coherentBrowser is the T4 DETECTION CEILING: a fully COHERENT session — real Chrome TLS
// (JA4=chrome), the full Sec-CH-UA / Sec-Fetch header set, every advanced capability present
// and self-consistent (WebGL and WebGPU vendors agree, Widevine/voices/media all present), and
// rich human-shaped behavior (high-variance mouse + human typing, no synthetic events). Every
// plane reconciles, so it SCORES ALLOW — the honest limit where detection alone cannot separate
// an engine-level spoof (BotBrowser-class) from a real human. Retained to keep the ceiling honest.
func coherentBrowser() (map[string]any, error) {
	cookie, _, _, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	body := `{"userAgent":"` + chromeUA + `","engineVersion":"wasm-1.0.0",` +
		`"advanced":{"probed":true,"mediaDeviceCount":3,"hasAudioInput":true,"hasVideoInput":true,"voiceCount":200,` +
		`"widevineSupported":true,"webgpuPresent":true,"webgpuVendor":"nvidia","webglVendor":"NVIDIA Corporation / NVIDIA GeForce RTX 3080",` +
		`"audioSampleRate":48000,"connectionPresent":true,"connectionRtt":50,"batteryPresent":true,"batteryLevel":0.8,` +
		`"timezoneIana":"America/New_York","language":"en-US","colorGamut":"srgb","maxTouchPoints":0},` +
		`"environment":{"probed":true},` +
		`"behavior":{"durationS":8,"mouse":{"samples":45,"velocityStdDev":0.6,"straightLineFrac":0.15,"accelEntropy":2.1,"meanJerk":0.4,"meanCurvature":0.3,"coalescedRatio":3.0},` +
		`"key":{"keystrokes":14,"meanDwellMs":95,"dwellStdDevMs":28,"meanFlightMs":140,"flightStdDevMs":35},` +
		`"events":{"totalEvents":60,"untrustedFrac":0,"clickCount":1}},"signals":[]}`
	v, _ := collect(utls.HelloChrome_Auto, chromeUA, cookie, withBrowserHeaders(nil), body)
	return v, nil
}

// nonBrowserUA: the cheapest bot — a bare HTTP library (library User-Agent, no browser
// headers, no JS). The UA is not a browser at all -> x.non_browser_ua (w45) + HR-10.
func nonBrowserUA() (map[string]any, error) {
	cookie, err := sessionStock()
	if err != nil {
		return nil, err
	}
	return collectStock("python-requests/2.31.0", cookie, nil, `{"userAgent":"python-requests/2.31.0","signals":[]}`)
}

// secCHUAAbsent: a Chromium UA (with JS evidence) that omits the Sec-CH-UA client hint a
// real Chromium always sends -> x.uach_present cross-check fails (w40) -> CHALLENGE.
func secCHUAAbsent() (map[string]any, error) {
	cookie, _, _, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	hdr := browserHeadersExcept("sec-ch-ua", "sec-ch-ua-mobile", "sec-ch-ua-platform")
	v, _ := collect(utls.HelloChrome_Auto, chromeUA, cookie, hdr, jsEvidenceBody)
	return v, nil
}

// secFetchAbsent: a Chrome UA (with JS evidence) that omits the Sec-Fetch-* metadata a real
// Chrome always sends -> l5.header.sec_fetch_missing + x.ua_vs_header -> CHALLENGE.
func secFetchAbsent() (map[string]any, error) {
	cookie, _, _, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	hdr := browserHeadersExcept("sec-fetch-site", "sec-fetch-mode", "sec-fetch-dest")
	v, _ := collect(utls.HelloChrome_Auto, chromeUA, cookie, hdr, jsEvidenceBody)
	return v, nil
}

// ritAbsent: an API client that never presents a RIT anti-tamper token. The first tokenless
// call gets the one-shot bootstrap grace; the SECOND emits l5.rit.absent -> HR-17.
func ritAbsent() (map[string]any, error) {
	cookie, _, _, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	// No X-HM-Token headers on either request (collect() adds none by default).
	_, _ = collect(utls.HelloChrome_Auto, chromeUA, cookie, withBrowserHeaders(nil), jsEvidenceBody)
	v, _ := collect(utls.HelloChrome_Auto, chromeUA, cookie, withBrowserHeaders(nil), jsEvidenceBody)
	return v, nil
}

// ja4Churn: three or more DISTINCT TLS fingerprints (spanning engine families) inside one
// cookied session -> l5.traffic.engine_rotation / ja4_rotation -> HR-14. A real browser keeps
// one TLS stack per session; churning it is a parrot rotating presets.
func ja4Churn() (map[string]any, error) {
	cookie, seed, n, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	hellos := []utls.ClientHelloID{utls.HelloChrome_100, utls.HelloFirefox_Auto, utls.HelloChrome_Auto}
	var v map[string]any
	for i, h := range hellos {
		var s []byte
		v, s, _ = signedCollect(h, chromeUA, cookie, sid, seed, n+uint64(i+1), jsEvidenceBody)
		if s != nil {
			seed = s
		}
	}
	return v, nil
}

// multiAxisRotate: rotate the User-Agent AND the TLS engine together in one session
// -> HR-15 (ua_rotation + ja4_rotation/engine axis), a coordinated multi-axis rotation a
// single browser never performs.
func multiAxisRotate() (map[string]any, error) {
	cookie, seed, n, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	_, seed1, _ := signedCollect(utls.HelloChrome_Auto, chromeUA, cookie, sid, seed, n+1, jsEvidenceBody)
	altUA := "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36"
	v, _, err := signedCollect(utls.HelloFirefox_Auto, altUA, cookie, sid, seed1, n+2, jsEvidenceBody)
	return v, err
}

// greaseAbsentJS: a no-GREASE (Go-default) TLS stack under a Chrome UA that DOES run JS (so
// HR-18 stays quiet), isolating the network tells: l5.tls.grease_absent (a real browser
// always sends a GREASE value) + x.ua_vs_ja4 (UA says Chrome, JA4 says Go).
func greaseAbsentJS() (map[string]any, error) {
	cookie, err := sessionStock()
	if err != nil {
		return nil, err
	}
	return collectStock(chromeUA, cookie, withBrowserHeaders(nil), jsEvidenceBody)
}

// tlsRotate: two cookied requests in one session with different TLS stacks
// (Chrome then Firefox) -> l5.traffic.engine_rotation -> HR-14 DENY.
func tlsRotate() (map[string]any, error) {
	cookie, seed, n, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	_, seed1, _ := signedCollect(utls.HelloChrome_Auto, chromeUA, cookie, sid, seed, n+1, collectBody)
	v, _, err := signedCollect(utls.HelloFirefox_Auto, chromeUA, cookie, sid, seed1, n+2, collectBody)
	return v, err
}

// tlsStatic: a static uTLS Chrome parrot that (unlike real Chrome 110+) sends
// the SAME extension order on every connection. Several cookied requests in one
// session all carry the identical ClientHello -> l5.traffic.tls_no_permutation
// -> HR-14 DENY (SoT-14 §B1).
func tlsStatic() (map[string]any, error) {
	cookie, seed, n, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	var v map[string]any
	for i := uint64(1); i <= 4; i++ {
		var s []byte
		// HelloChrome_100 is a pre-110 fingerprint that does NOT permute its
		// extension order -> every connection is byte-identical (static parrot).
		v, s, _ = signedCollect(utls.HelloChrome_100, chromeUA, cookie, sid, seed, n+i, collectBody)
		if s != nil {
			seed = s
		}
	}
	return v, nil
}

// uaRotate: two cookied requests with different User-Agents (same TLS)
// -> l5.traffic.ua_rotation.
func uaRotate() (map[string]any, error) {
	cookie, seed, n, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	_, seed1, _ := signedCollect(utls.HelloChrome_Auto, chromeUA, cookie, sid, seed, n+1, collectBody)
	altUA := "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36"
	v, _, err := signedCollect(utls.HelloChrome_Auto, altUA, cookie, sid, seed1, n+2, collectBody)
	return v, err
}

// ritReplay: one valid signed request (advances the counter), then REPLAY the
// same token -> the counter is stale -> l5.rit.stale_replay.
func ritReplay() (map[string]any, error) {
	cookie, seed, n, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	tb := nowTB()
	tok := ritToken(seed, sid, n+1, tb, collectBody)
	replayHdr := withBrowserHeaders(map[string]string{
		"X-HM-Token": tok, "X-HM-N": itoa(n + 1), "X-HM-TB": itoa(tb),
	})
	// request 1: valid (server advances to n+1).
	_, _ = collect(utls.HelloChrome_Auto, chromeUA, cookie, replayHdr, collectBody)
	// request 2: replay the SAME token/counter.
	return collect(utls.HelloChrome_Auto, chromeUA, cookie, replayHdr, collectBody)
}

// ritTamper: sign one body, send a DIFFERENT body with that token
// -> the HMAC over the observed body fails -> l5.rit.header_tampered.
func ritTamper() (map[string]any, error) {
	cookie, seed, n, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	tb := nowTB()
	tok := ritToken(seed, sid, n+1, tb, collectBody) // signs collectBody
	hdr := withBrowserHeaders(map[string]string{
		"X-HM-Token": tok, "X-HM-N": itoa(n + 1), "X-HM-TB": itoa(tb),
	})
	tamperedBody := `{"userAgent":"Mozilla/5.0 Chrome/126","signals":[],"injected":"evil"}`
	return collect(utls.HelloChrome_Auto, chromeUA, cookie, hdr, tamperedBody)
}

// distributed simulates a scraper behind a rotating residential-proxy pool: many
// requests carry the SAME device fingerprint but a DIFFERENT exit IP (X-Forwarded
// -For). Each session looks legit in isolation (browser headers + a client report
// with evidence), but the shared fingerprint across many subnets is unmasked by
// cross-session correlation -> l5.correlation.proxy_rotation -> HR-19 DENY (SoT-15).
func distributed() (map[string]any, error) {
	// A fixed device fingerprint + a client report that carries JS-execution
	// evidence, so per-session header/JS checks stay quiet and only correlation fires.
	const fpID = "deadbeefcafe0001deadbeef"
	body := `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",` +
		`"fingerprintId":"` + fpID + `","engineVersion":"wasm-1.0.0","advanced":{"probed":true},` +
		`"environment":{"probed":true},"behavior":{"mouse":{"samples":30,"velocityStdDev":0.4},` +
		`"events":{"totalEvents":40}},"signals":[]}`

	proxies := []string{"203.0.113.7", "198.51.100.23", "192.0.2.44", "203.0.113.99"}
	var v map[string]any
	for _, ip := range proxies {
		cookie, _, _, err := session(utls.HelloChrome_Auto)
		if err != nil {
			return nil, err
		}
		hdr := withBrowserHeaders(map[string]string{"X-Forwarded-For": ip})
		v, _ = collect(utls.HelloChrome_Auto, chromeUA, cookie, hdr, body)
	}
	return v, nil
}

// privacyEvasion is the distributed residential-proxy-rotation scraper (as above) that
// ALSO claims genuine privacy tooling (environment.adBlock=true) to try to disarm HR-19 —
// the round-5 regression where gating the proxy_rotation hard-DENY on a CLIENT-forgeable
// privacy flag let a scraper post adBlock:true and reach ALLOW. The Blue engine must NOT
// honor a self-asserted privacy flag to exempt a server-authoritative correlation rule:
// proxy_rotation still -> HR-19 DENY. Retained as a permanent regression wargame case.
func privacyEvasion() (map[string]any, error) {
	const fpID = "deadbeefcafe0002deadbeef"
	// Same shape as distributed(), but the report now claims ad-block + GPC/DNT privacy.
	body := `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",` +
		`"fingerprintId":"` + fpID + `","engineVersion":"wasm-1.0.0","advanced":{"probed":true},` +
		`"environment":{"probed":true,"adBlock":true,"gpc":true,"doNotTrack":"1"},` +
		`"behavior":{"mouse":{"samples":30,"velocityStdDev":0.4},"events":{"totalEvents":40}},"signals":[]}`
	proxies := []string{"203.0.113.7", "198.51.100.23", "192.0.2.44", "203.0.113.99"}
	var v map[string]any
	for _, ip := range proxies {
		cookie, _, _, err := session(utls.HelloChrome_Auto)
		if err != nil {
			return nil, err
		}
		hdr := withBrowserHeaders(map[string]string{"X-Forwarded-For": ip})
		v, _ = collect(utls.HelloChrome_Auto, chromeUA, cookie, hdr, body)
	}
	return v, nil
}

// signalForgery is a borderline-suspicious browser-ish client (real Chrome uTLS + JS-
// execution evidence, so L5/L6/HR-10/HR-18 stay quiet) carrying a couple of WebGL bot
// tells that put it in the CHALLENGE band — that then FORGES the server-only trust-upgrade
// signals l7.pass.solved / l7.pow.solved in its own /api/collect report to launder the
// CHALLENGE to ALLOW (the round-3 provenance blocker). The Blue engine strips any client-
// supplied signal in the reserved L5/L6/L7 namespace at ingest and honors an upgrade ONLY
// from a server-minted signal, so the forged upgrades are inert and the verdict stays
// CHALLENGE/DENY (never ALLOW). Retained as a permanent regression wargame case.
func signalForgery() (map[string]any, error) {
	// A borderline session: a FIREFOX TLS ClientHello while the report + UA claim Chrome, so
	// the UA-vs-JA4 cross-check disagrees and puts the session in the score-based CHALLENGE
	// band (over HTTP/1.1 there is no observed H2 fingerprint, so HR-2 — which needs BOTH the
	// JA4 and H2 cross-checks to fail — does not fire; it stays a score CHALLENGE, not a hard
	// DENY). It then FORGES the server-only trust-upgrades to try to reach ALLOW.
	cookie, _, _, err := session(utls.HelloFirefox_Auto)
	if err != nil {
		return nil, err
	}
	body := `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",` +
		`"engineVersion":"wasm-1.0.0","advanced":{"probed":true},"environment":{"probed":true},` +
		`"behavior":{"mouse":{"samples":30,"velocityStdDev":0.4},"events":{"totalEvents":40}},` +
		`"signals":[{"id":"l7.pass.solved","verdict":"OK","collected":"server","score":1},` +
		`{"id":"l7.pow.solved","verdict":"OK","collected":"server","score":1}]}`
	v, _ := collect(utls.HelloFirefox_Auto, chromeUA, cookie, withBrowserHeaders(nil), body)
	return v, nil
}

// flood hammers /api/collect with many rapid requests from one TLS fingerprint
// (application-layer DoS / credential-stuffing velocity). The Blue engine meters
// by JA4+subnet (not IP), so the flood is caught even though each request opens a
// fresh connection -> l5.abuse.flood -> a score-based CHALLENGE (not a categorical
// DENY, so a shared CGNAT subnet is not locked out) plus the escalating ban ladder
// (SoT-17/SoT-27).
func flood() (map[string]any, error) {
	cookie, _, _, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	// carry JS-execution evidence so the flood signal (not browser_no_js) is the
	// clean catch — models a real-browser-driven flood.
	body := `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",` +
		`"engineVersion":"wasm-1.0.0","advanced":{"probed":true},"environment":{"probed":true},` +
		`"behavior":{"mouse":{"samples":30,"velocityStdDev":0.4},"events":{"totalEvents":40}},"signals":[]}`
	var v map[string]any
	for i := 0; i < 90; i++ {
		v, _ = collect(utls.HelloChrome_Auto, chromeUA, cookie, withBrowserHeaders(nil), body)
	}
	return v, nil
}

// xffSpoof forges a PRIVATE X-Forwarded-For (+ X-Real-IP) to impersonate a
// trusted-LAN client. If the engine naively trusts the forwarding header from
// its NAT/proxy peer, the client sheds the datacenter/IP-intel signals; a single
// (non-rotated) source also dodges cross-session correlation (HR-19). Otherwise
// clean Chrome uTLS + a fabricated-but-consistent client report, so the forged
// IP is the ONLY variable. Blue must reject a forwarded client IP that is
// private/reserved — a real reverse proxy forwards the client's PUBLIC address,
// never a LAN one (l5.header.forwarded_private).
func xffSpoof() (map[string]any, error) {
	const fpID = "5p00fed15p00fed15p00fed1"
	body := `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",` +
		`"fingerprintId":"` + fpID + `","engineVersion":"wasm-1.0.0","advanced":{"probed":true},` +
		`"environment":{"probed":true},"behavior":{"mouse":{"samples":32,"velocityStdDev":0.5},` +
		`"events":{"totalEvents":44}},"signals":[]}`
	cookie, _, _, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	// forge "I'm on your LAN" so IP-intel treats the source as trusted/non-datacenter
	hdr := withBrowserHeaders(map[string]string{
		"X-Forwarded-For": "10.20.30.40",
		"X-Real-IP":       "10.20.30.40",
	})
	return collect(utls.HelloChrome_Auto, chromeUA, cookie, hdr, body)
}

func itoa(n uint64) string {
	if n == 0 {
		return "0"
	}
	var b [20]byte
	i := len(b)
	for n > 0 {
		i--
		b[i] = byte('0' + n%10)
		n /= 10
	}
	return string(b[i:])
}

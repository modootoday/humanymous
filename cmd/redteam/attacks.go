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
	return signedCollectHdr(hello, ua, cookie, sid, seed, n, body, nil)
}

// signedCollectHdr is signedCollect with extra request headers (e.g. XFF for proxy residual).
func signedCollectHdr(hello utls.ClientHelloID, ua, cookie, sid string, seed []byte, n uint64, body string, extra map[string]string) (map[string]any, []byte, error) {
	hdr := withBrowserHeaders(extra)
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

// proxyBody is a coherent-ish client report so HR-10/HR-18 stay quiet while isolating
// the proxy/VPN residual under test. Each attack MUST use a unique fingerprintId so
// sequential catalog runs do not cross-fire HR-19 via shared-fp correlation.
func proxyBody(fpID string) string {
	return `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",` +
		`"fingerprintId":"` + fpID + `","engineVersion":"wasm-1.0.0",` +
		`"advanced":{"probed":true,"audioSampleRate":48000,"voiceCount":12,"mediaDeviceCount":3,` +
		`"widevineSupported":true,"timezoneIana":"America/New_York","language":"en-US",` +
		`"colorGamut":"srgb","maxTouchPoints":0},` +
		`"environment":{"probed":true},"behavior":{"mouse":{"samples":32,"velocityStdDev":0.5},` +
		`"events":{"totalEvents":44}},"signals":[]}`
}

// squidForward models traffic through an open Squid forward proxy. Squid injects
// Via + X-Cache on the origin request — a direct browser never does.
// Blue: l5.header.proxy_hop → HR-24 CHALLENGE.
func squidForward() (map[string]any, error) {
	cookie, _, _, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	hdr := withBrowserHeaders(map[string]string{
		"Via":             `1.1 squid.lab (squid/5.7)`,
		"X-Cache":         "MISS from squid.lab",
		"X-Cache-Lookup":  "MISS from squid.lab:3128",
		"X-Forwarded-For": "203.0.113.40",
	})
	return collect(utls.HelloChrome_Auto, chromeUA, cookie, hdr, proxyBody("squ1df0rward000001squ1"))
}

// openvpnExit models OpenVPN (L3): TCP hits origin from the VPN exit (XFF from a
// trusted Docker peer) while WebRTC STUN still reveals a different public IP —
// classic VPN leak. Blue: l5.proxy.vpn_webrtc_leak → HR-24.
func openvpnExit() (map[string]any, error) {
	const body = `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",` +
		`"fingerprintId":"0penvpn0000000010penvpn","engineVersion":"wasm-1.0.0",` +
		`"advanced":{"probed":true,"audioSampleRate":48000,"voiceCount":12,"mediaDeviceCount":3,` +
		`"widevineSupported":true,"timezoneIana":"America/New_York","language":"en-US",` +
		`"webrtcHostAddrs":["10.8.0.2","192.168.1.42"],` +
		`"webrtcPublicAddrs":["198.51.100.77"],` +
		`"colorGamut":"srgb","maxTouchPoints":0},` +
		`"environment":{"probed":true},"behavior":{"mouse":{"samples":32,"velocityStdDev":0.5},` +
		`"events":{"totalEvents":44}},"signals":[]}`
	cookie, _, _, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	hdr := withBrowserHeaders(map[string]string{
		"X-Forwarded-For": "203.0.113.88",
	})
	return collect(utls.HelloChrome_Auto, chromeUA, cookie, hdr, body)
}

// torExit models Tor Browser behind a Tor circuit residual:
//  1. ≥3-hop X-Forwarded-For (entry/middle/exit style) → l5.proxy.tor_circuit → HR-24
//  2. same Tor-Browser-shaped fingerprint across ≥3 distinct exit subnets → HR-19
//
// Real Tor exit lists wire via SetTorExitCIDRs → l5.ip.tor_exit (also HR-24).
// Firefox uTLS + Tor Browser UA so the path is not a Chrome HTTP parrot.
func torExit() (map[string]any, error) {
	const torUA = "Mozilla/5.0 (Windows NT 10.0; rv:115.0) Gecko/20100101 Firefox/115.0"
	const body = `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; rv:115.0) Gecko/20100101 Firefox/115.0",` +
		`"fingerprintId":"t0rbr0wser000001t0rbr0","engineVersion":"wasm-1.0.0",` +
		`"advanced":{"probed":true,"audioSampleRate":44100,"voiceCount":0,"mediaDeviceCount":0,` +
		`"widevineSupported":false,"timezoneIana":"UTC","language":"en-US",` +
		`"webrtcHostAddrs":[],"webrtcPublicAddrs":[],` +
		`"colorGamut":"srgb","maxTouchPoints":0},` +
		`"environment":{"probed":true,"adBlock":true,"gpc":true,"doNotTrack":"1","trackingProtection":true},` +
		`"behavior":{"mouse":{"samples":28,"velocityStdDev":0.45},"events":{"totalEvents":36}},"signals":[]}`
	// First hit: full circuit multi-hop residual (entry, middle, exit).
	cookie, seed, n, err := session(utls.HelloFirefox_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	var v map[string]any
	var s []byte
	v, s, _ = signedCollectHdr(utls.HelloFirefox_Auto, torUA, cookie, sid, seed, n+1, body,
		map[string]string{"X-Forwarded-For": "185.220.101.1, 185.220.102.2, 203.0.113.50"})
	if s != nil {
		seed = s
	}
	// Then rotate exit subnets under the same fingerprint (Tor exit churn) → HR-19.
	exits := []string{"198.51.100.11", "203.0.113.22", "192.0.2.33"}
	for i, ip := range exits {
		// Fresh session cookie per exit would break correlation; keep same cookie + fp.
		v, s, _ = signedCollectHdr(utls.HelloFirefox_Auto, torUA, cookie, sid, seed, n+uint64(i+2), body,
			map[string]string{"X-Forwarded-For": ip})
		if s != nil {
			seed = s
		}
	}
	return v, nil
}

// --- Diverse anonymous-proxy / VPN evasion residuals (red catalog expansion) ---

// anonProxyChain models a free/anonymous open-proxy chain (4+ hops) that still
// leaks Proxy-Connection and a long XFF chain — elite lists without Via.
// Blue: l5.proxy.anon_chain (+ proxy_hop) → HR-24.
func anonProxyChain() (map[string]any, error) {
	cookie, _, _, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	hdr := withBrowserHeaders(map[string]string{
		"Proxy-Connection": "keep-alive",
		// entry → relay1 → relay2 → exit
		"X-Forwarded-For": "203.0.113.1, 198.51.100.2, 192.0.2.3, 203.0.113.99",
	})
	return collect(utls.HelloChrome_Auto, chromeUA, cookie, hdr, proxyBody("an0npr0xychain0001an0n"))
}

// eliteAnonProxy models an "elite" anonymous proxy that strips Via/X-Cache but
// still emits RFC 7239 Forwarded with multiple for= tokens (and by=).
// Blue: l5.header.proxy_hop (forwarded-multi) → HR-24.
func eliteAnonProxy() (map[string]any, error) {
	cookie, _, _, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	hdr := withBrowserHeaders(map[string]string{
		"Forwarded":       `for=203.0.113.10;proto=https, for=198.51.100.20;by=proxy.elite.lab`,
		"X-Forwarded-For": "203.0.113.10, 198.51.100.20",
	})
	return collect(utls.HelloChrome_Auto, chromeUA, cookie, hdr, proxyBody("el1tean0npr0xy0001el1t"))
}

// cdnIPSpoof forges CDN/edge client-identity headers (CF-Connecting-IP /
// True-Client-IP) to launder a residential victim IP while the real exit is a
// proxy. A browser never sends these to origin.
// Blue: l5.header.client_ip_spoof → HR-24.
func cdnIPSpoof() (map[string]any, error) {
	cookie, _, _, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	hdr := withBrowserHeaders(map[string]string{
		"X-Forwarded-For":          "203.0.113.50",
		"CF-Connecting-IP":         "8.8.8.8",
		"True-Client-IP":           "1.1.1.1",
		"X-Client-IP":              "9.9.9.9",
		"X-Original-Forwarded-For": "8.8.8.8",
	})
	return collect(utls.HelloChrome_Auto, chromeUA, cookie, hdr, proxyBody("cdn1psp00f00000001cdn1"))
}

// proxyUARotate combines residential exit rotation with mid-session User-Agent
// rotation (multi-axis evasion against simple IP reputation + sticky UA).
// Blue: ua_rotation + ip_hop → HR-15 (and/or proxy_rotation → HR-19).
func proxyUARotate() (map[string]any, error) {
	ua1 := chromeUA
	ua2 := "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
	cookie, seed, n, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	body1 := `{"userAgent":"` + ua1 + `","fingerprintId":"pr0xyuar0tate0001pr0x","engineVersion":"wasm-1.0.0",` +
		`"advanced":{"probed":true,"audioSampleRate":48000,"voiceCount":12,"mediaDeviceCount":3,` +
		`"widevineSupported":true,"timezoneIana":"America/New_York","language":"en-US",` +
		`"colorGamut":"srgb","maxTouchPoints":0},` +
		`"environment":{"probed":true},"behavior":{"mouse":{"samples":32,"velocityStdDev":0.5},` +
		`"events":{"totalEvents":44}},"signals":[]}`
	var v map[string]any
	var s []byte
	v, s, _ = signedCollectHdr(utls.HelloChrome_Auto, ua1, cookie, sid, seed, n+1, body1,
		map[string]string{"X-Forwarded-For": "203.0.113.10"})
	if s != nil {
		seed = s
	}
	body2 := `{"userAgent":"` + ua2 + `","fingerprintId":"pr0xyuar0tate0001pr0x","engineVersion":"wasm-1.0.0",` +
		`"advanced":{"probed":true,"audioSampleRate":48000,"voiceCount":12,"mediaDeviceCount":3,` +
		`"widevineSupported":true,"timezoneIana":"America/New_York","language":"en-US",` +
		`"colorGamut":"srgb","maxTouchPoints":0},` +
		`"environment":{"probed":true},"behavior":{"mouse":{"samples":32,"velocityStdDev":0.5},` +
		`"events":{"totalEvents":44}},"signals":[]}`
	v, s, _ = signedCollectHdr(utls.HelloChrome_Auto, ua2, cookie, sid, seed, n+2, body2,
		map[string]string{"X-Forwarded-For": "198.51.100.20"})
	if s != nil {
		seed = s
	}
	v, _, _ = signedCollectHdr(utls.HelloChrome_Auto, ua2, cookie, sid, seed, n+3, body2,
		map[string]string{"X-Forwarded-For": "192.0.2.30"})
	return v, nil
}

// fpChurnProxy rotates fingerprintId mid-session on every exit hop to dodge
// HR-19's fingerprint|ja4 key while still riding a rotating anonymous-proxy pool.
// Blue: l5.correlation.fp_churn_proxy (mid-session) → HR-19 DENY.
func fpChurnProxy() (map[string]any, error) {
	exits := []struct {
		fp string
		ip string
	}{
		{"fpchurn000000000000001", "203.0.113.11"},
		{"fpchurn000000000000002", "198.51.100.22"},
		{"fpchurn000000000000003", "192.0.2.33"},
	}
	cookie, seed, n, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	var v map[string]any
	var s []byte
	for i, e := range exits {
		body := `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",` +
			`"fingerprintId":"` + e.fp + `","engineVersion":"wasm-1.0.0",` +
			`"advanced":{"probed":true,"audioSampleRate":48000,"voiceCount":12,"mediaDeviceCount":3,` +
			`"widevineSupported":true,"timezoneIana":"America/New_York","language":"en-US",` +
			`"colorGamut":"srgb","maxTouchPoints":0},` +
			`"environment":{"probed":true},"behavior":{"mouse":{"samples":32,"velocityStdDev":0.5},` +
			`"events":{"totalEvents":44}},"signals":[]}`
		v, s, _ = signedCollectHdr(utls.HelloChrome_Auto, chromeUA, cookie, sid, seed, n+uint64(i+1), body,
			map[string]string{"X-Forwarded-For": e.ip})
		if s != nil {
			seed = s
		}
	}
	return v, nil
}

// stackedProxyVPN models Squid forward-proxy hop ON TOP of a VPN exit with
// WebRTC leak (common "proxy over VPN" scraper stack).
// Blue: proxy_hop + vpn_webrtc_leak → HR-24.
func stackedProxyVPN() (map[string]any, error) {
	const body = `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",` +
		`"fingerprintId":"stackpr0xyvpn00001stac","engineVersion":"wasm-1.0.0",` +
		`"advanced":{"probed":true,"audioSampleRate":48000,"voiceCount":12,"mediaDeviceCount":3,` +
		`"widevineSupported":true,"timezoneIana":"America/New_York","language":"en-US",` +
		`"webrtcHostAddrs":["10.8.0.5"],"webrtcPublicAddrs":["198.51.100.88"],` +
		`"colorGamut":"srgb","maxTouchPoints":0},` +
		`"environment":{"probed":true},"behavior":{"mouse":{"samples":32,"velocityStdDev":0.5},` +
		`"events":{"totalEvents":44}},"signals":[]}`
	cookie, _, _, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	hdr := withBrowserHeaders(map[string]string{
		"Via":             `1.1 squid-over-vpn (squid/5.7)`,
		"X-Cache":         "MISS from squid-over-vpn",
		"X-Forwarded-For": "203.0.113.77",
	})
	return collect(utls.HelloChrome_Auto, chromeUA, cookie, hdr, body)
}

// socksExitHop models a SOCKS5 (or commercial VPN) L4 tunnel: no HTTP hop
// headers, mid-session exit hop + multi-hop XFF on the second request.
// Blue: ip_hop + xff_multi_hop → HR-24 (same plane as WireGuard residual).
func socksExitHop() (map[string]any, error) {
	const body = `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",` +
		`"fingerprintId":"s0cks5exith0p0001s0ck","engineVersion":"wasm-1.0.0",` +
		`"advanced":{"probed":true,"audioSampleRate":48000,"voiceCount":12,"mediaDeviceCount":3,` +
		`"widevineSupported":true,"timezoneIana":"America/New_York","language":"en-US",` +
		`"colorGamut":"srgb","maxTouchPoints":0},` +
		`"environment":{"probed":true},"behavior":{"mouse":{"samples":32,"velocityStdDev":0.5},` +
		`"events":{"totalEvents":44}},"signals":[]}`
	cookie, seed, n, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	var v map[string]any
	var s []byte
	v, s, _ = signedCollectHdr(utls.HelloChrome_Auto, chromeUA, cookie, sid, seed, n+1, body,
		map[string]string{"X-Forwarded-For": "203.0.113.40"})
	if s != nil {
		seed = s
	}
	v, _, _ = signedCollectHdr(utls.HelloChrome_Auto, chromeUA, cookie, sid, seed, n+2, body,
		map[string]string{"X-Forwarded-For": "198.51.100.41, 203.0.113.40"})
	return v, nil
}

// wireguardHop models WireGuard exit rotation in one session: two distinct public
// exits via XFF plus a multi-hop chain on the second request. RIT is signed so
// HR-17 stays quiet and HR-24 (ip_hop + xff_multi_hop) is the measured rule.
func wireguardHop() (map[string]any, error) {
	const body = `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",` +
		`"fingerprintId":"w1reguard0000001w1regu","engineVersion":"wasm-1.0.0",` +
		`"advanced":{"probed":true,"audioSampleRate":48000,"voiceCount":12,"mediaDeviceCount":3,` +
		`"widevineSupported":true,"timezoneIana":"America/New_York","language":"en-US",` +
		`"colorGamut":"srgb","maxTouchPoints":0},` +
		`"environment":{"probed":true},"behavior":{"mouse":{"samples":32,"velocityStdDev":0.5},` +
		`"events":{"totalEvents":44}},"signals":[]}`
	cookie, seed, n, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	var v map[string]any
	var s []byte
	v, s, _ = signedCollectHdr(utls.HelloChrome_Auto, chromeUA, cookie, sid, seed, n+1, body,
		map[string]string{"X-Forwarded-For": "203.0.113.10"})
	if s != nil {
		seed = s
	}
	v, _, _ = signedCollectHdr(utls.HelloChrome_Auto, chromeUA, cookie, sid, seed, n+2, body,
		map[string]string{"X-Forwarded-For": "198.51.100.20, 203.0.113.10"})
	return v, nil
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

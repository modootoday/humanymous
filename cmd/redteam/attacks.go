package main

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/modootoday/humanymous/internal/pow"
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

// powFastSolve exercises the /api/pow active-defense path (SoT-13) that NO other
// catalog profile reaches: it drives a borderline session into the CHALLENGE band,
// reads the server-issued X-HM-PoW work factor, solves it with a NATIVE Go SHA-256
// loop (orders of magnitude past the ~3 MH/s Web-Crypto ceiling a real browser JS
// solver is bounded by), and returns the solution within milliseconds of issuance.
//
// The Blue engine measures solve time from the FIRST issuance and, when a solution
// beats plausibleBrowserSolve(difficulty), mints l7.pow.too_fast -> HR-19 DENY, so a
// native/GPU PoW offloader cannot buy a trust upgrade by out-computing a browser.
// The reported verdict is the FINAL /api/pow re-score (the honest outcome of the
// active defense), with the issued difficulty and measured intent exposed for the
// ledger. If no challenge is issued, the collect verdict is reported instead.
func powFastSolve() (map[string]any, error) {
	// Borderline session: a Firefox TLS ClientHello while the UA + report claim Chrome,
	// so the UA-vs-JA4 cross-check disagrees and lands a score-based CHALLENGE (over
	// HTTP/1.1 there is no observed H2 fingerprint, so the HR-2 hard DENY — which needs
	// BOTH the JA4 and H2 cross-checks to fail — stays quiet; SoT-13 then issues PoW).
	cookie, _, _, err := session(utls.HelloFirefox_Auto)
	if err != nil {
		return nil, err
	}
	// A couple of WebGL/hardware contradictions push risk into a solid CHALLENGE so the
	// issued difficulty (and thus the too-fast wall-clock budget) is meaningful.
	body := `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",` +
		`"engineVersion":"wasm-1.0.0","advanced":{"probed":true,"webglVendor":"Google Inc. (Intel)","webglRenderer":"llvmpipe (LLVM 15)"},` +
		`"environment":{"probed":true},` +
		`"behavior":{"mouse":{"samples":8,"velocityStdDev":0.05},"events":{"totalEvents":4}},"signals":[]}`
	hdr := withBrowserHeaders(map[string]string{"User-Agent": chromeUA, "Content-Type": "application/json"})
	r, err := do(utls.HelloFirefox_Auto, "POST", "/api/collect", hdr, body, cookie)
	if err != nil {
		return nil, err
	}
	challenge := r.headers["X-Hm-Pow"]
	if challenge == "" {
		// No PoW issued (session not challenged or difficulty 0): report the collect verdict.
		var v map[string]any
		_ = json.Unmarshal(r.body, &v)
		if v == nil {
			v = map[string]any{}
		}
		v["powIssued"] = false
		return v, nil
	}
	// X-HM-PoW is "seed:difficulty:bucket".
	parts := strings.SplitN(challenge, ":", 3)
	if len(parts) != 3 {
		return nil, fmt.Errorf("malformed X-HM-PoW header: %q", challenge)
	}
	difficulty, err := strconv.Atoi(parts[1])
	if err != nil {
		return nil, fmt.Errorf("bad PoW difficulty %q: %w", parts[1], err)
	}
	bucket, err := strconv.ParseUint(parts[2], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("bad PoW bucket %q: %w", parts[2], err)
	}
	// Native solve: a tight Go SHA-256 loop finds the nonce in microseconds.
	nonce, ok := pow.Solve(parts[0], difficulty, 1<<27)
	if !ok {
		return nil, fmt.Errorf("failed to solve PoW at difficulty %d", difficulty)
	}
	// Submit immediately -> elapsed since issuance << plausibleBrowserSolve(d).
	solBody := fmt.Sprintf(`{"bucket":%d,"nonce":%q}`, bucket, nonce)
	pr, err := do(utls.HelloFirefox_Auto, "POST", "/api/pow",
		map[string]string{"User-Agent": chromeUA, "Content-Type": "application/json"}, solBody, cookie)
	if err != nil {
		return nil, err
	}
	var v map[string]any
	_ = json.Unmarshal(pr.body, &v)
	if v == nil {
		v = map[string]any{}
	}
	v["powIssued"] = true
	v["powDifficulty"] = difficulty
	return v, nil
}

// powLaunder probes whether the too-fast native-solver TELL is REMEMBERED. It solves
// the PoW natively (instant), submits once to reveal the native-solver speed (the server
// mints l7.pow.too_fast -> HR-19 DENY for that attempt), then WAITS until elapsed since
// issuance exceeds plausibleBrowserSolve(difficulty) and resubmits the SAME valid nonce.
//
// The question this round asks: does observing a solve no human could produce stick to the
// session, or can the native solver launder its own DENY into the pow.solved trust upgrade
// simply by timing the retry to look browser-plausible? handlePoW records nothing on a
// too-fast hit (powIssued/powDiff persist, powSolved stays false), so the delayed resubmit
// takes the success branch. The reported verdict is the SECOND (laundered) /api/pow re-score.
func powLaunder() (map[string]any, error) {
	cookie, _, _, err := session(utls.HelloFirefox_Auto)
	if err != nil {
		return nil, err
	}
	body := `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",` +
		`"engineVersion":"wasm-1.0.0","advanced":{"probed":true,"webglVendor":"Google Inc. (Intel)","webglRenderer":"llvmpipe (LLVM 15)"},` +
		`"environment":{"probed":true},` +
		`"behavior":{"mouse":{"samples":8,"velocityStdDev":0.05},"events":{"totalEvents":4}},"signals":[]}`
	hdr := withBrowserHeaders(map[string]string{"User-Agent": chromeUA, "Content-Type": "application/json"})
	r, err := do(utls.HelloFirefox_Auto, "POST", "/api/collect", hdr, body, cookie)
	if err != nil {
		return nil, err
	}
	challenge := r.headers["X-Hm-Pow"]
	if challenge == "" {
		var v map[string]any
		_ = json.Unmarshal(r.body, &v)
		if v == nil {
			v = map[string]any{}
		}
		v["powIssued"] = false
		return v, nil
	}
	parts := strings.SplitN(challenge, ":", 3)
	if len(parts) != 3 {
		return nil, fmt.Errorf("malformed X-HM-PoW header: %q", challenge)
	}
	difficulty, err := strconv.Atoi(parts[1])
	if err != nil {
		return nil, fmt.Errorf("bad PoW difficulty %q: %w", parts[1], err)
	}
	bucket, err := strconv.ParseUint(parts[2], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("bad PoW bucket %q: %w", parts[2], err)
	}
	nonce, ok := pow.Solve(parts[0], difficulty, 1<<27)
	if !ok {
		return nil, fmt.Errorf("failed to solve PoW at difficulty %d", difficulty)
	}
	solBody := fmt.Sprintf(`{"bucket":%d,"nonce":%q}`, bucket, nonce)
	powPost := func() map[string]any {
		pr, e := do(utls.HelloFirefox_Auto, "POST", "/api/pow",
			map[string]string{"User-Agent": chromeUA, "Content-Type": "application/json"}, solBody, cookie)
		if e != nil {
			return map[string]any{"error": e.Error()}
		}
		var v map[string]any
		_ = json.Unmarshal(pr.body, &v)
		if v == nil {
			v = map[string]any{}
		}
		return v
	}
	// First submit: instant -> reveals the native solver -> expected too-fast DENY.
	first := powPost()
	// Wait past the browser-plausible floor for this difficulty, then resubmit the SAME nonce.
	// plausibleBrowserSolve = 2^d / 3e6 s; add margin so elapsed since issuance clears it.
	floor := time.Duration(float64(uint64(1)<<uint(difficulty)) / 3_000_000.0 * float64(time.Second))
	time.Sleep(floor + 250*time.Millisecond)
	second := powPost()
	second["firstVerdict"] = first["verdict"]
	second["powDifficulty"] = difficulty
	second["laundered"] = second["ok"] == true || second["verdict"] == "ALLOW"
	return second, nil
}

// h2ProtocolSplit is the 2026 "protocol-split" evasion (web-research grounded): a client
// that passes TLS/JA4 as a real Chrome (uTLS Chrome ClientHello) and sends a coherent Chrome
// UA + full browser headers + JS-execution evidence — so every L5/L6 header and TLS check
// stays quiet — but carries a LIBRARY HTTP/2 fingerprint (Go's golang.org/x/net/http2 SETTINGS
// / WINDOW_UPDATE / pseudo-header layout) instead of Chrome's. Akamai-class detectors
// specifically target bots that spoof TLS but not the HTTP/2 frame layout; this isolates
// whether the engine derives H2Engine != Chrome and fires x.ua_vs_h2 / l5.http2.engine_mismatch,
// or lets an "unknown" h2 profile through (engineConsistent treats unknown as no-evidence).
func h2ProtocolSplit() (map[string]any, error) {
	// Establish a cookied session over the SAME h2 path and capture the RIT seed, so the
	// collects are correctly RIT-signed and HR-17 (rit.absent) stays quiet — isolating the
	// HTTP/2 engine mismatch as the ONLY tell.
	sess, err := doH2(utls.HelloChrome_Auto, "GET", "/api/session",
		withBrowserHeaders(map[string]string{"User-Agent": chromeUA}), "", "")
	if err != nil {
		return nil, err
	}
	cookie := sess.cookie
	sid := sidFromCookie(cookie)
	var sj struct {
		RitSeed string `json:"ritSeed"`
		RitN    uint64 `json:"ritN"`
	}
	_ = json.Unmarshal(sess.body, &sj)
	seed, _ := base64.RawURLEncoding.DecodeString(sj.RitSeed)
	n := sj.RitN
	// A FULLY COHERENT client report (every advanced capability present + self-consistent,
	// rich human-shaped behavior) — identical in spirit to the T4 coherent-ceiling body — so
	// the ONLY residual tell is the Go HTTP/2 frame layout. If this reaches ALLOW, the h2
	// fingerprint failed to separate a library h2 client from a real Chrome.
	body := `{"userAgent":"` + chromeUA + `","engineVersion":"wasm-1.0.0",` +
		`"advanced":{"probed":true,"mediaDeviceCount":3,"hasAudioInput":true,"hasVideoInput":true,"voiceCount":200,` +
		`"widevineSupported":true,"webgpuPresent":true,"webgpuVendor":"nvidia","webglVendor":"NVIDIA Corporation / NVIDIA GeForce RTX 3080",` +
		`"audioSampleRate":48000,"connectionPresent":true,"connectionRtt":50,"batteryPresent":true,"batteryLevel":0.8,` +
		`"timezoneIana":"America/New_York","language":"en-US","colorGamut":"srgb","maxTouchPoints":0},` +
		`"environment":{"probed":true},` +
		`"behavior":{"durationS":8,"mouse":{"samples":45,"velocityStdDev":0.6,"straightLineFrac":0.15,"accelEntropy":2.1,"meanJerk":0.4,"meanCurvature":0.3,"coalescedRatio":3.0},` +
		`"key":{"keystrokes":14,"meanDwellMs":95,"dwellStdDevMs":28,"meanFlightMs":140,"flightStdDevMs":35},` +
		`"events":{"totalEvents":60,"untrustedFrac":0,"clickCount":1}},"signals":[]}`
	v := map[string]any{}
	// Two signed collects: the h2 fingerprint is pinned on the first observation; the
	// second re-scores with the session's pinned h2 engine.
	for i := 0; i < 2; i++ {
		hdr := withBrowserHeaders(map[string]string{"User-Agent": chromeUA, "Content-Type": "application/json"})
		if seed != nil {
			tb := nowTB()
			nn := n + uint64(i+1)
			hdr["X-HM-Token"] = ritToken(seed, sid, nn, tb, body)
			hdr["X-HM-N"] = itoa(nn)
			hdr["X-HM-TB"] = itoa(tb)
		}
		r, err := doH2(utls.HelloChrome_Auto, "POST", "/api/collect", hdr, body, cookie)
		if err != nil {
			return nil, err
		}
		_ = json.Unmarshal(r.body, &v)
		if s := r.headers["X-Hm-Seed"]; s != "" {
			if ns, e := base64.RawURLEncoding.DecodeString(s); e == nil {
				seed = ns
			}
		}
	}
	return v, nil
}

// chromePQUA claims Chrome 131 — the first stable Chrome that ships the X25519MLKEM768
// hybrid post-quantum key share by DEFAULT (2026). A scraper that keeps a pre-PQ TLS
// fingerprint while advertising this UA is the exact PQ mismatch R9 catches.
const chromePQUA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"

// coherentReportBodyPQ is coherentReportBody with the UA bumped to the PQ-era Chrome so the
// report body, header UA, and sec-ch-ua all agree — isolating the TLS PQ omission as the tell.
func coherentReportBodyPQ() string {
	return strings.Replace(coherentReportBody(), chromeUA, chromePQUA, 1)
}

// pqAbsent pins a scraper that keeps a PRE-post-quantum uTLS fingerprint (HelloChrome_100,
// whose ClientHello supported_groups lack X25519MLKEM768 / 0x11EC) while its UA claims
// Chrome 131 — the first stable Chrome that sends the PQ key share by default (measured vs
// real headless Chromium 149, which does send it). Headers, sec-ch-ua, report body, and RIT
// are all coherent, so the ONLY residual tell is the TLS PQ omission -> l5.tls.pq_keyshare
// -> HR-24 CHALLENGE. Web-research grounding: Chrome shipped X25519MLKEM768 on by default in
// M131 (Nov 2024); Firefox 132; scrapers pinning older parrots (curl_cffi, older uTLS) lack it.
func pqAbsent() (map[string]any, error) {
	cookie, seed, n, err := session(utls.HelloChrome_100)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	body := coherentReportBodyPQ()
	v, _, err := signedCollectHdr(utls.HelloChrome_100, chromePQUA, cookie, sid, seed, n+1, body,
		map[string]string{"sec-ch-ua": `"Chromium";v="131", "Google Chrome";v="131", "Not.A/Brand";v="24"`})
	return v, err
}

// stripALPS removes the ALPS (application_settings) extension(s) from a ClientHello spec,
// keeping every other extension in place — so the only delta from a real Chrome hello is the
// missing ALPS. Both codepoint variants are removed (Chrome migrated 17513 -> 17613).
func stripALPS(exts []utls.TLSExtension) []utls.TLSExtension {
	out := exts[:0:0]
	for _, e := range exts {
		switch e.(type) {
		case *utls.ApplicationSettingsExtension, *utls.ApplicationSettingsExtensionNew:
			continue // drop ALPS
		default:
			out = append(out, e)
		}
	}
	return out
}

// alpsAbsent pins a non-Chromium TLS stack wearing a Chrome UA: it sends a real Chrome_Auto
// ClientHello with the ALPS (application_settings) extension STRIPPED, over h2 (ALPN offers h2),
// with a coherent RIT-signed Chrome report. Every genuine Chromium build sends ALPS on h2;
// Firefox/Safari/Go/curl send none — so the missing ALPS is the only residual tell, isolated
// from all other fingerprints. -> l5.tls.alps_absent -> HR-24 net.tls.alps CHALLENGE.
// Web-research grounding: ALPS (application_settings, codepoint 17513/17613) is a Chromium-only
// TLS extension advertising per-ALPN settings for h2; non-Chromium stacks impersonating Chrome
// omit it (a 2025-2026 TLS-fingerprint tell alongside JA3/JA4).
func alpsAbsent() (map[string]any, error) {
	cookie, seed, n, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	body := coherentReportBody() // coherent Chrome report; the ONLY residual is the missing ALPS
	spec, err := utls.UTLSIdToSpec(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	spec.Extensions = stripALPS(spec.Extensions)
	tb := nowTB()
	nn := n + 1
	hdr := withBrowserHeaders(map[string]string{
		"User-Agent": chromeUA,
		"X-HM-Token": ritToken(seed, sid, nn, tb, body),
		"X-HM-N":     itoa(nn),
		"X-HM-TB":    itoa(tb),
	})
	r, err := doH2Spec(spec, "POST", "/api/collect", hdr, body, cookie)
	if err != nil {
		return nil, err
	}
	v := map[string]any{}
	_ = json.Unmarshal(r.body, &v)
	return v, nil
}

// headerOrderSplit sends a coherent Chrome client (real Chrome uTLS, coherent report, RIT-
// signed) over HTTP/1.1 but with the request headers in a NON-BROWSER order: user-agent
// BEFORE the sec-ch-ua client-hints. Real Chrome always emits the client-hints cluster
// first (measured), so this is the 2026 header-order tell. With the raw-capture front end
// (R8) the server now observes the wire order and fires l5.header.order -> HR-24 CHALLENGE.
func headerOrderSplit() (map[string]any, error) {
	cookie, seed, n, err := session(utls.HelloChrome_Auto)
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(cookie)
	body := coherentReportBody() // coherent report so the ONLY residual tell is the header order
	tb := nowTB()
	nn := n + 1
	// Ordered headers: user-agent FIRST, then the sec-ch-ua cluster AFTER (inverted vs a
	// real browser). Content-Type + RIT headers included so the collect is well-formed.
	ordered := [][2]string{
		{"user-agent", chromeUA},
		{"accept", "*/*"},
		{"content-type", "application/json"},
		{"sec-fetch-site", "same-origin"},
		{"sec-fetch-mode", "cors"},
		{"sec-fetch-dest", "empty"},
		{"sec-ch-ua", `"Chromium";v="126", "Google Chrome";v="126", "Not.A/Brand";v="24"`},
		{"sec-ch-ua-mobile", "?0"},
		{"sec-ch-ua-platform", `"Windows"`},
		{"accept-encoding", "gzip, deflate, br, zstd"},
		{"accept-language", "en-US,en;q=0.9"},
		{"x-hm-token", ritToken(seed, sid, nn, tb, body)},
		{"x-hm-n", itoa(nn)},
		{"x-hm-tb", itoa(tb)},
	}
	v, err := doOrderedH1(utls.HelloChrome_Auto, "POST", "/api/collect", ordered, body, cookie)
	if err != nil {
		return nil, err
	}
	return v, nil
}

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

// flood hammers /api/collect with many rapid requests from one TLS fingerprint
// (application-layer DoS / credential-stuffing velocity). The Blue engine meters
// by JA4+subnet (not IP), so the flood is caught even though each request opens
// a fresh connection -> l5.abuse.flood -> HR-21 DENY (SoT-17).
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

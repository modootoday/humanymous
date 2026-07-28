// SPDX-License-Identifier: Apache-2.0

// Command redteam is an aggressive Red-team client that attacks the anti-bypass
// systems directly (SoT-07/12). It controls the TLS fingerprint (uTLS) and RIT
// tokens precisely to attempt evasions a browser cannot, so the Blue engine's
// network-bypass and anti-tamper defenses can be verified. Local target only.
//
// Attacks (-attack), all local-target-only:
//
//	tls-static   : a fixed uTLS parrot (no extension permutation)
//	tls-rotate   : two requests in one session with different TLS fingerprints
//	ua-rotate    : two requests in one session with different User-Agents
//	rit-replay   : replay a previously valid RIT token
//	rit-tamper   : sign one body, send a different body
//	flood        : application-layer request flood (fingerprint-keyed rate)
//	distributed  : one fingerprint across rotated proxy subnets
//	privacy-evasion : distributed proxy-rotation that ALSO claims ad-block/GPC to try to
//	                  disarm HR-19 (round-5 regression case) — must still DENY
//	signal-forgery  : a borderline client forging l7.pass.solved/l7.pow.solved to launder
//	                  a CHALLENGE to ALLOW (round-3 provenance blocker) — must not ALLOW
//	xff-spoof    : forged private X-Forwarded-For to impersonate a LAN client
//	squid-forward: open Squid forward-proxy hop headers (Via/X-Cache) → HR-24
//	openvpn-exit : OpenVPN exit + WebRTC public IP leak → HR-24
//	wireguard-hop: WireGuard mid-session exit hop + multi-hop XFF → HR-24
//	tor-exit     : Tor circuit multi-hop + exit rotation → HR-24 / HR-19
//	anon-proxy-chain : free open-proxy 4+ hop chain + Proxy-Connection → HR-24
//	elite-anon-proxy : elite Forwarded multi-hop (Via stripped) → HR-24
//	cdn-ip-spoof : forged CF-Connecting-IP / True-Client-IP → HR-24
//	proxy-ua-rotate : exit hop + mid-session UA rotate → HR-15 / HR-19
//	fp-churn-proxy : rotate fingerprintId per exit to dodge HR-19 → still HR-19
//	stacked-proxy-vpn : Squid hop over VPN + WebRTC leak → HR-24
//	socks-exit-hop : SOCKS5/VPN L4 exit hop + multi-hop XFF → HR-24
//
// (HTTP/2 Rapid Reset is exercised by the rapid_reset node profile, not this binary.)
package main

import (
	"bufio"
	"crypto/hmac"
	"crypto/sha256"
	"crypto/tls"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"strconv"
	"strings"
	"time"

	utls "github.com/refraction-networking/utls"
	"golang.org/x/net/http2"
)

var (
	attack = flag.String("attack", "", "tls-static|tls-rotate|ua-rotate|rit-replay|rit-tamper|flood|distributed|privacy-evasion|signal-forgery|pow-fast-solve|pow-launder|h2-protocol-split|h2-settings-split|nonbrowser-ua|sec-chua-absent|sec-fetch-absent|rit-absent|ja4-churn|multi-axis-rotate|grease-absent-js|xff-spoof|squid-forward|openvpn-exit|wireguard-hop|tor-exit|anon-proxy-chain|elite-anon-proxy|cdn-ip-spoof|proxy-ua-rotate|fp-churn-proxy|stacked-proxy-vpn|socks-exit-hop")
	host   = flag.String("host", "127.0.0.1:8443", "target host:port")
)

func main() {
	flag.Parse()
	var v map[string]any
	var err error
	switch *attack {
	case "flood":
		v, err = flood()
	case "distributed":
		v, err = distributed()
	case "privacy-evasion":
		v, err = privacyEvasion()
	case "signal-forgery":
		v, err = signalForgery()
	case "pow-fast-solve":
		v, err = powFastSolve()
	case "pow-launder":
		v, err = powLaunder()
	case "h2-protocol-split":
		v, err = h2ProtocolSplit()
	case "h2-settings-split":
		v, err = h2SettingsSplit()
	case "header-order-split":
		v, err = headerOrderSplit()
	case "pq-absent":
		v, err = pqAbsent()
	case "alps-absent":
		v, err = alpsAbsent()
	case "h2-flow-control":
		v, err = h2FlowControlSplit()
	case "cert-compression-absent":
		v, err = certCompressionAbsent()
	case "nonbrowser-ua":
		v, err = nonBrowserUA()
	case "sec-chua-absent":
		v, err = secCHUAAbsent()
	case "sec-fetch-absent":
		v, err = secFetchAbsent()
	case "rit-absent":
		v, err = ritAbsent()
	case "ja4-churn":
		v, err = ja4Churn()
	case "multi-axis-rotate":
		v, err = multiAxisRotate()
	case "grease-absent-js":
		v, err = greaseAbsentJS()
	case "coherent-ceiling":
		v, err = coherentBrowser()
	case "xff-spoof":
		v, err = xffSpoof()
	case "squid-forward":
		v, err = squidForward()
	case "openvpn-exit":
		v, err = openvpnExit()
	case "wireguard-hop":
		v, err = wireguardHop()
	case "tor-exit":
		v, err = torExit()
	case "anon-proxy-chain":
		v, err = anonProxyChain()
	case "elite-anon-proxy":
		v, err = eliteAnonProxy()
	case "cdn-ip-spoof":
		v, err = cdnIPSpoof()
	case "proxy-ua-rotate":
		v, err = proxyUARotate()
	case "fp-churn-proxy":
		v, err = fpChurnProxy()
	case "stacked-proxy-vpn":
		v, err = stackedProxyVPN()
	case "socks-exit-hop":
		v, err = socksExitHop()
	case "tls-static":
		v, err = tlsStatic()
	case "tls-rotate":
		v, err = tlsRotate()
	case "ua-rotate":
		v, err = uaRotate()
	case "rit-replay":
		v, err = ritReplay()
	case "rit-tamper":
		v, err = ritTamper()
	default:
		fmt.Fprintln(os.Stderr, "unknown attack:", *attack)
		os.Exit(2)
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, "redteam:", err)
		os.Exit(1)
	}
	b, _ := json.Marshal(v)
	fmt.Println(string(b))
}

// --- HTTP/1.1 over a uTLS-controlled ClientHello ---

type resp struct {
	status  int
	body    []byte
	cookie  string
	headers map[string]string
}

// do sends one HTTP/1.1 request over a fresh TLS connection with the given
// uTLS ClientHelloID, returning the response.
func do(helloID utls.ClientHelloID, method, path string, hdr map[string]string, body, cookie string) (*resp, error) {
	raw, err := net.Dial("tcp", *host)
	if err != nil {
		return nil, err
	}
	defer raw.Close()
	spec, err := utls.UTLSIdToSpec(helloID)
	if err != nil {
		return nil, err
	}
	forceHTTP11(&spec)
	uconn := utls.UClient(raw, &utls.Config{ServerName: hostname(*host), InsecureSkipVerify: true}, utls.HelloCustom)
	if err := uconn.ApplyPreset(&spec); err != nil {
		return nil, err
	}
	if err := uconn.Handshake(); err != nil {
		return nil, err
	}
	return httpRoundTrip(uconn, method, path, hdr, body, cookie)
}

// doH2 negotiates HTTP/2 over a uTLS handshake (ALPN left intact so h2 is offered),
// then speaks HTTP/2 framing with Go's golang.org/x/net/http2 client. The TLS
// ClientHello is a real browser's (JA4 engine = the browser), but the HTTP/2 SETTINGS /
// WINDOW_UPDATE / pseudo-header layout is Go's — the 2026 "protocol-split" evasion where a
// client passes TLS/JA4 but carries a library HTTP/2 fingerprint. Returns the response.
func doH2(helloID utls.ClientHelloID, method, path string, hdr map[string]string, body, cookie string) (*resp, error) {
	spec, err := utls.UTLSIdToSpec(helloID)
	if err != nil {
		return nil, err
	}
	return doH2Spec(spec, method, path, hdr, body, cookie)
}

// doH2Spec is doH2 with a caller-supplied (possibly mutated) ClientHello spec, so a Red
// profile can strip or alter a single extension (e.g. ALPS) while keeping the browser ALPN
// (h2, http/1.1) so the server negotiates h2. The ONLY delta is the caller's spec edit.
func doH2Spec(spec utls.ClientHelloSpec, method, path string, hdr map[string]string, body, cookie string) (*resp, error) {
	raw, err := net.Dial("tcp", *host)
	if err != nil {
		return nil, err
	}
	// NOTE: no forceHTTP11 — leave the browser ALPN (h2, http/1.1) so the server negotiates h2.
	uconn := utls.UClient(raw, &utls.Config{ServerName: hostname(*host), InsecureSkipVerify: true}, utls.HelloCustom)
	if err := uconn.ApplyPreset(&spec); err != nil {
		raw.Close()
		return nil, err
	}
	if err := uconn.Handshake(); err != nil {
		raw.Close()
		return nil, err
	}
	if p := uconn.ConnectionState().NegotiatedProtocol; p != "h2" {
		uconn.Close()
		return nil, fmt.Errorf("h2 not negotiated (ALPN=%q)", p)
	}
	tr := &http2.Transport{}
	cc, err := tr.NewClientConn(uconn)
	if err != nil {
		uconn.Close()
		return nil, err
	}
	defer cc.Close()
	req, err := http.NewRequest(method, "https://"+hostname(*host)+path, strings.NewReader(body))
	if err != nil {
		return nil, err
	}
	for k, v := range hdr {
		req.Header.Set(k, v)
	}
	if cookie != "" {
		req.Header.Set("Cookie", cookie)
	}
	r, err := cc.RoundTrip(req)
	if err != nil {
		return nil, err
	}
	defer r.Body.Close()
	out, _ := io.ReadAll(r.Body)
	res := &resp{status: r.StatusCode, body: out, headers: map[string]string{}}
	for k := range r.Header {
		res.headers[k] = r.Header.Get(k)
	}
	if sc := r.Header.Get("Set-Cookie"); sc != "" {
		res.cookie = strings.SplitN(sc, ";", 2)[0]
	}
	return res, nil
}

// doOrderedH1 sends one HTTP/1.1 request over a uTLS ClientHello with the given headers
// written in the EXACT slice order (not a Go map), so the Red team controls the on-wire
// header ORDER the server observes (SoT-02 / R8 header-order capture).
func doOrderedH1(helloID utls.ClientHelloID, method, path string, ordered [][2]string, body, cookie string) (map[string]any, error) {
	raw, err := net.Dial("tcp", *host)
	if err != nil {
		return nil, err
	}
	defer raw.Close()
	spec, err := utls.UTLSIdToSpec(helloID)
	if err != nil {
		return nil, err
	}
	forceHTTP11(&spec)
	uconn := utls.UClient(raw, &utls.Config{ServerName: hostname(*host), InsecureSkipVerify: true}, utls.HelloCustom)
	if err := uconn.ApplyPreset(&spec); err != nil {
		return nil, err
	}
	if err := uconn.Handshake(); err != nil {
		return nil, err
	}
	var b strings.Builder
	fmt.Fprintf(&b, "%s %s HTTP/1.1\r\n", method, path)
	fmt.Fprintf(&b, "Host: %s\r\n", hostname(*host))
	for _, kv := range ordered {
		fmt.Fprintf(&b, "%s: %s\r\n", kv[0], kv[1])
	}
	if cookie != "" {
		fmt.Fprintf(&b, "Cookie: %s\r\n", cookie)
	}
	if body != "" {
		fmt.Fprintf(&b, "Content-Length: %d\r\n", len(body))
	}
	b.WriteString("Connection: close\r\n\r\n")
	b.WriteString(body)
	if _, err := io.WriteString(uconn, b.String()); err != nil {
		return nil, err
	}
	r, err := http.ReadResponse(bufio.NewReader(uconn), nil)
	if err != nil {
		return nil, err
	}
	defer r.Body.Close()
	out, _ := io.ReadAll(r.Body)
	var v map[string]any
	_ = json.Unmarshal(out, &v)
	return v, nil
}

// doStock sends one HTTP/1.1 request over Go's STOCK crypto/tls (no uTLS, no GREASE), so the
// server observes a Go TLS fingerprint (JA4 engine = go). Used by the library-client and
// grease-absent scenarios where the whole point is a genuine non-browser TLS stack.
func doStock(method, path string, hdr map[string]string, body, cookie string) (*resp, error) {
	raw, err := net.Dial("tcp", *host)
	if err != nil {
		return nil, err
	}
	defer raw.Close()
	conn := tls.Client(raw, &tls.Config{ServerName: hostname(*host), InsecureSkipVerify: true, NextProtos: []string{"http/1.1"}})
	if err := conn.Handshake(); err != nil {
		return nil, err
	}
	return httpRoundTrip(conn, method, path, hdr, body, cookie)
}

// httpRoundTrip writes one HTTP/1.1 request over an already-handshaked conn and parses the
// response. Shared by the uTLS (do) and stock-TLS (doStock) paths.
func httpRoundTrip(conn io.ReadWriter, method, path string, hdr map[string]string, body, cookie string) (*resp, error) {
	var b strings.Builder
	fmt.Fprintf(&b, "%s %s HTTP/1.1\r\n", method, path)
	fmt.Fprintf(&b, "Host: %s\r\n", hostname(*host))
	for k, v := range hdr {
		fmt.Fprintf(&b, "%s: %s\r\n", k, v)
	}
	if cookie != "" {
		fmt.Fprintf(&b, "Cookie: %s\r\n", cookie)
	}
	if body != "" {
		fmt.Fprintf(&b, "Content-Length: %d\r\n", len(body))
	}
	b.WriteString("Connection: close\r\n\r\n")
	b.WriteString(body)
	if _, err := io.WriteString(conn, b.String()); err != nil {
		return nil, err
	}
	r, err := http.ReadResponse(bufio.NewReader(conn), nil)
	if err != nil {
		return nil, err
	}
	defer r.Body.Close()
	out, _ := io.ReadAll(r.Body)
	res := &resp{status: r.StatusCode, body: out, headers: map[string]string{}}
	for k := range r.Header {
		res.headers[k] = r.Header.Get(k)
	}
	if sc := r.Header.Get("Set-Cookie"); sc != "" {
		res.cookie = strings.SplitN(sc, ";", 2)[0]
	}
	return res, nil
}

// sessionStock / collectStock mirror session/collect but over Go's stock crypto/tls, so the
// session's pinned network observation carries a Go (no-GREASE) TLS fingerprint.
func sessionStock() (cookie string, err error) {
	r, err := doStock("GET", "/api/session", withBrowserHeaders(map[string]string{"User-Agent": chromeUA}), "", "")
	if err != nil {
		return "", err
	}
	return r.cookie, nil
}

func collectStock(ua, cookie string, hdr map[string]string, body string) (map[string]any, error) {
	h := map[string]string{"User-Agent": ua, "Content-Type": "application/json"}
	for k, v := range hdr {
		h[k] = v
	}
	r, err := doStock("POST", "/api/collect", h, body, cookie)
	if err != nil {
		return nil, err
	}
	var v map[string]any
	_ = json.Unmarshal(r.body, &v)
	return v, nil
}

// session establishes a session (GET /api/session) with a given hello, returning
// the cookie and the RIT seed/n.
func session(helloID utls.ClientHelloID) (cookie string, seed []byte, n uint64, err error) {
	r, err := do(helloID, "GET", "/api/session", withBrowserHeaders(map[string]string{"User-Agent": chromeUA}), "", "")
	if err != nil {
		return "", nil, 0, err
	}
	cookie = r.cookie
	var j struct {
		RitSeed string `json:"ritSeed"`
		RitN    uint64 `json:"ritN"`
	}
	_ = json.Unmarshal(r.body, &j)
	if j.RitSeed != "" {
		seed, _ = base64.RawURLEncoding.DecodeString(j.RitSeed)
	}
	return cookie, seed, j.RitN, nil
}

const chromeUA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
const collectBody = `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36","signals":[]}`

// browserHeaders spoofs the client-hint / sec-fetch headers a real Chrome sends,
// so the L6 header cross-checks stay quiet and the Blue engine must catch the
// attack via the specific anti-bypass signal (a curl_cffi-class adversary).
func browserHeaders() map[string]string {
	return map[string]string{
		"sec-ch-ua":          `"Chromium";v="126", "Google Chrome";v="126", "Not.A/Brand";v="24"`,
		"sec-ch-ua-mobile":   "?0",
		"sec-ch-ua-platform": `"Windows"`,
		"sec-fetch-site":     "same-origin",
		"sec-fetch-mode":     "cors",
		"sec-fetch-dest":     "empty",
		"accept":             "*/*",
		"accept-language":    "en-US,en;q=0.9",
		"accept-encoding":    "gzip, deflate, br, zstd",
	}
}

func withBrowserHeaders(extra map[string]string) map[string]string {
	h := browserHeaders()
	for k, v := range extra {
		h[k] = v
	}
	return h
}

// ritToken computes the live RIT token for a body (matches cmd/server/rit.go).
func ritToken(seed []byte, sid string, n, tb uint64, body string) string {
	bh := sha256.Sum256([]byte(body))
	canonical := sid + "\n" + strconv.FormatUint(n, 10) + "\n" +
		strconv.FormatUint(tb, 10) + "\n" + hex.EncodeToString(bh[:])
	m := hmac.New(sha256.New, seed)
	m.Write([]byte(canonical))
	return base64.RawURLEncoding.EncodeToString(m.Sum(nil))
}

func sidFromCookie(cookie string) string {
	if i := strings.IndexByte(cookie, '='); i >= 0 {
		return cookie[i+1:]
	}
	return ""
}

func collect(hello utls.ClientHelloID, ua, cookie string, hdr map[string]string, body string) (map[string]any, error) {
	v, _, err := collectSeed(hello, ua, cookie, hdr, body)
	return v, err
}

// collectSeed POSTs /api/collect and also returns the rotated seed the server
// issued in the response (X-HM-Seed), so the caller can sign its next request.
func collectSeed(hello utls.ClientHelloID, ua, cookie string, hdr map[string]string, body string) (map[string]any, []byte, error) {
	h := map[string]string{"User-Agent": ua, "Content-Type": "application/json"}
	for k, v := range hdr {
		h[k] = v
	}
	r, err := do(hello, "POST", "/api/collect", h, body, cookie)
	if err != nil {
		return nil, nil, err
	}
	var v map[string]any
	_ = json.Unmarshal(r.body, &v)
	var seed []byte
	if s := r.headers["X-Hm-Seed"]; s != "" {
		seed, _ = base64.RawURLEncoding.DecodeString(s)
	}
	return v, seed, nil
}

func forceHTTP11(spec *utls.ClientHelloSpec) {
	for _, ext := range spec.Extensions {
		if alpn, ok := ext.(*utls.ALPNExtension); ok {
			alpn.AlpnProtocols = []string{"http/1.1"}
		}
	}
}

func hostname(hp string) string {
	if h, _, err := net.SplitHostPort(hp); err == nil {
		return h
	}
	return hp
}

var _ = time.Now

package main

import (
	"bytes"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"net"

	utls "github.com/refraction-networking/utls"
	"golang.org/x/net/http2"
	"golang.org/x/net/http2/hpack"
)

// h2raw.go is a RAW HTTP/2 client (own framer, not Go's http2.Transport) so the Red team
// controls the exact SETTINGS and the HEADERS pseudo-header ORDER on the wire. Go's
// Transport always emits a,m,p,s; a raw framer can emit Chrome's m,a,s,p while shipping a
// non-Chrome SETTINGS profile — the 2026 "protocol-split at the h2 layer" evasion (R6).

// rawH2Conn is a minimal client-side HTTP/2 connection over an established (uTLS) conn.
type rawH2Conn struct {
	conn          net.Conn
	fr            *http2.Framer
	enc           *hpack.Encoder
	hbuf          bytes.Buffer
	lastSetCookie string
	lastSeed      string // X-Hm-Seed rotated seed from the response
}

// browserHeaders2 returns the spoofed Chrome header set with LOWERCASE names (HTTP/2
// requires lowercase), plus any extra headers.
func browserHeaders2(extra map[string]string) map[string]string {
	h := map[string]string{}
	for k, v := range browserHeaders() {
		h[lower(k)] = v
	}
	for k, v := range extra {
		h[lower(k)] = v
	}
	return h
}

// coherentReportBody is a fully self-consistent Chrome client report (mirrors the T4
// coherent ceiling), so the ONLY residual tell is the atypical h2 SETTINGS profile.
func coherentReportBody() string {
	return `{"userAgent":"` + chromeUA + `","engineVersion":"wasm-1.0.0",` +
		`"advanced":{"probed":true,"mediaDeviceCount":3,"hasAudioInput":true,"hasVideoInput":true,"voiceCount":200,` +
		`"widevineSupported":true,"webgpuPresent":true,"webgpuVendor":"nvidia","webglVendor":"NVIDIA Corporation / NVIDIA GeForce RTX 3080",` +
		`"audioSampleRate":48000,"connectionPresent":true,"connectionRtt":50,"batteryPresent":true,"batteryLevel":0.8,` +
		`"timezoneIana":"America/New_York","language":"en-US","colorGamut":"srgb","maxTouchPoints":0},` +
		`"environment":{"probed":true},` +
		`"behavior":{"durationS":8,"mouse":{"samples":45,"velocityStdDev":0.6,"straightLineFrac":0.15,"accelEntropy":2.1,"meanJerk":0.4,"meanCurvature":0.3,"coalescedRatio":3.0},` +
		`"key":{"keystrokes":14,"meanDwellMs":95,"dwellStdDevMs":28,"meanFlightMs":140,"flightStdDevMs":35},` +
		`"events":{"totalEvents":60,"untrustedFrac":0,"clickCount":1}},"signals":[]}`
}

// dialRawH2 completes a uTLS handshake (h2 ALPN) and the HTTP/2 SETTINGS exchange, sending
// the caller's SETTINGS verbatim (so their order/values are the observed fingerprint).
func dialRawH2(helloID utls.ClientHelloID, settings []http2.Setting) (*rawH2Conn, error) {
	return dialRawH2WU(helloID, settings, 0)
}

// dialRawH2WU is dialRawH2 that ALSO sends a connection-level WINDOW_UPDATE (stream 0) with the
// given increment right after SETTINGS, so the Red team controls the h2 flow-control window the
// server's peek observes (increment 0 = send none, real-Chrome behaviour for that framer). A
// gigabyte increment mimics Go's http2 default — the flow-control tell (R11).
func dialRawH2WU(helloID utls.ClientHelloID, settings []http2.Setting, windowUpdate uint32) (*rawH2Conn, error) {
	raw, err := net.Dial("tcp", *host)
	if err != nil {
		return nil, err
	}
	spec, err := utls.UTLSIdToSpec(helloID)
	if err != nil {
		raw.Close()
		return nil, err
	}
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
	if _, err := io.WriteString(uconn, http2.ClientPreface); err != nil {
		uconn.Close()
		return nil, err
	}
	c := &rawH2Conn{conn: uconn, fr: http2.NewFramer(uconn, uconn)}
	c.enc = hpack.NewEncoder(&c.hbuf)
	// Send the client preface SETTINGS and return immediately WITHOUT reading the server's
	// SETTINGS — a real browser sends SETTINGS + the first request HEADERS back-to-back and
	// does not wait for the server. The server's edge fingerprinter (peekH2) captures the
	// SETTINGS + first HEADERS before the h2 server ever sends its own SETTINGS, so blocking
	// here would deadlock the fingerprint capture. Server SETTINGS/ACK are handled in the read
	// loop of request().
	if err := c.fr.WriteSettings(settings...); err != nil {
		uconn.Close()
		return nil, err
	}
	// Connection-level WINDOW_UPDATE (stream 0) between SETTINGS and the first HEADERS, matching
	// where real browsers place it, so the server's peekH2 captures the increment.
	if windowUpdate > 0 {
		if err := c.fr.WriteWindowUpdate(0, windowUpdate); err != nil {
			uconn.Close()
			return nil, err
		}
	}
	return c, nil
}

// request sends one request on a fresh stream with pseudo-headers in m,a,s,p (Chrome) order
// and returns the decoded response body.
func (c *rawH2Conn) request(streamID uint32, method, path string, headers map[string]string, body, cookie string) ([]byte, error) {
	c.hbuf.Reset()
	// Chrome pseudo-header order: :method, :authority, :scheme, :path (m,a,s,p).
	c.enc.WriteField(hpack.HeaderField{Name: ":method", Value: method})
	c.enc.WriteField(hpack.HeaderField{Name: ":authority", Value: hostname(*host)})
	c.enc.WriteField(hpack.HeaderField{Name: ":scheme", Value: "https"})
	c.enc.WriteField(hpack.HeaderField{Name: ":path", Value: path})
	for k, v := range headers {
		c.enc.WriteField(hpack.HeaderField{Name: lower(k), Value: v})
	}
	if cookie != "" {
		c.enc.WriteField(hpack.HeaderField{Name: "cookie", Value: cookie})
	}
	if err := c.fr.WriteHeaders(http2.HeadersFrameParam{
		StreamID: streamID, BlockFragment: c.hbuf.Bytes(), EndHeaders: true, EndStream: body == "",
	}); err != nil {
		return nil, err
	}
	if body != "" {
		if err := c.fr.WriteData(streamID, true, []byte(body)); err != nil {
			return nil, err
		}
	}
	var out bytes.Buffer
	setCookie := ""
	for {
		f, err := c.fr.ReadFrame()
		if err != nil {
			if out.Len() > 0 {
				break
			}
			return nil, err
		}
		switch fr := f.(type) {
		case *http2.SettingsFrame:
			if !fr.IsAck() {
				_ = c.fr.WriteSettingsAck() // ACK the server's SETTINGS
			}
		case *http2.PingFrame:
			if !fr.IsAck() {
				_ = c.fr.WritePing(true, fr.Data)
			}
		case *http2.DataFrame:
			if fr.StreamID == streamID {
				out.Write(fr.Data())
				if fr.StreamEnded() {
					c.lastSetCookie = setCookie
					return out.Bytes(), nil
				}
			}
		case *http2.HeadersFrame:
			if fr.StreamID == streamID {
				if fields, e := hpack.NewDecoder(4096, nil).DecodeFull(fr.HeaderBlockFragment()); e == nil {
					for _, hf := range fields {
						if hf.Name == "set-cookie" && setCookie == "" {
							setCookie = splitCookie(hf.Value)
						}
						if hf.Name == "x-hm-seed" {
							c.lastSeed = hf.Value
						}
					}
				}
				if fr.StreamEnded() {
					c.lastSetCookie = setCookie
					return out.Bytes(), nil
				}
			}
		case *http2.GoAwayFrame:
			c.lastSetCookie = setCookie
			if out.Len() > 0 {
				return out.Bytes(), nil
			}
			return nil, fmt.Errorf("h2 GOAWAY: %s", fr.ErrCode)
		}
	}
	c.lastSetCookie = setCookie
	return out.Bytes(), nil
}

func (c *rawH2Conn) close() { c.conn.Close() }

func splitCookie(v string) string {
	if i := indexByte(v, ';'); i >= 0 {
		return v[:i]
	}
	return v
}

func indexByte(s string, b byte) int {
	for i := 0; i < len(s); i++ {
		if s[i] == b {
			return i
		}
	}
	return -1
}

func lower(s string) string {
	out := []byte(s)
	for i, c := range out {
		if c >= 'A' && c <= 'Z' {
			out[i] = c + 32
		}
	}
	return string(out)
}

// h2SettingsSplit is the R6 live exploit: a raw HTTP/2 client that ships Chrome's m,a,s,p
// pseudo-header order (so EngineFromH2 classifies it as Chrome) but a NON-Chrome SETTINGS
// profile (no HEADER_TABLE_SIZE, a 1 GiB window update). With a coherent Chrome UA + report +
// RIT signing, the ONLY residual tell is the atypical SETTINGS under a browser pseudo-order.
// The reported verdict is the collect re-score; the freeze-safe residual
// l5.http2.browser_settings_atypical fires without moving it.
func h2SettingsSplit() (map[string]any, error) {
	// Non-Chrome SETTINGS: omit HEADER_TABLE_SIZE (id 1); large values a browser never sends.
	settings := []http2.Setting{
		{ID: http2.SettingEnablePush, Val: 0},
		{ID: http2.SettingInitialWindowSize, Val: 4194304},
		{ID: http2.SettingMaxFrameSize, Val: 16384},
		{ID: http2.SettingMaxHeaderListSize, Val: 10485760},
	}
	// Session over the same raw-h2 profile to capture the RIT seed.
	c1, err := dialRawH2(utls.HelloChrome_Auto, settings)
	return h2RawEvasion(c1, err, settings, 0)
}

// h2FlowControlSplit is the R11 live exploit: a raw HTTP/2 client that ships Chrome's m,a,s,p
// pseudo-order AND a COHERENT Chrome SETTINGS profile (includes HEADER_TABLE_SIZE, so the R6
// residual stays quiet) but opens a 1 GiB connection flow-control window (Go's http2 default) —
// no browser does this. With a coherent Chrome UA + report + RIT signing, the ONLY residual tell
// is the gigabyte WINDOW_UPDATE under a browser pseudo-order + coherent SETTINGS. Isolates the
// flow-control (W) dimension R6 (SETTINGS) and R7 (pseudo-order) do not cover.
func h2FlowControlSplit() (map[string]any, error) {
	const oneGiB = 1073741824
	// Coherent Chrome SETTINGS (with HEADER_TABLE_SIZE id 1) so browser_settings_atypical is quiet.
	settings := []http2.Setting{
		{ID: http2.SettingHeaderTableSize, Val: 65536},
		{ID: http2.SettingEnablePush, Val: 0},
		{ID: http2.SettingInitialWindowSize, Val: 6291456},
		{ID: http2.SettingMaxHeaderListSize, Val: 262144},
	}
	c1, err := dialRawH2WU(utls.HelloChrome_Auto, settings, oneGiB)
	return h2RawEvasion(c1, err, settings, oneGiB)
}

// h2PriorityAbsent is the R14 live exploit: a raw HTTP/2 client that ships Chrome's m,a,s,p
// pseudo-order, a COHERENT Chrome SETTINGS profile (passes R6), AND a real Chrome connection
// window (15663105, passes R11) — but its HEADERS frame carries NO priority field and it sends
// no separate PRIORITY frame. Every real browser signals priority (measured: Chrome HEADERS
// excl=1/weight=255, Firefox excl=0/weight=41); the raw framer omits it. With a coherent Chrome
// UA + report + RIT, the ONLY residual tell is the missing priority — the 4th Akamai h2
// component that R6 (SETTINGS), R7 (pseudo-order) and R11 (WINDOW_UPDATE) do not cover.
func h2PriorityAbsent() (map[string]any, error) {
	const chromeWindow = 15663105 // real Chrome connection WINDOW_UPDATE (passes R11)
	settings := []http2.Setting{
		{ID: http2.SettingHeaderTableSize, Val: 65536},
		{ID: http2.SettingEnablePush, Val: 0},
		{ID: http2.SettingInitialWindowSize, Val: 6291456},
		{ID: http2.SettingMaxHeaderListSize, Val: 262144},
	}
	c1, err := dialRawH2WU(utls.HelloChrome_Auto, settings, chromeWindow)
	return h2RawEvasion(c1, err, settings, chromeWindow)
}

// h2RawEvasion runs the shared raw-h2 session→2×collect flow for the R6/R11 evasions: it takes
// the already-dialed session conn, replays the same SETTINGS (+ windowUpdate) on each collect,
// signs RIT, and returns the re-scored verdict. windowUpdate 0 = send none (R6).
func h2RawEvasion(c1 *rawH2Conn, dialErr error, settings []http2.Setting, windowUpdate uint32) (map[string]any, error) {
	err := dialErr
	if err != nil {
		return nil, err
	}
	sessBody, err := c1.request(1, "GET", "/api/session",
		browserHeaders2(map[string]string{"user-agent": chromeUA}), "", "")
	c1Cookie := c1.lastSetCookie
	c1.close()
	if err != nil {
		return nil, err
	}
	sid := sidFromCookie(c1Cookie)
	var sj struct {
		RitSeed string `json:"ritSeed"`
		RitN    uint64 `json:"ritN"`
	}
	_ = json.Unmarshal(sessBody, &sj)
	seed, _ := base64.RawURLEncoding.DecodeString(sj.RitSeed)
	n := sj.RitN

	body := coherentReportBody()
	v := map[string]any{}
	for i := 0; i < 2; i++ {
		c, err := dialRawH2WU(utls.HelloChrome_Auto, settings, windowUpdate)
		if err != nil {
			return nil, err
		}
		hdr := browserHeaders2(map[string]string{"user-agent": chromeUA, "content-type": "application/json"})
		if seed != nil {
			tb := nowTB()
			nn := n + uint64(i+1)
			hdr["x-hm-token"] = ritToken(seed, sid, nn, tb, body)
			hdr["x-hm-n"] = itoa(nn)
			hdr["x-hm-tb"] = itoa(tb)
		}
		respBody, err := c.request(1, "POST", "/api/collect", hdr, body, c1Cookie)
		rotated := c.lastSeed
		c.close()
		if err != nil {
			return nil, err
		}
		_ = json.Unmarshal(respBody, &v)
		if rotated != "" {
			if ns, e := base64.RawURLEncoding.DecodeString(rotated); e == nil {
				seed = ns // next collect signs n+(i+1) under the rotated seed
			}
		}
	}
	return v, nil
}

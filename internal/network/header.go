package network

import (
	"sort"
	"strings"
)

// header.go computes HTTP header-order signals and the JA4H fingerprint, and
// flags library/automation anomalies (SoT-02 §L5 header). It works on a small
// HeaderInfo struct rather than *http.Request so it is unit-testable and
// decoupled from the server (SRP).

// HeaderInfo is the decoupled, order-preserving view of a request's headers.
type HeaderInfo struct {
	Method  string   // GET/POST/...
	Version string   // "10"|"11"|"20"|"30"
	IsH2    bool     // true for HTTP/2+
	Names   []string // header names in wire order (original case)
	// CasingReliable is true only when Names preserve on-the-wire casing (raw
	// capture). Go's net/http canonicalizes names, so the server adapter sets
	// this false and the h2-uppercase check is suppressed to avoid false
	// positives (plan/02 §3.3).
	CasingReliable bool
	HasCookie      bool
	HasReferer     bool
	AcceptLanguage string
	AcceptEncoding string
	CookieNames    []string
	UserAgent      string
}

// SecFetchPresent reports whether any sec-fetch-* header is present.
func (h HeaderInfo) SecFetchPresent() bool {
	for _, n := range h.Names {
		if strings.HasPrefix(strings.ToLower(n), "sec-fetch-") {
			return true
		}
	}
	return false
}

// SecCHUAPresent reports whether sec-ch-ua is present.
func (h HeaderInfo) SecCHUAPresent() bool {
	for _, n := range h.Names {
		if strings.EqualFold(n, "sec-ch-ua") {
			return true
		}
	}
	return false
}

// HasUppercaseInH2 reports a malformed HTTP/2 request carrying an uppercase
// header name (HTTP/2 mandates lowercase) — a strong non-browser tell.
func (h HeaderInfo) HasUppercaseInH2() bool {
	if !h.IsH2 || !h.CasingReliable {
		return false
	}
	for _, n := range h.Names {
		if n != strings.ToLower(n) {
			return true
		}
	}
	return false
}

// ChromeAcceptEncodingOK reports whether accept-encoding includes zstd, which
// modern Chrome sends ("gzip, deflate, br, zstd").
func (h HeaderInfo) ChromeAcceptEncodingOK() bool {
	return strings.Contains(strings.ToLower(h.AcceptEncoding), "zstd")
}

// Order returns the lowercased header-name order (for order comparison).
func (h HeaderInfo) Order() []string {
	out := make([]string, len(h.Names))
	for i, n := range h.Names {
		out[i] = strings.ToLower(n)
	}
	return out
}

// JA4H computes the JA4H HTTP fingerprint (FoxIO):
//
//	{method:2}{version:2}{cookie:1}{referer:1}{hdrCount:02}_{alHash:12}_{hdrHash:12}_{ckHash:12}
func (h HeaderInfo) JA4H() string {
	method := twoChar(strings.ToLower(h.Method))
	ver := h.Version
	if ver == "" {
		ver = "11"
	}
	cookie := "n"
	if h.HasCookie {
		cookie = "c"
	}
	ref := "n"
	if h.HasReferer {
		ref = "r"
	}
	// header names excluding cookie/referer, in order.
	var hdrNames []string
	for _, n := range h.Names {
		ln := strings.ToLower(n)
		if ln == "cookie" || ln == "referer" {
			continue
		}
		hdrNames = append(hdrNames, ln)
	}
	al := "000000000000"
	if h.AcceptLanguage != "" {
		al = trunc12(sha256Hex(h.AcceptLanguage))
	}
	hdrHash := "000000000000"
	if len(hdrNames) > 0 {
		hdrHash = trunc12(sha256Hex(strings.Join(hdrNames, ",")))
	}
	ckHash := "000000000000"
	if len(h.CookieNames) > 0 {
		cn := append([]string(nil), h.CookieNames...)
		sort.Strings(cn)
		ckHash = trunc12(sha256Hex(strings.Join(cn, ",")))
	}
	a := method + ver + cookie + ref + pad2(len(hdrNames))
	return a + "_" + al + "_" + hdrHash + "_" + ckHash
}

func twoChar(s string) string {
	if len(s) >= 2 {
		return s[:2]
	}
	return (s + "xx")[:2]
}

func pad2(n int) string {
	if n > 99 {
		n = 99
	}
	if n < 10 {
		return "0" + string(rune('0'+n))
	}
	return string(rune('0'+n/10)) + string(rune('0'+n%10))
}

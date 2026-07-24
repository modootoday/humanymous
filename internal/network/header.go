package network

import (
	"strings"
)

// header.go computes HTTP header-order signals and flags library/automation
// anomalies (SoT-02 §L5 header). It works on a small
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

// NOTE: a JA4H HTTP-fingerprint implementation was removed here (PLAN-08
// deployment-review). JA4H is covered by the FoxIO License 1.1, unlike the JA4 TLS
// fingerprint in internal/network/ja4.go which is this project's own code under the
// project's Apache-2.0 license. It was dead code (no production caller, never emitted
// as a scoring signal), so it is deleted rather than carried as a licence liability.
// If ever revived, add explicit FoxIO License 1.1 attribution to
// NOTICE/THIRD_PARTY_LICENSES.md and confirm Apache-2.0 compatibility first.

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
	// Forward-proxy / hop residual (Squid, commercial forward proxies). A direct
	// browser does not send these; they leak when traffic passes an HTTP proxy.
	Via             string // RFC 7230 Via
	ProxyConnection string // obsolete hop-by-hop Proxy-Connection
	XCache          string // Squid/CDN X-Cache / X-Cache-Lookup
	XSquidError     string // Squid X-Squid-Error
	XForwardedFor   string // full X-Forwarded-For (multi-hop chain)
	Forwarded       string // RFC 7239 Forwarded
	// Client-identity laundering headers a browser never sends to origin.
	// Scrapers forge these to impersonate a CDN/edge-resolved client IP.
	CFConnectingIP        string // CF-Connecting-IP
	TrueClientIP          string // True-Client-IP (Akamai)
	XClientIP             string // X-Client-IP
	XOriginalForwardedFor string // X-Original-Forwarded-For
	XBlueCoatVia          string // commercial proxy residual
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

// ProxyHopKind returns a short label for a forward-proxy hop residual, or "".
// Direct browsers never emit Via / Proxy-Connection / Squid cache headers on
// the origin request; open proxies (Squid etc.) often do.
func (h HeaderInfo) ProxyHopKind() string {
	via := strings.ToLower(h.Via)
	if via != "" {
		if strings.Contains(via, "squid") {
			return "via-squid"
		}
		return "via"
	}
	if h.XSquidError != "" {
		return "x-squid-error"
	}
	if h.XBlueCoatVia != "" {
		return "bluecoat-via"
	}
	if h.XCache != "" {
		// X-Cache alone can be set by reverse CDNs on responses; on *requests*
		// it is a forward-proxy residual (Squid often injects X-Cache-Lookup).
		return "x-cache-request"
	}
	if h.ProxyConnection != "" {
		return "proxy-connection"
	}
	// RFC 7239 multi-hop: multiple for= tokens, or by= present on a client request.
	// Elite anonymous proxies often strip Via but leave Forwarded.
	fwd := strings.ToLower(h.Forwarded)
	if fwd != "" && (strings.Count(fwd, "for=") >= 2 || strings.Contains(fwd, "by=")) {
		return "forwarded-multi"
	}
	return ""
}

// ClientIPSpoofKind returns a label when the request carries forged CDN/edge
// client-identity headers (CF-Connecting-IP, True-Client-IP, …). A real browser
// talking to origin never sends these; scrapers use them to launder identity
// through anonymous proxies or to frame a victim IP.
func (h HeaderInfo) ClientIPSpoofKind() string {
	switch {
	case strings.TrimSpace(h.CFConnectingIP) != "":
		return "cf-connecting-ip"
	case strings.TrimSpace(h.TrueClientIP) != "":
		return "true-client-ip"
	case strings.TrimSpace(h.XClientIP) != "":
		return "x-client-ip"
	case strings.TrimSpace(h.XOriginalForwardedFor) != "":
		return "x-original-forwarded-for"
	default:
		return ""
	}
}

// XFFHopCount counts comma-separated X-Forwarded-For hops (0 if empty).
func (h HeaderInfo) XFFHopCount() int {
	s := strings.TrimSpace(h.XForwardedFor)
	if s == "" {
		return 0
	}
	n := 1
	for i := 0; i < len(s); i++ {
		if s[i] == ',' {
			n++
		}
	}
	return n
}

// NOTE: a JA4H HTTP-fingerprint implementation was removed here (PLAN-08
// deployment-review). JA4H is covered by the FoxIO License 1.1, unlike the JA4 TLS
// fingerprint in internal/network/ja4.go which is this project's own code under the
// project's Apache-2.0 license. It was dead code (no production caller, never emitted
// as a scoring signal), so it is deleted rather than carried as a licence liability.
// If ever revived, add explicit FoxIO License 1.1 attribution to
// NOTICE/THIRD_PARTY_LICENSES.md and confirm Apache-2.0 compatibility first.

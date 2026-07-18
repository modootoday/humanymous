package main

import (
	"net"
	"net/http"
	"sort"
	"strings"

	"github.com/modootoday/humanymous/internal/network"
)

// headerinfo.go adapts an *http.Request into network.HeaderInfo. Note: Go's
// net/http canonicalizes header names and does not preserve wire order for
// HTTP/1; full order fidelity requires raw capture (documented in plan/02).
// For the demo we extract presence/values, which drives the header signals.
func reqToHeaderInfo(r *http.Request) network.HeaderInfo {
	// Go's net/http stores headers in a map, so iteration order is random and
	// the on-wire order is lost. Sort for a STABLE header-set hash (used by the
	// traffic guard); real wire-order detection would need raw capture.
	names := make([]string, 0, len(r.Header))
	for k := range r.Header {
		names = append(names, k)
	}
	sort.Strings(names)
	version := "11"
	switch {
	case r.ProtoMajor == 2:
		version = "20"
	case r.ProtoMajor == 1 && r.ProtoMinor == 0:
		version = "10"
	}
	var cookieNames []string
	for _, c := range r.Cookies() {
		cookieNames = append(cookieNames, c.Name)
	}
	return network.HeaderInfo{
		Method:         r.Method,
		Version:        version,
		IsH2:           r.ProtoMajor == 2,
		Names:          names,
		HasCookie:      r.Header.Get("Cookie") != "",
		HasReferer:     r.Header.Get("Referer") != "",
		AcceptLanguage: r.Header.Get("Accept-Language"),
		AcceptEncoding: r.Header.Get("Accept-Encoding"),
		CookieNames:    cookieNames,
		UserAgent:      r.Header.Get("User-Agent"),
	}
}

// clientSubnet returns the /24 (or IPv6 /48) of the client for cross-session
// correlation. It prefers X-Forwarded-For (so the demo can simulate rotating
// residential-proxy exit IPs); production would trust only the real edge IP.
func clientSubnet(r *http.Request) string {
	ip := clientIP(r)
	if xff := r.Header.Get("X-Forwarded-For"); xff != "" {
		if i := strings.IndexByte(xff, ','); i >= 0 {
			ip = strings.TrimSpace(xff[:i])
		} else {
			ip = strings.TrimSpace(xff)
		}
	}
	if strings.Count(ip, ".") == 3 {
		parts := strings.Split(ip, ".")
		return parts[0] + "." + parts[1] + "." + parts[2]
	}
	return ip
}

// clientIP extracts the remote IP (without port) for IP-intel lookups.
// Uses net.SplitHostPort so IPv6 (e.g. "[::1]:1234") is handled correctly.
// clientIP returns the best-effort REAL (public) client IP for scoring. The
// immediate peer (r.RemoteAddr) is only the last hop — behind a reverse proxy /
// load balancer / CDN (e.g. the humanymous Gate) it is the proxy, not the user,
// and would score every visitor with the proxy's own (often datacenter) IP. When
// the peer is a TRUSTED proxy we resolve the originating client from the standard
// forwarding headers (left-most X-Forwarded-For entry = the first proxy's view of
// the client, else X-Real-IP). From an UNtrusted peer these headers are ignored,
// so a client cannot forge its source; the Gate additionally strips inbound
// forwarding headers from clients (HR-27b) before it sets its own.
func clientIP(r *http.Request) string {
	peer := directPeer(r.RemoteAddr)
	if !isTrustedProxy(peer) {
		return peer
	}
	if xff := r.Header.Get("X-Forwarded-For"); xff != "" {
		for _, part := range strings.Split(xff, ",") {
			if ip := strings.TrimSpace(part); ip != "" {
				return ip // left-most = original client
			}
		}
	}
	if xr := strings.TrimSpace(r.Header.Get("X-Real-IP")); xr != "" {
		return xr
	}
	return peer
}

// forwardedIP returns the IP the CLIENT asserted via a forwarding header
// (X-Forwarded-For left-most, else X-Real-IP), with NO trust decision applied. It
// feeds the forwarded_private spoof signal — a client claiming a private/reserved
// source is forging "I'm on your LAN" to shed IP-intel signals.
func forwardedIP(r *http.Request) string {
	if xff := r.Header.Get("X-Forwarded-For"); xff != "" {
		if i := strings.IndexByte(xff, ','); i >= 0 {
			return strings.TrimSpace(xff[:i])
		}
		return strings.TrimSpace(xff)
	}
	return strings.TrimSpace(r.Header.Get("X-Real-IP"))
}

// directPeer extracts the bare IP (no port, no brackets) of the connecting peer.
func directPeer(remoteAddr string) string {
	if host, _, err := net.SplitHostPort(remoteAddr); err == nil {
		return host
	}
	return strings.Trim(remoteAddr, "[]")
}

// isTrustedProxy reports whether a connecting peer is allowed to set forwarding
// headers on our behalf. Default trust = loopback + private + link-local ranges,
// where reverse proxies / load balancers live (the Gate sits on an internal
// network). A deployment fronted by a public-IP CDN should pin the CDN egress
// ranges here so only it can rewrite the client IP.
func isTrustedProxy(ip string) bool {
	p := net.ParseIP(ip)
	if p == nil {
		return false
	}
	return p.IsLoopback() || p.IsPrivate() || p.IsLinkLocalUnicast()
}

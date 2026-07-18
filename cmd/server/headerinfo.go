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
func clientIP(r *http.Request) string {
	if host, _, err := net.SplitHostPort(r.RemoteAddr); err == nil {
		return host
	}
	return strings.Trim(r.RemoteAddr, "[]")
}

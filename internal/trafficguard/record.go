// Package trafficguard logs all TCP/TLS/HTTP communication per session and
// inspects intra-session consistency to catch network-bypass evasion — an
// attacker rotating TLS/UA/IP/headers across requests to defeat single-request
// fingerprinting (SoT-12). Complements the per-request L5/L6 checks with the
// temporal dimension.
package trafficguard

import (
	"crypto/sha256"
	"encoding/hex"
	"strings"
	"time"
)

// TrafficRecord is one logged communication event (SoT-12 §1).
type TrafficRecord struct {
	TS          time.Time `json:"ts"`
	SessionID   string    `json:"sessionId"`
	RemoteAddr  string    `json:"remoteAddr"`
	JA3         string    `json:"ja3,omitempty"`
	JA4         string    `json:"ja4,omitempty"`
	JA4Engine   string    `json:"ja4Engine,omitempty"`
	ExtOrder    string    `json:"extOrder,omitempty"` // TLS extension-order hash (permutation)
	ALPN        string    `json:"alpn,omitempty"`
	TLSVersion  string    `json:"tlsVersion,omitempty"`
	Proto       string    `json:"proto,omitempty"` // "1.1" | "2"
	Method      string    `json:"method,omitempty"`
	Path        string    `json:"path,omitempty"`
	UAHash      string    `json:"uaHash,omitempty"`
	HeaderOrder string    `json:"headerOrderHash,omitempty"`
	HasSecFetch bool      `json:"hasSecFetch"`
}

// IsTLS reports a TLS-only record (JA4 seen, no HTTP method yet) — the two kinds are
// distinguished for backfill correlation.
func (r TrafficRecord) IsTLS() bool { return r.JA4 != "" && r.Method == "" }

// IsHTTP reports a record that carries an HTTP request line.
func (r TrafficRecord) IsHTTP() bool { return r.Method != "" }

// HashUA returns a short stable hash of a User-Agent (privacy: store hash).
func HashUA(ua string) string {
	if ua == "" {
		return ""
	}
	sum := sha256.Sum256([]byte(ua))
	return hex.EncodeToString(sum[:6])
}

// HashOrder hashes an ordered header-name list.
func HashOrder(names []string) string {
	if len(names) == 0 {
		return ""
	}
	sum := sha256.Sum256([]byte(strings.Join(names, ",")))
	return hex.EncodeToString(sum[:6])
}

// Subnet24 returns the /24 (IPv4) or /48 prefix (IPv6) of an ip:port addr, so a
// stable client that changes ephemeral ports is not treated as an IP hop.
func Subnet24(remoteAddr string) string {
	host := remoteAddr
	if i := strings.LastIndex(host, ":"); i > 0 && strings.Count(host, ":") == 1 {
		host = host[:i] // ipv4:port
	}
	host = strings.Trim(host, "[]")
	if strings.Contains(host, ".") { // ipv4
		parts := strings.Split(host, ".")
		if len(parts) == 4 {
			return parts[0] + "." + parts[1] + "." + parts[2]
		}
	}
	if strings.Contains(host, ":") { // ipv6 — first 3 groups
		g := strings.Split(host, ":")
		if len(g) >= 3 {
			return g[0] + ":" + g[1] + ":" + g[2]
		}
	}
	return host
}

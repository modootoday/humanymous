package main

import (
	"net"
	"net/http"
	"testing"
)

// With NO datacenter dataset wired, isDatacenterIP FAILS OPEN: every IP → false, so a real
// user's public IP is never accused (deep-review: the old stub flagged every public IP and
// HR-11 mass-CHALLENGEd 100% of real humans).
func TestIsDatacenterIP_FailsOpenByDefault(t *testing.T) {
	datacenterNets = nil // default state
	for _, ip := range []string{"127.0.0.1", "::1", "10.1.2.3", "172.30.0.1", "169.254.1.1", "garbage", "", "8.8.8.8", "1.2.3.4", "203.0.113.7"} {
		if isDatacenterIP(ip) {
			t.Errorf("isDatacenterIP(%q)=true with no dataset, want false (fail-open)", ip)
		}
	}
}

// With a real CIDR set wired, only IPs INSIDE it are datacenter; a residential/public IP
// outside the set is not accused.
func TestIsDatacenterIP_WithDataset(t *testing.T) {
	_, n, _ := net.ParseCIDR("203.0.113.0/24")
	SetDatacenterCIDRs([]*net.IPNet{n})
	defer SetDatacenterCIDRs(nil)
	if !isDatacenterIP("203.0.113.7") {
		t.Error("an IP inside a wired datacenter CIDR must be flagged")
	}
	for _, ip := range []string{"8.8.8.8", "198.51.100.4", "10.0.0.1", ""} {
		if isDatacenterIP(ip) {
			t.Errorf("isDatacenterIP(%q)=true, want false (outside the wired set / private)", ip)
		}
	}
}

func TestIsTrustedProxy(t *testing.T) {
	trusted := []string{"127.0.0.1", "::1", "10.0.0.1", "192.168.1.1", "172.30.0.1", "169.254.0.1", "fc00::5"}
	untrusted := []string{"8.8.8.8", "1.2.3.4", "172.32.0.1", "garbage", ""}
	for _, ip := range trusted {
		if !isTrustedProxy(ip) {
			t.Errorf("isTrustedProxy(%q)=false, want true", ip)
		}
	}
	for _, ip := range untrusted {
		if isTrustedProxy(ip) {
			t.Errorf("isTrustedProxy(%q)=true, want false", ip)
		}
	}
}

func TestClientIP(t *testing.T) {
	mk := func(remote string, hdr map[string]string) *http.Request {
		r := &http.Request{RemoteAddr: remote, Header: http.Header{}}
		for k, v := range hdr {
			r.Header.Set(k, v)
		}
		return r
	}

	tests := []struct {
		name   string
		remote string
		hdr    map[string]string
		want   string
	}{
		{"direct-untrusted-strips-port", "8.8.8.8:44210", nil, "8.8.8.8"},
		{"untrusted-peer-ignores-xff", "8.8.8.8:1000", map[string]string{"X-Forwarded-For": "1.2.3.4"}, "8.8.8.8"},
		{"trusted-proxy-uses-xff-leftmost", "172.30.0.1:8443", map[string]string{"X-Forwarded-For": "203.0.113.9, 172.30.0.1"}, "203.0.113.9"},
		{"trusted-proxy-uses-xrealip", "172.18.0.1:8443", map[string]string{"X-Real-IP": "203.0.113.7"}, "203.0.113.7"},
		{"trusted-proxy-no-headers-falls-back", "172.30.0.1:8443", nil, "172.30.0.1"},
		{"xff-preferred-over-xrealip", "10.0.0.1:80", map[string]string{"X-Forwarded-For": "203.0.113.1", "X-Real-IP": "203.0.113.2"}, "203.0.113.1"},
		{"v6-loopback-peer-trusted", "[::1]:5000", map[string]string{"X-Real-IP": "203.0.113.5"}, "203.0.113.5"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			if got := clientIP(mk(tc.remote, tc.hdr)); got != tc.want {
				t.Errorf("clientIP()=%q, want %q", got, tc.want)
			}
		})
	}
}

package main

import (
	"net/http"
	"testing"
)

func TestIsDatacenterIP(t *testing.T) {
	cases := map[string]bool{
		"127.0.0.1":    false, // loopback
		"::1":          false, // loopback v6
		"10.1.2.3":     false, // private 10/8
		"192.168.65.1": false, // private 192.168/16 (Docker Desktop gw)
		"172.16.0.1":   false, // private 172.16/12 low edge
		"172.30.0.1":   false, // Docker compose bridge gw (regression: old stub flagged this)
		"172.31.255.9": false, // private 172.16/12 high edge
		"172.18.0.5":   false, // Docker default bridge
		"169.254.1.1":  false, // link-local
		"fc00::1":      false, // IPv6 ULA (private)
		"0.0.0.0":      false, // unspecified
		"garbage":      false, // unresolvable → do not accuse
		"":             false,
		"8.8.8.8":      true, // public
		"1.2.3.4":      true, // public
		"172.32.0.1":   true, // just ABOVE 172.16/12 → public
	}
	for ip, want := range cases {
		if got := isDatacenterIP(ip); got != want {
			t.Errorf("isDatacenterIP(%q)=%v, want %v", ip, got, want)
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

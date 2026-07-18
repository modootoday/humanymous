package network

import (
	"strings"
	"testing"
)

// JA4's headline property: it is order-insensitive (ciphers/extensions are
// sorted), so it stays STABLE across Chrome's per-connection ClientHello
// permutation — the exact evasion JA3 falls to (SoT-02 §JA4).
func TestJA4PermutationStable(t *testing.T) {
	h1 := &ClientHello{
		LegacyVersion: 0x0303,
		SupportedVers: []uint16{0x0304, 0x0303},
		CipherSuites:  []uint16{0x1301, 0x1302, 0x1303, 0xc02b},
		Extensions:    []uint16{0x0000, 0x0010, 0x000d, 0x002b, 0x000a},
		SigAlgs:       []uint16{0x0403, 0x0804, 0x0401},
		ALPN:          []string{"h2", "http/1.1"},
		SNIPresent:    true,
	}
	// Same sets, permuted cipher + extension order (sigalgs order preserved —
	// JA4 keeps sigalgs in wire order by design).
	h2 := &ClientHello{
		LegacyVersion: 0x0303,
		SupportedVers: []uint16{0x0303, 0x0304},
		CipherSuites:  []uint16{0xc02b, 0x1303, 0x1301, 0x1302},
		Extensions:    []uint16{0x002b, 0x000a, 0x0010, 0x0000, 0x000d},
		SigAlgs:       []uint16{0x0403, 0x0804, 0x0401},
		ALPN:          []string{"h2", "http/1.1"},
		SNIPresent:    true,
	}
	j1, _ := JA4(h1)
	j2, _ := JA4(h2)
	if j1 != j2 {
		t.Fatalf("JA4 not permutation-stable:\n %q\n %q", j1, j2)
	}
	// a-section encodes TLS1.3 (13), SNI present (d).
	if !strings.HasPrefix(j1, "t13d") {
		t.Errorf("a-section = %q, want prefix t13d", j1)
	}
}

// GREASE code points must be stripped before hashing, so an injected GREASE
// cipher/extension does not change the fingerprint.
func TestJA4StripsGREASE(t *testing.T) {
	base := &ClientHello{
		LegacyVersion: 0x0303, SupportedVers: []uint16{0x0304},
		CipherSuites: []uint16{0x1301, 0x1302}, Extensions: []uint16{0x0000, 0x000d},
		SigAlgs: []uint16{0x0403}, SNIPresent: true,
	}
	greased := &ClientHello{
		LegacyVersion: 0x0303, SupportedVers: []uint16{0x0304},
		CipherSuites: []uint16{0x0a0a, 0x1301, 0x1302}, Extensions: []uint16{0x1a1a, 0x0000, 0x000d},
		SigAlgs: []uint16{0x0403}, SNIPresent: true,
	}
	jb, _ := JA4(base)
	jg, _ := JA4(greased)
	if jb != jg {
		t.Errorf("GREASE not stripped: %q vs %q", jb, jg)
	}
}

func TestJA4Deterministic(t *testing.T) {
	h := &ClientHello{
		LegacyVersion: 0x0303, SupportedVers: []uint16{0x0304},
		CipherSuites: []uint16{0x1301}, Extensions: []uint16{0x0000}, SigAlgs: []uint16{0x0403},
		ALPN: []string{"h2"}, SNIPresent: true,
	}
	a, _ := JA4(h)
	b, _ := JA4(h)
	if a != b {
		t.Errorf("JA4 not deterministic: %q vs %q", a, b)
	}
}

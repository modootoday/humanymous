package network

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"sort"
	"strings"
)

// ja4.go computes the JA4 TLS client fingerprint (FoxIO spec). JA4 is
// order-insensitive (ciphers/extensions are sorted) so it survives Chrome's
// ClientHello extension permutation, unlike JA3 (SoT-02 §JA4).
//
// JA4 = ja4_a "_" ja4_b "_" ja4_c
//   a = {proto}{tlsver}{sni}{cipherCount:02}{extCount:02}{alpn}
//   b = sha256(sorted cipher hex list) [:12]
//   c = sha256(sorted ext hex list "_" sigalgs-in-order) [:12]

// JA4 returns the hashed JA4 and a raw (unhashed) JA4_r variant for debugging.
func JA4(ch *ClientHello) (ja4 string, raw string) {
	a := ja4a(ch)

	ciphers := hexSortedList(stripGREASE(ch.CipherSuites))
	b := trunc12(sha256Hex(strings.Join(ciphers, ",")))
	if len(ciphers) == 0 {
		b = "000000000000"
	}

	// c: extensions minus SNI(0x0000) and ALPN(0x0010), sorted; then sigalgs in
	// original order appended after '_'.
	exts := stripGREASE(ch.Extensions)
	exts = removeU16(exts, 0x0000, 0x0010)
	extHex := hexSortedList(exts)
	sigHex := hexListInOrder(ch.SigAlgs) // original order, no sort
	cInput := strings.Join(extHex, ",") + "_" + strings.Join(sigHex, ",")
	c := trunc12(sha256Hex(cInput))
	if len(extHex) == 0 {
		c = "000000000000"
	}

	rawC := strings.Join(extHex, ",") + "_" + strings.Join(sigHex, ",")
	return a + "_" + b + "_" + c, a + "_" + strings.Join(ciphers, ",") + "_" + rawC
}

// ja4a builds the human-readable a-section.
func ja4a(ch *ClientHello) string {
	proto := "t" // TLS over TCP (this project doesn't parse QUIC/DTLS)
	ver := tlsVersionCode(ch)
	sni := "i"
	if ch.SNIPresent {
		sni = "d"
	}
	cc := cap2(len(stripGREASE(ch.CipherSuites)))
	ec := cap2(len(stripGREASE(ch.Extensions))) // SNI+ALPN counted here (spec)
	alpn := alpnCode(ch.ALPN)
	return fmt.Sprintf("%s%s%s%02d%02d%s", proto, ver, sni, cc, ec, alpn)
}

// tlsVersionCode maps the highest offered TLS version to JA4's 2-char code.
func tlsVersionCode(ch *ClientHello) string {
	ver := ch.LegacyVersion
	if len(ch.SupportedVers) > 0 {
		ver = maxNonGREASE(ch.SupportedVers)
	}
	switch ver {
	case 0x0304:
		return "13"
	case 0x0303:
		return "12"
	case 0x0302:
		return "11"
	case 0x0301:
		return "10"
	default:
		return "00"
	}
}

// alpnCode returns first+last char of the first ALPN value, or "00".
func alpnCode(alpn []string) string {
	if len(alpn) == 0 || alpn[0] == "" {
		return "00"
	}
	v := alpn[0]
	return string(v[0]) + string(v[len(v)-1])
}

// cap2 caps a count at 99 (JA4 uses a 2-digit count).
func cap2(n int) int {
	if n > 99 {
		return 99
	}
	return n
}

// hexSortedList returns 4-hex-char strings sorted ascending by value.
func hexSortedList(vs []uint16) []string {
	cp := append([]uint16(nil), vs...)
	sort.Slice(cp, func(i, j int) bool { return cp[i] < cp[j] })
	return hexListInOrder(cp)
}

// hexListInOrder returns 4-hex-char strings preserving input order.
func hexListInOrder(vs []uint16) []string {
	out := make([]string, len(vs))
	for i, v := range vs {
		out[i] = fmt.Sprintf("%04x", v)
	}
	return out
}

func removeU16(vs []uint16, drop ...uint16) []uint16 {
	dropset := map[uint16]bool{}
	for _, d := range drop {
		dropset[d] = true
	}
	out := make([]uint16, 0, len(vs))
	for _, v := range vs {
		if !dropset[v] {
			out = append(out, v)
		}
	}
	return out
}

func sha256Hex(s string) string {
	sum := sha256.Sum256([]byte(s))
	return hex.EncodeToString(sum[:])
}

func trunc12(s string) string {
	if len(s) < 12 {
		return s
	}
	return s[:12]
}

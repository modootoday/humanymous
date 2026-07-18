package network

import (
	"crypto/md5"
	"encoding/hex"
	"strconv"
	"strings"
)

// ja3.go computes the JA3 fingerprint (Salesforce). JA3 is kept for
// compatibility/logging only: Chrome's ClientHello extension permutation makes
// it unstable, so scoring weight is ~0 (SoT-02 §JA3). Prefer JA4.
//
// JA3 string = TLSVersion,Ciphers,Extensions,Curves,PointFormats  (MD5 hashed)
// GREASE values are excluded from every field.

// JA3 returns the JA3 MD5 hex and the underlying JA3 string.
func JA3(ch *ClientHello) (hash string, text string) {
	ver := ch.LegacyVersion
	if len(ch.SupportedVers) > 0 {
		ver = maxNonGREASE(ch.SupportedVers)
	}
	fields := []string{
		strconv.Itoa(int(ver)),
		joinU16(stripGREASE(ch.CipherSuites)),
		joinU16(stripGREASE(ch.Extensions)),
		joinU16(stripGREASE(ch.Curves)),
		joinU16(stripGREASE(ch.PointFormats)),
	}
	text = strings.Join(fields, ",")
	sum := md5.Sum([]byte(text))
	return hex.EncodeToString(sum[:]), text
}

// joinU16 joins values with '-' (JA3 intra-field separator).
func joinU16(vs []uint16) string {
	parts := make([]string, len(vs))
	for i, v := range vs {
		parts[i] = strconv.Itoa(int(v))
	}
	return strings.Join(parts, "-")
}

// maxNonGREASE returns the largest non-GREASE value (for highest TLS version).
func maxNonGREASE(vs []uint16) uint16 {
	var max uint16
	for _, v := range vs {
		if isGREASE(v) {
			continue
		}
		if v > max {
			max = v
		}
	}
	return max
}

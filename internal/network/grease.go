// Package network computes L5 protocol fingerprints (TLS JA3/JA4, HTTP/2
// Akamai, header order) and maps them to browser-engine families for the L6
// cross-check. See sots/02-network-tls-signals.md.
package network

// grease.go isolates GREASE handling (RFC 8701). GREASE values are the 16
// reserved 0x?a?a code points browsers inject; they must be stripped before
// computing stable fingerprints (JA3/JA4).

// isGREASE reports whether v is one of the 16 GREASE code points (0x0a0a,
// 0x1a1a, ... 0xfafa) — the pattern is high nibble == low nibble of each byte
// and both bytes equal, i.e. 0x?a?a with the two bytes identical.
func isGREASE(v uint16) bool {
	// GREASE values: both bytes equal and low nibble == 0x0a.
	return (v&0x0f0f) == 0x0a0a && (v>>8) == (v&0x00ff)
}

// stripGREASE returns a copy of vs with GREASE code points removed.
func stripGREASE(vs []uint16) []uint16 {
	out := make([]uint16, 0, len(vs))
	for _, v := range vs {
		if !isGREASE(v) {
			out = append(out, v)
		}
	}
	return out
}

package network

import (
	"crypto/sha256"
	"encoding/hex"
	"strconv"
	"strings"
)

// permutation.go supports TLS ClientHello extension-permutation detection
// (SoT-14 §B1). Chrome 110+ permutes its extension order every connection, so a
// real Chrome yields a DIFFERENT extension-order hash on each connection. A
// static parrot (uTLS HelloChrome, curl-impersonate) sends the SAME order every
// time. Comparing extension-order hashes across a session's connections reveals
// the static parrot (done in the traffic guard, SoT-12).

// ExtOrderHash returns a short hash of the non-GREASE extension order (in wire
// order). Empty if no extensions captured.
func ExtOrderHash(ch *ClientHello) string {
	if ch == nil || len(ch.Extensions) == 0 {
		return ""
	}
	var parts []string
	for _, e := range ch.Extensions {
		if isGREASE(e) {
			continue
		}
		parts = append(parts, strconv.Itoa(int(e)))
	}
	if len(parts) == 0 {
		return ""
	}
	sum := sha256.Sum256([]byte(strings.Join(parts, "-")))
	return hex.EncodeToString(sum[:6])
}

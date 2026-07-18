package integrity

import (
	"crypto/sha256"
	"encoding/hex"
	"sort"
	"strings"
)

// canonical.go defines the request canonicalization that both client and server
// sign over (SoT-07 §2.3). Only tamper-relevant headers are included, in a
// fixed order, normalized (trimmed, collapsed whitespace, lowercased names).
// Headers that browsers/proxies legitimately rewrite are excluded to avoid FPs.

// SignedHeaders is the fixed, ordered set of headers included in the canonical
// string. Order is stable so client and server agree byte-for-byte.
var SignedHeaders = []string{
	"user-agent",
	"sec-ch-ua",
	"sec-ch-ua-platform",
	"sec-ch-ua-mobile",
	"accept-language",
	"accept",
}

// CanonicalInput is the tamper-relevant view of a request.
type CanonicalInput struct {
	Headers  map[string]string // lowercased header name -> value
	CookieID string            // hsid cookie value (session binding)
	Body     []byte            // request body (hashed)
}

// Canonical builds the canonical string signed by the RIT token.
//
//	<h1name>:<h1val>\n...\n cookie:<hsid>\n body:<sha256hex>
func Canonical(in CanonicalInput) string {
	var b strings.Builder
	for _, name := range SignedHeaders {
		b.WriteString(name)
		b.WriteByte(':')
		b.WriteString(normalizeHeader(in.Headers[name]))
		b.WriteByte('\n')
	}
	b.WriteString("cookie:")
	b.WriteString(in.CookieID)
	b.WriteByte('\n')
	b.WriteString("body:")
	b.WriteString(sha256hex(in.Body))
	return b.String()
}

// normalizeHeader trims and collapses internal whitespace so cosmetic
// differences don't break the signature.
func normalizeHeader(v string) string {
	return strings.Join(strings.Fields(strings.TrimSpace(v)), " ")
}

func sha256hex(b []byte) string {
	sum := sha256.Sum256(b)
	return hex.EncodeToString(sum[:])
}

// LowerHeaderMap normalizes a name->value map to lowercase keys (helper for the
// server middleware which receives canonical-cased names).
func LowerHeaderMap(in map[string]string) map[string]string {
	out := make(map[string]string, len(in))
	keys := make([]string, 0, len(in))
	for k := range in {
		keys = append(keys, k)
	}
	sort.Strings(keys) // deterministic on duplicate-cased keys
	for _, k := range keys {
		out[strings.ToLower(k)] = in[k]
	}
	return out
}

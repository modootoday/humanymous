package watermark

import "strings"

// codec.go defines the per-MIME Codec contract and registry (SoT-08 §9). Each
// codec embeds/recovers a framed carrier while preserving the resource's
// semantics (render/parse/execute result unchanged). inject.go/extract.go route
// by MIME to the registered codec.

// Codec injects and recovers a framed carrier for one resource family.
type Codec interface {
	// Encode injects framed(carrier) into src, preserving semantics.
	Encode(src, framed []byte) ([]byte, error)
	// Decode recovers the framed carrier (best-effort per channel).
	Decode(src []byte) ([]byte, error)
	// Name identifies the codec/channel for the ledger + reports.
	Name() string
}

// registry maps a normalized MIME family to a codec.
var registry = map[string]Codec{}

// Register binds a codec to a MIME family key (e.g. "text", "image/png").
func Register(key string, c Codec) { registry[key] = c }

// codecFor resolves a codec from a full MIME type, falling back to the generic
// binary trailer codec for unknown types (SoT-08 §3.4 fallback).
func codecFor(mime string) Codec {
	key := familyKey(mime)
	if c, ok := registry[key]; ok {
		return c
	}
	if c, ok := registry["text"]; ok && isText(mime) {
		return c
	}
	return registry["binary"] // trailer fallback (always registered)
}

// familyKey normalizes a MIME type to a registry key.
func familyKey(mime string) string {
	m := strings.ToLower(strings.TrimSpace(mime))
	if i := strings.IndexByte(m, ';'); i >= 0 {
		m = strings.TrimSpace(m[:i])
	}
	switch {
	case m == "image/png":
		return "image/png"
	case m == "application/wasm":
		return "application/wasm"
	case isText(m):
		return "text"
	default:
		return "binary"
	}
}

// isText reports whether a MIME type is a text/code family we can comment-inject.
func isText(mime string) bool {
	m := strings.ToLower(mime)
	if strings.HasPrefix(m, "text/") {
		return true
	}
	for _, t := range []string{"javascript", "json", "xml", "svg", "css", "html", "ecmascript"} {
		if strings.Contains(m, t) {
			return true
		}
	}
	return false
}

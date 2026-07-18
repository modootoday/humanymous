// Package resource implements adaptive resource gating and embed/media
// inspection (SoT-10): heavy resources (video/embeds) are served, downgraded or
// denied by session trust to save bandwidth and block scrapers/leeches.
package resource

import (
	"path"
	"strings"
)

// Tier is a resource cost class (SoT-10 §1).
type Tier int

const (
	TierEssential Tier = iota // T0: html/css/js/wasm/RIT — always served
	TierLight                 // T1: small images, json
	TierMedia                 // T2: large images, audio
	TierHeavy                 // T3: video, large downloads
	TierEmbed                 // T4: iframe/embed contexts
)

func (t Tier) String() string {
	switch t {
	case TierEssential:
		return "T0-essential"
	case TierLight:
		return "T1-light"
	case TierMedia:
		return "T2-media"
	case TierHeavy:
		return "T3-heavy"
	case TierEmbed:
		return "T4-embed"
	default:
		return "unknown"
	}
}

// Classify assigns a cost tier from the request path, MIME and size (SoT-10 §1).
// secFetchDest is the Sec-Fetch-Dest header (empty if absent).
func Classify(urlPath, mime string, sizeBytes int64, secFetchDest string) Tier {
	ext := strings.ToLower(path.Ext(urlPath))
	m := strings.ToLower(mime)

	// Embed contexts are driven by the fetch destination.
	switch secFetchDest {
	case "iframe", "embed", "object", "frame":
		return TierEmbed
	}

	switch {
	case isEssentialExt(ext) || strings.Contains(m, "html") || strings.Contains(m, "css") ||
		strings.Contains(m, "javascript") || strings.Contains(m, "wasm"):
		return TierEssential
	case strings.HasPrefix(m, "video/") || ext == ".mp4" || ext == ".webm" || ext == ".m3u8" || ext == ".ts":
		return TierHeavy
	case sizeBytes > 2<<20: // >2 MiB anything is heavy
		return TierHeavy
	case strings.HasPrefix(m, "audio/") || (strings.HasPrefix(m, "image/") && sizeBytes > 256<<10):
		return TierMedia
	case strings.HasPrefix(m, "image/") || strings.Contains(m, "json"):
		return TierLight
	default:
		return TierLight
	}
}

func isEssentialExt(ext string) bool {
	switch ext {
	case ".html", ".htm", ".css", ".js", ".mjs", ".wasm", ".map":
		return true
	}
	return false
}

package scoring

import "strings"

// ua.go holds the minimal User-Agent -> engine-family classification used by
// the L6 cross-checks. It is intentionally coarse: we only need the engine
// family the UA *claims*, to diff against what the network layer *observed*.
// Full UA parsing is out of scope (SoT-02 §L6).

// engineFamily buckets: chrome | firefox | safari | other.
func claimedEngine(ua string) string {
	u := strings.ToLower(ua)
	switch {
	// Order matters: Edge/Chrome both contain "chrome"; treat Chromium family
	// together since their TLS/H2 engine is BoringSSL either way.
	case strings.Contains(u, "firefox"):
		return "firefox"
	case strings.Contains(u, "chrome") || strings.Contains(u, "chromium") || strings.Contains(u, "edg/"):
		return "chrome"
	case strings.Contains(u, "safari") && !strings.Contains(u, "chrome"):
		return "safari"
	default:
		return "other"
	}
}

// claimsChromium reports whether the UA claims a Chromium-based browser, which
// on HTTPS must always send Sec-CH-UA and sec-fetch-* headers (SoT-02 §4).
func claimsChromium(ua string) bool {
	return claimedEngine(ua) == "chrome"
}

// isBrowserUA reports whether the UA looks like a real browser at all (vs a
// library default like "Go-http-client/2.0" or "python-requests/2.x").
func isBrowserUA(ua string) bool {
	u := strings.ToLower(ua)
	if u == "" {
		return false
	}
	for _, lib := range []string{"go-http-client", "python-requests", "curl/", "okhttp", "axios", "node-fetch", "java/", "libwww"} {
		if strings.Contains(u, lib) {
			return false
		}
	}
	return strings.Contains(u, "mozilla")
}

// engineConsistent reports whether an observed network engine is compatible
// with the claimed UA engine. "unknown"/"" observed is treated as no evidence.
func engineConsistent(claimed, observed string) bool {
	if observed == "" || observed == "unknown" {
		return true // no evidence -> don't flag
	}
	// Chrome family (BoringSSL) is the only browser here; firefox=NSS; safari.
	return claimed == observed
}

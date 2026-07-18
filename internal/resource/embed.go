package resource

import (
	"strings"

	"github.com/modootoday/humanymous/internal/signals"
)

// embed.go inspects embed/iframe resource contexts (SoT-10 §3.1): the embedding
// origin must be allow-listed and the RIT token must survive into the embed
// context (a stripped token signals an embed-based evasion).

// EmbedContext describes an embed/iframe request.
type EmbedContext struct {
	SecFetchDest string   // iframe/embed/object/frame
	Origin       string   // Origin header (or Referer host)
	AllowedHosts []string // embedding allow-list
	RITPresent   bool
}

// IsEmbed reports whether the request is an embed/iframe context.
func (e EmbedContext) IsEmbed() bool {
	switch e.SecFetchDest {
	case "iframe", "embed", "object", "frame":
		return true
	}
	return false
}

// Inspect returns embed signals for a request (SoT-10 §4).
func Inspect(e EmbedContext) []signals.Signal {
	if !e.IsEmbed() {
		return nil
	}
	var out []signals.Signal
	add := func(id string, val any, v signals.Verdict, notes string) {
		out = append(out, signals.New(id, val, v, 1.0, signals.SourceServer, notes))
	}
	if e.Origin != "" && !hostAllowed(e.Origin, e.AllowedHosts) {
		add("l5.embed.origin_disallowed", e.Origin, signals.VerdictSuspicious, "embed from non-allowlisted origin")
	}
	if !e.RITPresent {
		add("l5.embed.token_stripped", true, signals.VerdictBot, "RIT token absent in embed context")
	}
	return out
}

func hostAllowed(origin string, allowed []string) bool {
	o := normalizeHost(origin)
	for _, a := range allowed {
		if normalizeHost(a) == o {
			return true
		}
	}
	return len(allowed) == 0 // empty allow-list => permit (demo default)
}

func normalizeHost(s string) string {
	s = strings.ToLower(strings.TrimSpace(s))
	s = strings.TrimPrefix(s, "https://")
	s = strings.TrimPrefix(s, "http://")
	if i := strings.IndexByte(s, '/'); i >= 0 {
		s = s[:i]
	}
	return s
}

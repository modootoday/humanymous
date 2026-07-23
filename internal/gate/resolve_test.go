package gate

import "testing"

// resolve_test.go pins the path-normalization + segment-boundary matching that closes the
// deep-review path-confusion bypass: an obfuscated path that an origin collapses back to a
// protected route must resolve to that route's (stricter) preset, not fall through to the
// weaker balanced default.
func TestResolve_PathConfusionSelectsStrict(t *testing.T) {
	c := Config{Routes: map[string]string{"/admin": "strict", "/health": "off"}}
	strictCases := []string{"/admin", "/admin/", "/admin/users", "/./admin", "//admin", "/admin/../admin", "/foo/../admin"}
	for _, p := range strictCases {
		if got := c.resolve(p); got.name != "strict" {
			t.Errorf("resolve(%q) = %q, want strict (path-confusion must not bypass)", p, got.name)
		}
	}
	// Segment anchoring: /health (off) must NOT over-scope onto a look-alike path.
	if got := c.resolve("/healthstatus-secret"); got.name == "off" {
		t.Errorf("resolve(/healthstatus-secret) = off, but /health must match on a segment boundary only")
	}
	if got := c.resolve("/health"); got.name != "off" {
		t.Errorf("resolve(/health) = %q, want off (exact match)", got.name)
	}
	// An unrelated path takes the balanced default.
	if got := c.resolve("/store/item"); got.name != "balanced" {
		t.Errorf("resolve(/store/item) = %q, want balanced default", got.name)
	}
}

package sentinel

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/modootoday/humanymous/internal/audit"
)

// WS5: a state-changing method with no prior verdict fails CLOSED (challenge),
// even on a balanced route that fails OPEN for safe methods — closing the bare
// POST /checkout with only a User-Agent path.
func TestUnsafeMethodFailsClosed(t *testing.T) {
	originHits := 0
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		originHits++
		w.Write([]byte("ok"))
	}))
	defer up.Close()
	srv, alog := buildStack(t, up.URL) // default route = balanced (fail-open on GET)

	// A bare GET on a balanced route with no evidence still passes (fail-open).
	g := do(srv, "GET", "/cart", "", "", map[string]string{"User-Agent": "Chrome/126"})
	if g.Code == http.StatusUnauthorized {
		t.Fatal("safe GET on a balanced route must fail open, not challenge")
	}

	// A bare POST (mutation) with no verdict must be challenged (401), and the
	// origin must not be contacted.
	before := originHits
	p := do(srv, "POST", "/cart", "", "x=1", map[string]string{"User-Agent": "Chrome/126", "Content-Type": "text/plain"})
	if p.Code != http.StatusUnauthorized {
		t.Fatalf("unsafe POST with no evidence must challenge (401), got %d", p.Code)
	}
	if originHits != before {
		t.Fatal("origin was contacted for a fail-closed mutation")
	}
	if !hasEvent(alog, "enf.failclosed.mutating") {
		t.Fatal("a mutating fail-closed must emit enf.failclosed.mutating")
	}
}

// WS5: the action mapping is method-aware — Unknown is pass on safe balanced,
// challenge on unsafe or sensitive/sync routes.
func TestFailClosedActionMapping(t *testing.T) {
	cases := []struct {
		verdict Verdict
		route   routePolicy
		unsafe  bool
		want    string
	}{
		{VerdictUnknown, presetBalanced, false, "pass"},        // safe GET, public → open
		{VerdictUnknown, presetBalanced, true, "challenge_pow"}, // mutation → closed
		{VerdictUnknown, presetStrict, false, "challenge_pow"},  // sensitive route → closed
		{VerdictAllow, presetBalanced, true, "pass"},            // scored ALLOW passes
		{VerdictDeny, presetBalanced, false, "block"},
	}
	for i, c := range cases {
		_, got := c.verdict.action(c.route, c.unsafe)
		if got != c.want {
			t.Errorf("case %d: got %q want %q", i, got, c.want)
		}
	}
	_ = audit.EventEnfAllow
}

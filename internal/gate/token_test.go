package gate

import (
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

func reqWith(ua, lang, chua string) string {
	r := httptest.NewRequest("GET", "http://p/", nil)
	r.Header.Set("User-Agent", ua)
	r.Header.Set("Accept-Language", lang)
	r.Header.Set("sec-ch-ua", chua)
	return bindKey(r)
}

func TestVerdictTokenRoundTrip(t *testing.T) {
	key := []byte("server-only-token-key")
	now := time.Unix(1_000_000, 0)
	bind := reqWith("Chrome/126", "en-US", `"Chromium";v="126"`)
	tok := issueVerdictToken(key, "sid1", bind, "e1", now.Add(time.Minute))
	if r := verifyVerdictToken(key, tok, bind, "sid1", now, "e1", "e0"); r != tokenOK {
		t.Fatalf("valid token rejected: %s", r)
	}
}

// a token lifted to a different client fingerprint (bot with different UA) fails
// the binding check (HR-28).
func TestVerdictTokenBindingMismatch(t *testing.T) {
	key := []byte("server-only-token-key")
	now := time.Unix(1_000_000, 0)
	humanBind := reqWith("Chrome/126", "en-US", `"Chromium";v="126"`)
	botBind := reqWith("HeadlessChrome/126", "", "")
	tok := issueVerdictToken(key, "sid1", humanBind, "e1", now.Add(time.Minute))
	if r := verifyVerdictToken(key, tok, botBind, "sid1", now, "e1", "e0"); r != tokenBindingMismatch {
		t.Fatalf("lifted token not caught: %s", r)
	}
}

// WS6: the token is bound to the source SUBNET and SESSION — a token replayed
// from a different network (datacenter bot) or a different session cookie fails,
// even if the three fingerprint headers are copied exactly.
func TestVerdictTokenSubnetAndSidBinding(t *testing.T) {
	key := []byte("server-only-token-key")
	now := time.Unix(1_000_000, 0)
	reqAt := func(ip string) *http.Request {
		r := httptest.NewRequest("GET", "http://p/", nil)
		r.RemoteAddr = ip + ":4000"
		r.Header.Set("User-Agent", "Chrome/126")
		r.Header.Set("Accept-Language", "en-US")
		r.Header.Set("sec-ch-ua", `"Chromium";v="126"`)
		return r
	}
	human := reqAt("203.0.113.10")            // residential
	tok := issueVerdictToken(key, "sidH", tokenBind(human), "e1", now.Add(time.Minute))

	// Same subnet + same session → OK.
	if r := verifyVerdictToken(key, tok, tokenBind(reqAt("203.0.113.99")), "sidH", now, "e1"); r != tokenOK {
		t.Fatalf("same-/24 same-session token should verify, got %s", r)
	}
	// Copied headers, DIFFERENT subnet (datacenter bot) → binding mismatch.
	if r := verifyVerdictToken(key, tok, tokenBind(reqAt("198.51.100.7")), "sidH", now, "e1"); r != tokenBindingMismatch {
		t.Fatalf("cross-subnet replay must fail, got %s", r)
	}
	// Same subnet but a DIFFERENT session cookie → binding mismatch.
	if r := verifyVerdictToken(key, tok, tokenBind(reqAt("203.0.113.99")), "sidBOT", now, "e1"); r != tokenBindingMismatch {
		t.Fatalf("cross-session replay must fail, got %s", r)
	}
}

// a forged token (wrong key / alg downgrade attempt) fails the signature check.
func TestVerdictTokenForgery(t *testing.T) {
	now := time.Unix(1_000_000, 0)
	bind := reqWith("Chrome/126", "en-US", "")
	forged := issueVerdictToken([]byte("attacker-key"), "sid1", bind, "e1", now.Add(time.Minute))
	if r := verifyVerdictToken([]byte("server-only-token-key"), forged, bind, "sid1", now, "e1"); r != tokenBadSig {
		t.Fatalf("forged token not caught: %s", r)
	}
}

func TestVerdictTokenExpiry(t *testing.T) {
	key := []byte("k")
	now := time.Unix(1_000_000, 0)
	bind := reqWith("Chrome/126", "en-US", "")
	tok := issueVerdictToken(key, "sid1", bind, "e1", now.Add(-time.Second)) // already expired
	if r := verifyVerdictToken(key, tok, bind, "sid1", now, "e1"); r != tokenExpired {
		t.Fatalf("expired token accepted: %s", r)
	}
}

// beacon nonce is single-use; a replay (same nonce, or reused across bindings)
// is rejected (HR-29).
func TestNonceReplay(t *testing.T) {
	c := NewNonceCache(time.Minute)
	now := time.Unix(1_000_000, 0)
	if !c.Use("n1", "bindA", now) {
		t.Fatal("first use must succeed")
	}
	if c.Use("n1", "bindA", now) {
		t.Fatal("same nonce+binding replay must be rejected")
	}
	if c.Use("n1", "bindB", now) {
		t.Fatal("nonce reused across a different binding must be rejected (solve-once-reuse-many)")
	}
	if !c.Use("n2", "bindA", now) {
		t.Fatal("a fresh nonce must succeed")
	}
}

package gate

import (
	"crypto/ed25519"
	"encoding/base64"
	"strconv"
	"testing"
	"time"
)

// webbotauth_test.go verifies the RFC 9421 Web Bot Auth path: a valid signature from
// an allowlisted key is trusted, a forgery of an allowlisted key is caught, and an
// unknown key / expired / absent signature is neutral (never a false deny). The covered
// set binds the request line (@authority @method @path), so a lifted signature cannot be
// replayed to a different path/method, and a single-use nonce blocks same-request replay.

const testKID = "test-agent-key-1"

var wbaComponents = []string{"@authority", "@method", "@path"}

// signWebBotAuth produces a valid Signature-Input + Signature over the request line,
// mirroring the verifier's base construction (the two must agree for verification to hold).
func signWebBotAuth(host, method, path, kid string, priv ed25519.PrivateKey, created int64, nonce string) (string, string) {
	inner := `("@authority" "@method" "@path");created=` + strconv.FormatInt(created, 10) +
		`;keyid="` + kid + `";alg="ed25519";tag="web-bot-auth"`
	if nonce != "" {
		inner += `;nonce="` + nonce + `"`
	}
	base := buildSignatureBase(host, method, path, wbaComponents, inner)
	sig := ed25519.Sign(priv, []byte(base))
	return "sig1=" + inner, "sig1=:" + base64.StdEncoding.EncodeToString(sig) + ":"
}

func testDir(t *testing.T, kid string, pub ed25519.PublicKey) KeyDirectory {
	t.Helper()
	spec := kid + " " + base64.RawURLEncoding.EncodeToString(pub)
	d, err := NewStaticKeyDirectory(spec)
	if err != nil {
		t.Fatal(err)
	}
	return d
}

func TestVerifyAgentTrusted(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	nc := NewNonceCache(time.Hour)
	now := time.Unix(1_700_000_000, 0)
	si, sig := signWebBotAuth("example.com", "GET", "/data", testKID, priv, now.Unix(), "n1")
	if v, kid := verifyAgent("example.com", "GET", "/data", si, sig, dir, nc, now); v != agentVerifiedTrusted || kid != testKID {
		t.Fatalf("valid signature from allowlisted key: got verdict %d kid %q, want trusted/%s", v, kid, testKID)
	}
}

func TestVerifyAgentForged(t *testing.T) {
	pub, _, _ := ed25519.GenerateKey(nil)      // the ALLOWLISTED key
	_, attacker, _ := ed25519.GenerateKey(nil) // a DIFFERENT key the attacker holds
	dir := testDir(t, testKID, pub)            // directory trusts pub, not attacker
	now := time.Unix(1_700_000_000, 0)
	// Attacker signs with their own key but CLAIMS the allowlisted keyid.
	si, sig := signWebBotAuth("example.com", "GET", "/data", testKID, attacker, now.Unix(), "n1")
	if v, _ := verifyAgent("example.com", "GET", "/data", si, sig, dir, NewNonceCache(time.Hour), now); v != agentForged {
		t.Fatalf("forgery of an allowlisted keyid must be agentForged, got %d", v)
	}
}

func TestVerifyAgentTamperedAuthority(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	si, sig := signWebBotAuth("example.com", "GET", "/data", testKID, priv, now.Unix(), "n1")
	// Same signature replayed against a DIFFERENT authority must not verify (forgery).
	if v, _ := verifyAgent("victim.com", "GET", "/data", si, sig, dir, NewNonceCache(time.Hour), now); v != agentForged {
		t.Fatalf("signature bound to example.com must not verify for victim.com, got %d", v)
	}
}

// TestVerifyAgentReplayDifferentRequestLine: a signature captured on one path/method must
// not verify when replayed on another — the request line is now covered (deep-review).
func TestVerifyAgentReplayDifferentRequestLine(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	si, sig := signWebBotAuth("example.com", "GET", "/public", testKID, priv, now.Unix(), "n1")
	// Replay to a different PATH → base mismatch → forged (does not verify).
	if v, _ := verifyAgent("example.com", "GET", "/admin", si, sig, dir, NewNonceCache(time.Hour), now); v != agentForged {
		t.Fatalf("signature bound to /public must not verify for /admin, got %d", v)
	}
	// Replay with a different METHOD → base mismatch → forged.
	if v, _ := verifyAgent("example.com", "POST", "/public", si, sig, dir, NewNonceCache(time.Hour), now); v != agentForged {
		t.Fatalf("GET signature must not verify for POST, got %d", v)
	}
}

// TestVerifyAgentNonceSingleUse: a valid signature with a nonce is trusted once; a replay of
// the SAME nonce (same request line, within lifetime) confers no upgrade.
func TestVerifyAgentNonceSingleUse(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	nc := NewNonceCache(time.Hour)
	now := time.Unix(1_700_000_000, 0)
	si, sig := signWebBotAuth("example.com", "GET", "/data", testKID, priv, now.Unix(), "nonce-xyz")
	if v, _ := verifyAgent("example.com", "GET", "/data", si, sig, dir, nc, now); v != agentVerifiedTrusted {
		t.Fatalf("first use must be trusted, got %d", v)
	}
	if v, _ := verifyAgent("example.com", "GET", "/data", si, sig, dir, nc, now); v != agentVerifiedUnknown {
		t.Fatalf("replayed nonce must not be trusted again, got %d", v)
	}
}

// TestVerifyAgentAuthorityOnlyRejected: a signature covering only @authority (no request
// line) is unverifiable here → neutral, closing the any-path/any-method replay.
func TestVerifyAgentAuthorityOnlyRejected(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	inner := `("@authority");created=` + strconv.FormatInt(now.Unix(), 10) + `;keyid="` + testKID + `";alg="ed25519";tag="web-bot-auth"`
	base := buildSignatureBase("example.com", "GET", "/data", []string{"@authority"}, inner)
	sig := "sig1=:" + base64.StdEncoding.EncodeToString(ed25519.Sign(priv, []byte(base))) + ":"
	if v, _ := verifyAgent("example.com", "GET", "/data", "sig1="+inner, sig, dir, NewNonceCache(time.Hour), now); v != agentVerifiedUnknown {
		t.Fatalf("an @authority-only signature must be neutral (unverifiable request-line), got %d", v)
	}
}

func TestVerifyAgentUnknownKey(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	// A valid signature but keyid not in the directory → neutral, never a deny.
	si, sig := signWebBotAuth("example.com", "GET", "/data", "some-other-kid", priv, now.Unix(), "n1")
	if v, _ := verifyAgent("example.com", "GET", "/data", si, sig, dir, NewNonceCache(time.Hour), now); v != agentVerifiedUnknown {
		t.Fatalf("unknown keyid must be neutral (agentVerifiedUnknown), got %d", v)
	}
}

func TestVerifyAgentExpired(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	// created far in the past with an expires also in the past → stale → neutral.
	inner := `("@authority" "@method" "@path");created=1000000000;expires=1000000100;keyid="` + testKID + `";alg="ed25519";tag="web-bot-auth"`
	base := buildSignatureBase("example.com", "GET", "/data", wbaComponents, inner)
	sig := "sig1=:" + base64.StdEncoding.EncodeToString(ed25519.Sign(priv, []byte(base))) + ":"
	if v, _ := verifyAgent("example.com", "GET", "/data", "sig1="+inner, sig, dir, NewNonceCache(time.Hour), now); v != agentVerifiedUnknown {
		t.Fatalf("expired signature must be neutral, got %d", v)
	}
	_ = pub
}

func TestVerifyAgentNone(t *testing.T) {
	pub, _, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	if v, _ := verifyAgent("example.com", "GET", "/", "", "", dir, NewNonceCache(time.Hour), time.Now()); v != agentNone {
		t.Fatalf("no signature headers must be agentNone, got %d", v)
	}
}

// PLAN-08 backlog: a Web Bot Auth token must be lifetime-bounded; unbounded or stale is rejected.
func TestVerifyAgentLifetimeBound(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	// created far in the past, no expires → beyond max age → neutral (not trusted).
	old := `("@authority" "@method" "@path");created=1000000000;keyid="` + testKID + `";alg="ed25519";tag="web-bot-auth"`
	sigOld := "sig1=:" + base64.StdEncoding.EncodeToString(ed25519.Sign(priv, []byte(buildSignatureBase("example.com", "GET", "/data", wbaComponents, old)))) + ":"
	if v, _ := verifyAgent("example.com", "GET", "/data", "sig1="+old, sigOld, dir, NewNonceCache(time.Hour), now); v != agentVerifiedUnknown {
		t.Fatalf("an unbounded/stale token must not be trusted, got %d", v)
	}
	// No created and no expires → unbounded → neutral.
	none := `("@authority" "@method" "@path");keyid="` + testKID + `";alg="ed25519";tag="web-bot-auth"`
	sigNone := "sig1=:" + base64.StdEncoding.EncodeToString(ed25519.Sign(priv, []byte(buildSignatureBase("example.com", "GET", "/data", wbaComponents, none)))) + ":"
	if v, _ := verifyAgent("example.com", "GET", "/data", "sig1="+none, sigNone, dir, NewNonceCache(time.Hour), now); v != agentVerifiedUnknown {
		t.Fatalf("a token with no created/expires must not be trusted, got %d", v)
	}
}

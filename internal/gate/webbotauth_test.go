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
// unknown key / expired / absent signature is neutral (never a false deny).

const testKID = "test-agent-key-1"

// signWebBotAuth produces a valid Signature-Input + Signature for host, mirroring the
// verifier's base construction (the two must agree for verification to hold).
func signWebBotAuth(host, kid string, priv ed25519.PrivateKey, created int64) (string, string) {
	inner := `("@authority");created=` + strconv.FormatInt(created, 10) +
		`;keyid="` + kid + `";alg="ed25519";tag="web-bot-auth"`
	sigInput := "sig1=" + inner
	base := buildSignatureBase(host, inner)
	sig := ed25519.Sign(priv, []byte(base))
	return sigInput, "sig1=:" + base64.StdEncoding.EncodeToString(sig) + ":"
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
	now := time.Unix(1_700_000_000, 0)
	si, sig := signWebBotAuth("example.com", testKID, priv, now.Unix())
	if v, kid := verifyAgent("example.com", si, sig, dir, now); v != agentVerifiedTrusted || kid != testKID {
		t.Fatalf("valid signature from allowlisted key: got verdict %d kid %q, want trusted/%s", v, kid, testKID)
	}
}

func TestVerifyAgentForged(t *testing.T) {
	pub, _, _ := ed25519.GenerateKey(nil)         // the ALLOWLISTED key
	_, attacker, _ := ed25519.GenerateKey(nil)    // a DIFFERENT key the attacker holds
	dir := testDir(t, testKID, pub)               // directory trusts pub, not attacker
	now := time.Unix(1_700_000_000, 0)
	// Attacker signs with their own key but CLAIMS the allowlisted keyid.
	si, sig := signWebBotAuth("example.com", testKID, attacker, now.Unix())
	if v, _ := verifyAgent("example.com", si, sig, dir, now); v != agentForged {
		t.Fatalf("forgery of an allowlisted keyid must be agentForged, got %d", v)
	}
}

func TestVerifyAgentTamperedAuthority(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	si, sig := signWebBotAuth("example.com", testKID, priv, now.Unix())
	// Same signature replayed against a DIFFERENT authority must not verify (forgery).
	if v, _ := verifyAgent("victim.com", si, sig, dir, now); v != agentForged {
		t.Fatalf("signature bound to example.com must not verify for victim.com, got %d", v)
	}
}

func TestVerifyAgentUnknownKey(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	// A valid signature but keyid not in the directory → neutral, never a deny.
	si, sig := signWebBotAuth("example.com", "some-other-kid", priv, now.Unix())
	if v, _ := verifyAgent("example.com", si, sig, dir, now); v != agentVerifiedUnknown {
		t.Fatalf("unknown keyid must be neutral (agentVerifiedUnknown), got %d", v)
	}
}

func TestVerifyAgentExpired(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	// created far in the past with an expires also in the past → stale → neutral.
	inner := `("@authority");created=1000000000;expires=1000000100;keyid="` + testKID + `";alg="ed25519";tag="web-bot-auth"`
	base := buildSignatureBase("example.com", inner)
	sig := "sig1=:" + base64.StdEncoding.EncodeToString(ed25519.Sign(priv, []byte(base))) + ":"
	if v, _ := verifyAgent("example.com", "sig1="+inner, sig, dir, now); v != agentVerifiedUnknown {
		t.Fatalf("expired signature must be neutral, got %d", v)
	}
	_ = pub
}

func TestVerifyAgentNone(t *testing.T) {
	pub, _, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	if v, _ := verifyAgent("example.com", "", "", dir, time.Now()); v != agentNone {
		t.Fatalf("no signature headers must be agentNone, got %d", v)
	}
}

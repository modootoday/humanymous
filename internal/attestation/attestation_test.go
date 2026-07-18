package attestation

import "testing"

func TestIssueVerifyRoundTrip(t *testing.T) {
	key := []byte("master")
	tok := Issue(key, "ja4x|10.0.0.0", "nonce-a", 100)
	if !Verify(key, "ja4x|10.0.0.0", "nonce-a", tok, 100) {
		t.Fatal("a freshly issued token must verify in its window")
	}
	// neighbouring windows accepted (issuance-then-solve straddle)
	if !Verify(key, "ja4x|10.0.0.0", "nonce-a", tok, 101) {
		t.Fatal("token must verify one window later")
	}
}

func TestTokenBoundToNonce(t *testing.T) {
	key := []byte("master")
	tok := Issue(key, "fp", "nonce-a", 100)
	// A token for instance A must NOT satisfy a different challenge nonce — so a bot
	// cannot mint once and replay it across many fresh cookies/instances.
	if Verify(key, "fp", "nonce-b", tok, 100) {
		t.Fatal("token must not verify for a different challenge nonce")
	}
}

func TestTokenBoundToFingerprint(t *testing.T) {
	key := []byte("master")
	tok := Issue(key, "fp-alice", "nonce-a", 100)
	if Verify(key, "fp-bob", "nonce-a", tok, 100) {
		t.Fatal("token must not verify for a different fingerprint")
	}
}

func TestStaleTokenRejected(t *testing.T) {
	key := []byte("master")
	tok := Issue(key, "fp", "nonce-a", 100)
	if Verify(key, "fp", "nonce-a", tok, 200) {
		t.Fatal("a token far outside its window must be rejected")
	}
}

func TestEmptyTokenRejected(t *testing.T) {
	if Verify([]byte("master"), "fp", "n", "", 100) {
		t.Fatal("empty token must be rejected")
	}
}

package gate

import (
	"net/http/httptest"
	"testing"
	"time"
)

// WS6: a verdict token is accepted under its issuing epoch and for ONE rotation
// after (grace), but is revoked once the epoch advances twice.
func TestTokenEpochRotation(t *testing.T) {
	key := []byte("server-token-key")
	now := time.Unix(1_000_000, 0)
	em := NewEpochManager()
	r := httptest.NewRequest("GET", "http://p/", nil)
	r.RemoteAddr = "203.0.113.4:1"
	r.Header.Set("User-Agent", "Chrome/126")
	bind := tokenBind(r)

	// Mint under e1.
	tok := issueVerdictToken(key, "sid1", bind, em.Current(), now.Add(time.Hour))
	if verifyVerdictToken(key, tok, bind, "sid1", now, em.Accepted()...) != tokenOK {
		t.Fatal("token should verify under its issuing epoch")
	}

	// One rotation: e1 becomes the accepted predecessor → still valid (grace).
	em.Advance() // now current e2, prev e1
	if verifyVerdictToken(key, tok, bind, "sid1", now, em.Accepted()...) != tokenOK {
		t.Fatal("token should survive one rotation (current+previous grace)")
	}

	// Second rotation: e1 falls out of the accepted window → revoked.
	em.Advance() // now current e3, prev e2
	if verifyVerdictToken(key, tok, bind, "sid1", now, em.Accepted()...) == tokenOK {
		t.Fatal("token must be revoked two rotations after issuance")
	}
	// A fresh token minted under the new epoch verifies.
	tok3 := issueVerdictToken(key, "sid1", bind, em.Current(), now.Add(time.Hour))
	if verifyVerdictToken(key, tok3, bind, "sid1", now, em.Accepted()...) != tokenOK {
		t.Fatal("a token minted under the current epoch must verify")
	}
}

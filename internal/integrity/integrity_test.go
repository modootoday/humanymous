package integrity

import (
	"testing"

	"github.com/modootoday/humanymous/internal/signals"
)

func canonInput(ua string) CanonicalInput {
	return CanonicalInput{
		Headers: map[string]string{
			"user-agent":         ua,
			"sec-ch-ua":          `"Chromium";v="126"`,
			"sec-ch-ua-platform": "Windows",
			"sec-ch-ua-mobile":   "?0",
			"accept-language":    "en-US,en;q=0.9",
			"accept":             "text/html",
		},
		CookieID: "sess-123",
		Body:     []byte(`{"x":1}`),
	}
}

// clientSign mimics the browser: derive seed_n, build canonical, compute token.
func clientSign(mk []byte, sid string, n, tb uint64, in CanonicalInput) string {
	ks := SessionKey(mk, sid)
	seed := Seed(ks, n)
	return ComputeToken(seed, Canonical(in), n+1, tb)
}

func TestRIT_ValidRoundTrip(t *testing.T) {
	mk := []byte("master-secret")
	sid := "sess-123"
	in := canonInput("Mozilla/5.0 Chrome/126")
	tb := TimeBucket(1_000_000, 10)

	token := clientSign(mk, sid, 0, tb, in)

	sig := Verify(VerifyParams{
		SessionKey:  SessionKey(mk, sid),
		LastN:       0,
		PresentedN:  1,
		PresentedTB: tb,
		CurrentTB:   tb,
		Token:       token,
		Canonical:   Canonical(in),
	})
	if sig.ID != "l5.rit.ok" || sig.Verdict != signals.VerdictOK {
		t.Fatalf("valid RIT should be ok, got %s/%s", sig.ID, sig.Verdict)
	}
}

func TestRIT_HeaderTamperDetected(t *testing.T) {
	mk := []byte("master-secret")
	sid := "sess-123"
	in := canonInput("Mozilla/5.0 Chrome/126")
	tb := TimeBucket(1_000_000, 10)
	token := clientSign(mk, sid, 0, tb, in)

	// Attacker rewrites the UA header after the client signed it.
	tampered := canonInput("Mozilla/5.0 Chrome/999 EVIL")

	sig := Verify(VerifyParams{
		SessionKey:  SessionKey(mk, sid),
		LastN:       0,
		PresentedN:  1,
		PresentedTB: tb,
		CurrentTB:   tb,
		Token:       token,
		Canonical:   Canonical(tampered),
	})
	if sig.ID != "l5.rit.header_tampered" {
		t.Fatalf("expected header_tampered, got %s", sig.ID)
	}
}

func TestRIT_AbsentToken(t *testing.T) {
	sig := Verify(VerifyParams{Token: "", PresentedN: 1, LastN: 0})
	if sig.ID != "l5.rit.absent" {
		t.Fatalf("expected absent, got %s", sig.ID)
	}
}

func TestRIT_ReplayOutOfSequence(t *testing.T) {
	mk := []byte("master-secret")
	sid := "sess-123"
	in := canonInput("Mozilla/5.0 Chrome/126")
	tb := TimeBucket(1_000_000, 10)
	token := clientSign(mk, sid, 0, tb, in)
	// Replay with a stale counter (server already advanced to 5).
	sig := Verify(VerifyParams{
		SessionKey: SessionKey(mk, sid), LastN: 5, PresentedN: 1,
		PresentedTB: tb, CurrentTB: tb, Token: token, Canonical: Canonical(in),
	})
	if sig.ID != "l5.rit.stale_replay" {
		t.Fatalf("expected stale_replay, got %s", sig.ID)
	}
}

func TestRIT_FirstRequestGrace(t *testing.T) {
	sig := Verify(VerifyParams{FirstReq: true})
	if sig.ID != "l5.rit.ok" {
		t.Fatalf("bootstrap should be ok, got %s", sig.ID)
	}
}

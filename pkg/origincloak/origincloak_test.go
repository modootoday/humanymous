package origincloak

import (
	"net/http/httptest"
	"testing"
	"time"
)

func TestTokenIsHMACHex(t *testing.T) {
	tok := Token([]byte("secret"), "e42")
	if len(tok) != 64 { // HMAC-SHA256 hex = 32 bytes * 2
		t.Fatalf("token len = %d, want 64 hex chars", len(tok))
	}
	if Token([]byte("secret"), "e42") != tok {
		t.Error("Token must be deterministic")
	}
	if Token([]byte("other"), "e42") == tok {
		t.Error("different key must produce a different token")
	}
	if Token([]byte("secret"), "e43") == tok {
		t.Error("different epoch must produce a different token")
	}
}

func TestEpochBucketsByHour(t *testing.T) {
	base := time.Unix(1_700_000_000/3600*3600, 0) // aligned to a bucket start
	if Epoch(base) != Epoch(base.Add(59*time.Minute)) {
		t.Error("same hour bucket must share an epoch")
	}
	if Epoch(base) == Epoch(base.Add(EpochWindow)) {
		t.Error("next hour must be a new epoch")
	}
}

func TestValidWithGrace(t *testing.T) {
	key := []byte("shared-origin-key")
	now := time.Unix(1_700_003_600, 0)

	// A token minted for the current epoch validates now.
	cur := Token(key, Epoch(now))
	if !Valid(key, cur, now) {
		t.Fatal("current-epoch token must validate")
	}
	// ±1 bucket skew in either direction still validates (the symmetric grace).
	if !Valid(key, cur, now.Add(EpochWindow)) {
		t.Error("token must survive the origin being one bucket behind")
	}
	if !Valid(key, cur, now.Add(-EpochWindow)) {
		t.Error("token must survive the origin being one bucket ahead")
	}
	// Two buckets away is outside the grace → rejected (leaked token expires ~2h).
	if Valid(key, cur, now.Add(2*EpochWindow+time.Minute)) {
		t.Error("a token two buckets stale must be rejected")
	}
	// Wrong key, empty, and garbage are rejected.
	if Valid([]byte("wrong"), cur, now) || Valid(key, "", now) || Valid(key, "deadbeef", now) {
		t.Error("wrong key / empty / garbage must be rejected")
	}
}

// The documented origin-side usage: verify then 421.
func TestOriginHandlerExample(t *testing.T) {
	key := []byte("k")
	now := time.Unix(1_700_007_200, 0)
	good := httptest.NewRequest("GET", "/", nil)
	good.Header.Set(Header, Token(key, Epoch(now)))
	if !Valid(key, good.Header.Get(Header), now) {
		t.Error("a Gate-forwarded request must pass")
	}
	direct := httptest.NewRequest("GET", "/", nil) // no header = direct hit
	if Valid(key, direct.Header.Get(Header), now) {
		t.Error("a direct hit (no header) must fail → 421")
	}
}

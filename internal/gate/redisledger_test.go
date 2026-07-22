package gate

import "testing"

// redisledger_test.go: PLAN-08 backlog — HMAC value authentication. A compromised
// coordinator that can write Redis must not be able to forge a verdict/ban value.
func TestSealOpenValue(t *testing.T) {
	key := []byte("fleet-shared-secret")
	sealed := sealValue(key, `{"v":"ALLOW"}`)

	// Genuine value opens.
	if got, ok := openValue(key, sealed); !ok || got != `{"v":"ALLOW"}` {
		t.Fatalf("genuine sealed value must open, got %q ok=%v", got, ok)
	}
	// A forged value (attacker-written JSON, no valid tag) is rejected.
	if _, ok := openValue(key, `{"v":"ALLOW"}`); ok {
		t.Fatal("an unsigned value must be rejected in signed mode (injection)")
	}
	// A tampered payload under a stale tag is rejected.
	if _, ok := openValue(key, `{"v":"DENY"}`+"\x00"+sealed[len(sealed)-64:]); ok {
		t.Fatal("a tampered payload must be rejected")
	}
	// Unsigned mode (nil key) is a passthrough (backward compatible).
	if got, ok := openValue(nil, `raw`); !ok || got != "raw" {
		t.Fatalf("nil-key mode must passthrough, got %q ok=%v", got, ok)
	}
}

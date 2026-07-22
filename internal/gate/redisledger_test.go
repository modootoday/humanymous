package gate

import "testing"

// redisledger_test.go: PLAN-08 deep-review — key-bound HMAC value authentication. A
// compromised coordinator that can write Redis must not be able to FORGE, RELOCATE, or
// (via the caller's freshness check) ROLLBACK a verdict/ban value.
func TestSealOpenValue(t *testing.T) {
	key := []byte("fleet-shared-secret")
	ctx := "hmn:verdict:sid-A"
	sealed := sealValue(key, ctx, `{"v":"ALLOW"}`)

	// Genuine value opens under its own key.
	if got, ok := openValue(key, ctx, sealed); !ok || got != `{"v":"ALLOW"}` {
		t.Fatalf("genuine sealed value must open, got %q ok=%v", got, ok)
	}
	// RELOCATION: a value sealed for sid-A must NOT verify under sid-B.
	if _, ok := openValue(key, "hmn:verdict:sid-B", sealed); ok {
		t.Fatal("a value relocated to another key must be rejected")
	}
	// A forged value (attacker JSON, no valid tag) is rejected.
	if _, ok := openValue(key, ctx, `{"v":"ALLOW"}`); ok {
		t.Fatal("an unsigned value must be rejected in signed mode (injection)")
	}
	// A tampered payload under a stale tag is rejected.
	if _, ok := openValue(key, ctx, `{"v":"DENY"}`+"\x00"+sealed[len(sealed)-64:]); ok {
		t.Fatal("a tampered payload must be rejected")
	}
	// Unsigned mode (nil key) is a passthrough (backward compatible).
	if got, ok := openValue(nil, ctx, `raw`); !ok || got != "raw" {
		t.Fatalf("nil-key mode must passthrough, got %q ok=%v", got, ok)
	}
}

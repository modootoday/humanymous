package audit

import (
	"testing"
	"time"
)

// WS8: KDF-stretched pseudonyms stay deterministic (links preserved), have the
// same 64-hex shape, and differ from the bare-HMAC value (a different, costlier
// mapping).
func TestPseudonymStretch(t *testing.T) {
	plain := NewVault()
	stretched := NewVault().SetStretch(true)

	// determinism: same subject+value -> same pseudonym.
	a := stretched.Pseudonymize("subjA", "1.2.3.4")
	if stretched.Pseudonymize("subjA", "1.2.3.4") != a {
		t.Fatal("stretched pseudonym must be deterministic (preserves linkage)")
	}
	if len(a) != 64 {
		t.Fatalf("stretched pseudonym should be 64 hex, got %d", len(a))
	}
	// different from the bare-HMAC mapping (harder to invert).
	if plain.Pseudonymize("subjA", "1.2.3.4") == a {
		t.Fatal("stretched value should differ from bare HMAC")
	}
	// shred still severs it.
	stretched.Shred("subjA")
	if stretched.Pseudonymize("subjA", "1.2.3.4") == a {
		t.Fatal("shred must sever the stretched pseudonym too")
	}
}

// WS8: retention tiers classify a record age deterministically.
func TestRetentionTiers(t *testing.T) {
	p := DefaultRetention()
	cases := []struct {
		age  time.Duration
		want string
	}{
		{24 * time.Hour, "HOT"},
		{120 * 24 * time.Hour, "WARM"},
		{2 * 365 * 24 * time.Hour, "COLD"},
		{9 * 365 * 24 * time.Hour, "EXPIRED"},
	}
	for _, c := range cases {
		if got := p.Tier(c.age); got != c.want {
			t.Errorf("age %v: got %q want %q", c.age, got, c.want)
		}
	}
	if d := p.Days(); d["hot"] != 90 {
		t.Fatalf("hot window should be 90 days, got %d", d["hot"])
	}
}

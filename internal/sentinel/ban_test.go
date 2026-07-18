package sentinel

import (
	"testing"
	"time"
)

func fixedBanStore() (*BanStore, *time.Time) {
	now := time.Unix(1_000_000, 0)
	s := NewBanStore(10*time.Second, 3, 5)
	clk := &now
	s.nowFn = func() time.Time { return *clk }
	return s, clk
}

// a hard flood auto-bans the source for 1h on the first strike, escalating on
// repeat strikes to 6h, 24h, then permanent (SoT-27 §2).
func TestAutoBanEscalation(t *testing.T) {
	s, clk := fixedBanStore()
	flood := func() BanEntry {
		var e BanEntry
		for i := 0; i < 8; i++ { // exceed hard=5
			ent, banned, _ := s.Observe("ip:9.9.9.9")
			if banned {
				e = ent
			}
		}
		return e
	}
	e1 := flood()
	if e1.Permanent() || e1.Until.Sub(*clk) != time.Hour {
		t.Fatalf("strike 1 should be 1h, got until=%v perm=%v", e1.Until.Sub(*clk), e1.Permanent())
	}
	// lift + re-flood within decay window -> strike 2 = 6h.
	s.Lift("ip:9.9.9.9")
	*clk = clk.Add(time.Minute)
	e2 := flood()
	if e2.Until.Sub(*clk) != 6*time.Hour {
		t.Fatalf("strike 2 should be 6h, got %v", e2.Until.Sub(*clk))
	}
	s.Lift("ip:9.9.9.9")
	*clk = clk.Add(time.Minute)
	e3 := flood()
	if e3.Until.Sub(*clk) != 24*time.Hour {
		t.Fatalf("strike 3 should be 24h, got %v", e3.Until.Sub(*clk))
	}
	s.Lift("ip:9.9.9.9")
	*clk = clk.Add(time.Minute)
	e4 := flood()
	if !e4.Permanent() {
		t.Fatalf("strike 4 should be permanent, got until=%v", e4.Until)
	}
}

// a temporary ban is enforced until it expires, then clears.
func TestBanExpiry(t *testing.T) {
	s, clk := fixedBanStore()
	s.Add("ip:1.1.1.1", "manual", "op", "INC-1", time.Hour)
	if _, ok := s.Check("ip:1.1.1.1"); !ok {
		t.Fatal("ban should be active")
	}
	*clk = clk.Add(61 * time.Minute)
	if _, ok := s.Check("ip:1.1.1.1"); ok {
		t.Fatal("ban should have expired")
	}
}

// a permanent ban never expires; lift removes it.
func TestPermanentBanAndLift(t *testing.T) {
	s, clk := fixedBanStore()
	s.Add("fp:abc", "coordinated abuse", "op", "INC-2", 0)
	*clk = clk.Add(1000 * time.Hour)
	if _, ok := s.Check("fp:abc"); !ok {
		t.Fatal("permanent ban must persist")
	}
	if !s.Lift("fp:abc") {
		t.Fatal("lift should report removal")
	}
	if _, ok := s.Check("fp:abc"); ok {
		t.Fatal("lifted ban must be gone")
	}
}

// List returns only active bans and sweeps expired ones.
func TestBanList(t *testing.T) {
	s, clk := fixedBanStore()
	s.Add("ip:1", "a", "op", "", time.Hour)
	s.Add("ip:2", "b", "op", "", 0) // permanent
	s.Add("ip:3", "c", "op", "", time.Minute)
	*clk = clk.Add(2 * time.Minute) // ip:3 expires
	got := s.List()
	if len(got) != 2 {
		t.Fatalf("expected 2 active bans, got %d", len(got))
	}
}

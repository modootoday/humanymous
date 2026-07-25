package settings

import (
	"os"
	"path/filepath"
	"testing"
)

func TestValidateRefusesIntegrityOff(t *testing.T) {
	o := &Overlay{
		SchemaVersion: "1.0.0",
		Gates:         map[string]Mode{"gate.smuggle": ModeOff},
	}
	if err := Validate(o); err == nil {
		t.Fatal("expected smuggle off refused")
	}
}

func TestValidateRefusesHROff(t *testing.T) {
	o := &Overlay{HardRules: map[string]Mode{"HR-18": ModeOff}}
	if err := Validate(o); err == nil {
		t.Fatal("expected HR off refused")
	}
}

func TestValidateRefusesNetTCPEnforce(t *testing.T) {
	o := &Overlay{NetPolicy: map[string]Mode{"net.tcp": ModeEnforce}}
	if err := Validate(o); err == nil {
		t.Fatal("expected net.tcp enforce refused")
	}
}

func TestValidateScoringBounds(t *testing.T) {
	ch := 5.0
	o := &Overlay{Scoring: &ScoringPatch{ChallengeAt: &ch}}
	if err := Validate(o); err == nil {
		t.Fatal("challengeAt 5 should fail")
	}
}

func TestResolveEmptyOverlayIdentity(t *testing.T) {
	boot := BootInput{
		HMACKey:       []byte("test-key-32-bytes-long!!!!!!!!"),
		RateWindowSec: 10, RateSoft: 60, RateHard: 120,
		Routes: map[string]string{"/": "balanced"},
	}
	a := Resolve(boot, nil)
	b := Resolve(boot, nil)
	if !a.EmptyOverlay || a.ConfigVersion != b.ConfigVersion {
		t.Fatalf("empty resolve unstable: %+v vs %+v", a, b)
	}
	if a.ChallengeAt != 30 || a.DenyAt != 70 || a.LayerCap != 60 {
		t.Fatalf("defaults wrong: %+v", a)
	}
	if a.HRMode("HR-12") != ModeEnforce {
		t.Fatal("missing HR override must be enforce")
	}
}

func TestResolveOverlayScoringAndKill(t *testing.T) {
	ch := 40.0
	o := &Overlay{
		SchemaVersion: "1.0.0", OverlayID: "ovl_test", Status: "active",
		Scoring:   &ScoringPatch{ChallengeAt: &ch},
		HardRules: map[string]Mode{"HR-12": ModeMonitor},
	}
	if err := Validate(o); err != nil {
		t.Fatal(err)
	}
	boot := BootInput{HMACKey: []byte("k"), RateWindowSec: 10, RateSoft: 60, RateHard: 120}
	eff := Resolve(boot, o)
	if eff.ChallengeAt != 40 || eff.HRMode("HR-12") != ModeMonitor {
		t.Fatalf("overlay not applied: %+v", eff)
	}
	boot.KillSwitch = true
	eff2 := Resolve(boot, o)
	if !eff2.GlobalMonitor {
		t.Fatal("kill switch must force globalMonitor")
	}
}

func TestFileStoreRoundTrip(t *testing.T) {
	dir := t.TempDir()
	st, err := NewFileStore(dir)
	if err != nil {
		t.Fatal(err)
	}
	if st.Active() != nil {
		t.Fatal("expected empty active")
	}
	o := &Overlay{
		SchemaVersion: "1.0.0", OverlayID: "ovl1", Status: "active",
		HardRules: map[string]Mode{"HR-12": ModeMonitor},
	}
	if err := st.SetActive(o); err != nil {
		t.Fatal(err)
	}
	st2, err := NewFileStore(dir)
	if err != nil {
		t.Fatal(err)
	}
	got := st2.Active()
	if got == nil || got.OverlayID != "ovl1" || got.HardRules["HR-12"] != ModeMonitor {
		t.Fatalf("reload failed: %+v", got)
	}
	if err := os.WriteFile(filepath.Join(dir, "settings.overlay.v1.json"), []byte("{not-json"), 0o600); err != nil {
		t.Fatal(err)
	}
	st3, err := NewFileStore(dir)
	if err != nil {
		t.Fatal(err)
	}
	if st3.Active() == nil || st3.Active().OverlayID != "ovl1" {
		t.Fatalf("expected LKG recovery, got %+v err=%v", st3.Active(), st3.LoadError())
	}
}

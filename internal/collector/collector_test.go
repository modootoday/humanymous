package collector

import (
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/signals"
)

func TestStoreLifecycle(t *testing.T) {
	s := NewStore(time.Hour)
	now := time.Now()
	s.Ensure("sid", now)
	if _, ok := s.Get("sid"); !ok {
		t.Fatal("Ensure did not create the session")
	}
	if _, ok := s.Get("missing"); ok {
		t.Fatal("Get returned a non-existent session")
	}
	// RIT counter advances.
	if s.RITCounter("sid") != 0 {
		t.Errorf("new session RIT should be 0")
	}
	if n := s.AdvanceRIT("sid", now); n != 1 {
		t.Errorf("AdvanceRIT = %d, want 1", n)
	}
	// PoW flags.
	if s.PowSolved("sid") {
		t.Error("new session must not be pow-solved")
	}
	s.SetPowSolved("sid", now)
	if !s.PowSolved("sid") {
		t.Error("SetPowSolved not reflected")
	}
}

func TestStoreGCExpiry(t *testing.T) {
	s := NewStore(time.Minute)
	t0 := time.Now()
	s.Ensure("old", t0)
	s.Ensure("fresh", t0.Add(90*time.Second))
	s.GC(t0.Add(2 * time.Minute)) // "old" is >1min stale, "fresh" is 30s stale
	if _, ok := s.Get("old"); ok {
		t.Error("expired session was not GC'd")
	}
	if _, ok := s.Get("fresh"); !ok {
		t.Error("fresh session was wrongly GC'd")
	}
}

// MergeNetwork pins the FIRST observation only; a later one must not replace it
// (plan/02 §2), but AppendNetworkSignals still adds to the pinned report.
func TestMergePinsFirstNetwork(t *testing.T) {
	s := NewStore(time.Hour)
	now := time.Now()
	s.MergeNetwork("sid", signals.NetworkReport{JA4: "first"}, now)
	s.MergeNetwork("sid", signals.NetworkReport{JA4: "second"}, now) // ignored (pinned)
	rep, _ := s.Get("sid")
	if rep.Network.JA4 != "first" {
		t.Errorf("network pin broken: JA4 = %q, want first", rep.Network.JA4)
	}

	s.AppendNetworkSignals("sid", []signals.Signal{{ID: "l5.tls.ja3"}}, now)
	s.AppendNetworkSignals("sid", []signals.Signal{{ID: "l5.rit.absent"}}, now)
	rep, _ = s.Get("sid")
	if len(rep.Network.Signals) != 2 {
		t.Errorf("appended signals = %d, want 2", len(rep.Network.Signals))
	}
	if rep.Network.JA4 != "first" {
		t.Error("AppendNetworkSignals must not replace the pinned network report")
	}
}

// TestMergeNetwork_RequestScopedHopResiduals: after the TLS pin, a later collect
// with multi-hop XFF / proxy Via must still append those hop signals so WireGuard
// exit rotation mid-session is visible to HR-24.
func TestMergeNetwork_RequestScopedHopResiduals(t *testing.T) {
	s := NewStore(time.Hour)
	now := time.Now()
	s.MergeNetwork("sid", signals.NetworkReport{
		JA4: "pinned",
		Signals: []signals.Signal{
			{ID: "l5.tls.grease_absent"},
		},
	}, now)
	s.MergeNetwork("sid", signals.NetworkReport{
		JA4: "ignored",
		Signals: []signals.Signal{
			{ID: "l5.header.xff_multi_hop"},
			{ID: "l5.header.proxy_hop"},
			{ID: "l5.tls.grease_absent"}, // not request-scoped — must NOT re-append
		},
	}, now)
	rep, _ := s.Get("sid")
	if rep.Network.JA4 != "pinned" {
		t.Fatalf("JA4 pin broken: %q", rep.Network.JA4)
	}
	ids := map[string]int{}
	for _, sig := range rep.Network.Signals {
		ids[sig.ID]++
	}
	if ids["l5.header.xff_multi_hop"] != 1 || ids["l5.header.proxy_hop"] != 1 {
		t.Fatalf("want hop residuals once each, got %v", ids)
	}
	if ids["l5.tls.grease_absent"] != 1 {
		t.Fatalf("TLS signal must stay single-pinned, got count %d", ids["l5.tls.grease_absent"])
	}
}

func TestMergeClientAndLabel(t *testing.T) {
	s := NewStore(time.Hour)
	now := time.Now()
	s.MergeClient("sid", signals.ClientReport{UserAgent: "Chrome"}, now)
	s.SetLabel("sid", "bot:selenium", now)
	rep, _ := s.Get("sid")
	if rep.Client.UserAgent != "Chrome" {
		t.Errorf("client UA = %q, want Chrome", rep.Client.UserAgent)
	}
	if rep.Label != "bot:selenium" {
		t.Errorf("label = %q, want bot:selenium", rep.Label)
	}
}

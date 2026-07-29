package mlcorrect

import (
	"testing"

	"github.com/modootoday/humanymous/internal/mlserve"
)

func TestShadowComparator_TracksDivergence(t *testing.T) {
	var c ShadowComparator
	theta := float32(0.5)
	// active always 0.4 (does not fire); shadow always 0.8 (fires) → shadow is more aggressive.
	for i := 0; i < 100; i++ {
		c.observe(0.4, 0.8, false, false, theta)
	}
	s := c.Snapshot()
	if s.N != 100 || s.BothScored != 100 {
		t.Fatalf("expected 100 comparable pairs, got n=%d both=%d", s.N, s.BothScored)
	}
	if s.ShadowFiresMore != 100 || s.ActiveFiresMore != 0 {
		t.Fatalf("shadow must fire more (0.8>θ, 0.4<θ): shadowMore=%d activeMore=%d", s.ShadowFiresMore, s.ActiveFiresMore)
	}
	if d := s.MeanAbsDelta; d < 0.39 || d > 0.41 {
		t.Fatalf("mean |Δ| should be ~0.4, got %.3f", d)
	}
	if s.FireAgreement != 0 {
		t.Fatalf("the models never agree here, agreement should be 0, got %.3f", s.FireAgreement)
	}
}

func TestShadowComparator_AgreementAndAbstain(t *testing.T) {
	var c ShadowComparator
	theta := float32(0.5)
	// agreeing pairs (both fire)
	for i := 0; i < 60; i++ {
		c.observe(0.9, 0.85, false, false, theta)
	}
	// candidate abstains where active scored
	for i := 0; i < 40; i++ {
		c.observe(0.9, 0, false, true, theta)
	}
	s := c.Snapshot()
	if s.BothScored != 60 {
		t.Fatalf("only 60 pairs are comparable, got %d", s.BothScored)
	}
	if s.ShadowAbstained != 40 {
		t.Fatalf("candidate abstained on 40, got %d", s.ShadowAbstained)
	}
	if s.FireAgreement != 1 {
		t.Fatalf("all comparable pairs fire together, agreement should be 1, got %.3f", s.FireAgreement)
	}
}

func TestController_ObserveShadow_FeedsSnapshot(t *testing.T) {
	c := NewController(0.01, 0.5, 0.02)
	for i := 0; i < 50; i++ {
		c.ObserveShadow(mlserve.Prediction{PBot: 0.3}, mlserve.Prediction{PBot: 0.9})
	}
	sh := c.Snapshot().Shadow
	if sh.N != 50 || sh.ShadowFiresMore != 50 {
		t.Fatalf("controller must route shadow observations to the comparator: %+v", sh)
	}
}

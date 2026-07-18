package signals

import "testing"

// The registry is the single source of truth for signal weights (SoT-00 §4).
// These invariants guard against a mis-registered signal: the id's layer digit
// must match its Layer, weights stay in [0,100], and no id is registered twice
// (def() would have panic'd at init, so a loaded package already proves that).
func TestRegistryIntegrity(t *testing.T) {
	defs := All()
	if len(defs) < 50 {
		t.Fatalf("registry suspiciously small: %d definitions", len(defs))
	}
	seen := map[string]bool{}
	for _, d := range defs {
		if seen[d.ID] {
			t.Errorf("duplicate id survived: %s", d.ID)
		}
		seen[d.ID] = true
		// id looks like l{n}.{group}.{item}; its layer digit must match d.Layer.
		if len(d.ID) < 2 || d.ID[0] != 'l' {
			t.Errorf("id %q is not l{n}.… form", d.ID)
			continue
		}
		if len(d.Layer) != 2 || d.Layer[0] != 'L' || d.Layer[1] != d.ID[1] {
			t.Errorf("id %q layer digit does not match Layer %q", d.ID, d.Layer)
		}
		if d.Weight < 0 || d.Weight > 100 {
			t.Errorf("id %q weight %.0f out of [0,100]", d.ID, d.Weight)
		}
	}
}

// New applies the registered weight and computes the score; an unknown id is
// UNKNOWN-safe (weight 0), never a crash.
func TestNewAppliesRegistryWeightAndScore(t *testing.T) {
	s := New("l1.navigator.webdriver", true, VerdictBot, 1.0, SourceWASM, "webdriver true")
	if s.Weight != 15 {
		t.Errorf("weight = %.0f, want 15 (from registry)", s.Weight)
	}
	if s.Score != 15 { // weight 15 * severity(BOT)=1 * confidence 1
		t.Errorf("score = %.1f, want 15", s.Score)
	}
	if s.Layer != LayerStatic {
		t.Errorf("layer = %q, want L1", s.Layer)
	}
	unknown := New("l9.made.up", nil, VerdictBot, 1.0, SourceServer, "")
	if unknown.Weight != 0 || unknown.Score != 0 {
		t.Errorf("unknown id must be weight/score 0, got w=%.0f s=%.1f", unknown.Weight, unknown.Score)
	}
}

func TestSeverityAndComputeScoreClamp(t *testing.T) {
	if Severity(VerdictBot) != 1.0 || Severity(VerdictSuspicious) != 0.5 ||
		Severity(VerdictOK) != 0 || Severity(VerdictUnknown) != 0 {
		t.Fatal("severity mapping wrong")
	}
	// SUSPICIOUS halves the weight.
	if got := ComputeScore(40, 1.0, VerdictSuspicious); got != 20 {
		t.Errorf("suspicious 40 -> %.1f, want 20", got)
	}
	// confidence clamps to [0,1]; score never exceeds weight.
	if got := ComputeScore(30, 5.0, VerdictBot); got != 30 {
		t.Errorf("over-confidence must clamp to weight 30, got %.1f", got)
	}
	if got := ComputeScore(30, -1, VerdictBot); got != 0 {
		t.Errorf("negative confidence -> 0, got %.1f", got)
	}
	// OK contributes nothing regardless of weight/confidence.
	if got := ComputeScore(99, 1.0, VerdictOK); got != 0 {
		t.Errorf("OK verdict must score 0, got %.1f", got)
	}
}

func TestGroupOf(t *testing.T) {
	if GroupOf("l1.navigator.webdriver") != "webdriver" {
		t.Errorf("group = %q, want webdriver", GroupOf("l1.navigator.webdriver"))
	}
	if GroupOf("l9.unknown") != "" {
		t.Error("unknown id group must be empty")
	}
}

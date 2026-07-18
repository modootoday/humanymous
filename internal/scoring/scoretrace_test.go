package scoring

import (
	"testing"

	"github.com/modootoday/humanymous/internal/signals"
)

// combineTrace must return exactly the same score as the production Combine over
// the same contributions — otherwise the observatory would mis-teach the math.
func TestCombineTraceMatchesCombine(t *testing.T) {
	p := DefaultPolicy()
	sets := [][]scored{
		{},
		{{id: "a", layer: "L1", group: "", score: 40}},
		{ // two tells in one group -> dedup keeps the 40, drops the 25
			{id: "l1.navigator.plugins_empty", layer: "L1", group: "plugins", score: 25},
			{id: "l1.navigator.pdfviewer", layer: "L1", group: "plugins", score: 40},
			{id: "l2.canvas.hash", layer: "L2", group: "canvas", score: 55},
		},
		{ // a saturating layer to exercise the LayerCap clamp
			{id: "x1", layer: "L1", group: "", score: 55},
			{id: "x2", layer: "L1", group: "", score: 55},
			{id: "x3", layer: "L5", group: "", score: 30},
		},
	}
	for i, in := range sets {
		want := Combine(in, p)
		got, layers, _ := combineTrace(in, p)
		if got != want {
			t.Fatalf("set %d: combineTrace=%v Combine=%v", i, got, want)
		}
		// LayerCap clamp must be recorded where a layer exceeded the cap.
		for _, l := range layers {
			if l.RawProb > clamp01(p.LayerCap/100)+1e-12 && !l.CapHit {
				t.Fatalf("set %d layer %s exceeded cap but CapHit=false", i, l.Layer)
			}
		}
	}
}

func botReport() signals.SessionReport {
	return signals.SessionReport{
		Client: signals.ClientReport{
			UserAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",
			Signals: []signals.Signal{
				{ID: "l1.navigator.webdriver", Layer: "L1", Verdict: signals.VerdictBot, Weight: 40, Score: 40, Confidence: 0.95, Collected: "js"},
				{ID: "l1.cdp.proxy_leak", Layer: "L1", Verdict: signals.VerdictBot, Weight: 40, Score: 40, Confidence: 0.95, Collected: "wasm"},
			},
		},
	}
}

// ScoreWithTrace must yield the identical ScoreResult as Score, and the trace's
// winning hard rule must equal the ScoreResult's HardRuleFired.
func TestScoreWithTraceMatchesScore(t *testing.T) {
	e := NewEngine()

	a := botReport()
	b := botReport()
	want := e.Score(&a)
	got, trace := e.ScoreWithTrace(&b)

	if got.RiskScore != want.RiskScore || got.Verdict != want.Verdict || got.HardRuleFired != want.HardRuleFired {
		t.Fatalf("ScoreWithTrace diverged from Score:\n got=%+v\nwant=%+v", got, want)
	}
	if trace.Score != want.RiskScore {
		t.Fatalf("trace.Score=%v != result.RiskScore=%v", trace.Score, want.RiskScore)
	}
	// The CDP bot must DENY via HR-9, and the trace must mark HR-9 as the winner.
	if want.Verdict != "DENY" || want.HardRuleFired != "HR-9" {
		t.Fatalf("expected DENY via HR-9, got %s via %q", want.Verdict, want.HardRuleFired)
	}
	var winner string
	winners := 0
	for _, hr := range trace.HardRules {
		if hr.Won {
			winner = hr.Rule
			winners++
		}
	}
	if winners != 1 || winner != "HR-9" {
		t.Fatalf("trace winner: got %d winners, winner=%q", winners, winner)
	}
	// Every rule before HR-9 must be recorded as not-won.
	if len(trace.HardRules) != len(promotionRules) {
		t.Fatalf("trace evaluated %d rules, want %d", len(trace.HardRules), len(promotionRules))
	}
}

// The traced rule winner always equals the production applyPromotionRules result.
func TestTraceWinnerEqualsApply(t *testing.T) {
	cases := []ruleContext{
		{}, // no client -> HR-10
		{hasClient: true, sigs: []signals.Signal{
			{ID: "l1.artifact.selenium", Verdict: signals.VerdictBot},
		}},
		{hasClient: true, sigs: []signals.Signal{
			{ID: "l4.event.no_interaction", Verdict: signals.VerdictSuspicious},
		}},
	}
	for i, c := range cases {
		hv := applyPromotionRules(c)
		evals := tracePromotionRules(c)
		var winner string
		for _, e := range evals {
			if e.Won {
				winner = e.Rule
			}
		}
		if winner != hv.rule {
			t.Fatalf("case %d: trace winner %q != applyPromotionRules %q", i, winner, hv.rule)
		}
	}
}

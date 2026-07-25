package scoring

import "github.com/modootoday/humanymous/internal/signals"

// scoretrace.go produces an additive, playground-only ScoreTrace (SoT-30 §6): it
// exposes the per-layer noisy-OR subtotals, LayerCap clamps, correlated-group
// dedup drops, and the ordered hard-rule evaluation that the opaque Score() pass
// hides. It REUSES the real dedup/noisy-OR math and the shared promotionRules
// table, so it can never diverge from the production verdict (guarded by a
// golden test asserting trace.Score == Combine). The production Score() path is
// untouched.

// ScoreTrace decomposes how a session reached its verdict.
type ScoreTrace struct {
	Contributions []ContribTrace `json:"contributions"`
	DedupDropped  []DedupTrace   `json:"dedupDropped"`
	PerLayer      []LayerTrace   `json:"perLayer"`
	HardRules     []HardRuleEval `json:"hardRuleEval"`
	Score         float64        `json:"score"`
	Verdict       string         `json:"verdict"`
	HardRuleFired string         `json:"hardRuleFired"`
	Policy        PolicyView     `json:"policy"`
}

// ContribTrace is one scored contribution as it entered the combination.
type ContribTrace struct {
	ID    string  `json:"id"`
	Layer string  `json:"layer"`
	Group string  `json:"group"`
	Score float64 `json:"score"`
}

// DedupTrace records a signal dropped because a higher-scoring signal in the
// same correlation group was kept.
type DedupTrace struct {
	Layer        string  `json:"layer"`
	Group        string  `json:"group"`
	KeptID       string  `json:"keptId"`
	DroppedID    string  `json:"droppedId"`
	KeptScore    float64 `json:"keptScore"`
	DroppedScore float64 `json:"droppedScore"`
}

// LayerTrace records one layer's noisy-OR subtotal and whether LayerCap clamped it.
type LayerTrace struct {
	Layer      string  `json:"layer"`
	RawProb    float64 `json:"rawProb"`
	CappedProb float64 `json:"cappedProb"`
	CapHit     bool    `json:"capHit"`
}

// HardRuleEval records one promotion rule's evaluation. Won marks the first
// matching rule (the one that actually set the override).
type HardRuleEval struct {
	Rule    string `json:"rule"`
	Verdict string `json:"verdict"`
	Why     string `json:"why"`
	Matched bool   `json:"matched"`
	Won     bool   `json:"won"`
}

// PolicyView is the JSON-friendly policy snapshot.
type PolicyView struct {
	Version     string  `json:"version"`
	LayerCap    float64 `json:"layerCap"`
	ChallengeAt float64 `json:"challengeAt"`
	DenyAt      float64 `json:"denyAt"`
}

// combineTrace mirrors Combine (dedupGroups -> capByLayer -> 100*noisyOR) while
// recording the intermediates. It reuses dedupGroups' keep-rule and noisyOR, so
// its returned score equals Combine(in, p) exactly (asserted by a golden test).
func combineTrace(in []scored, p Policy) (score float64, layers []LayerTrace, drops []DedupTrace) {
	kept, drops := dedupTraced(in)
	probs, layers := layerTrace(kept, p.LayerCap)
	return 100 * noisyOR(probs), layers, drops
}

// dedupTraced keeps the highest-scoring signal per (layer/group) — identical to
// dedupGroups — and records every dropped group member.
func dedupTraced(in []scored) (kept []scored, drops []DedupTrace) {
	best := map[string]scored{}
	members := map[string][]scored{}
	var loose []scored
	var order []string
	for _, s := range in {
		if s.group == "" {
			loose = append(loose, s)
			continue
		}
		key := string(s.layer) + "/" + s.group
		if _, seen := members[key]; !seen {
			order = append(order, key)
		}
		members[key] = append(members[key], s)
		if cur, ok := best[key]; !ok || s.score > cur.score {
			best[key] = s
		}
	}
	for _, key := range order {
		b := best[key]
		kept = append(kept, b)
		for _, m := range members[key] {
			if m.id != b.id {
				drops = append(drops, DedupTrace{
					Layer: string(b.layer), Group: b.group,
					KeptID: b.id, DroppedID: m.id, KeptScore: b.score, DroppedScore: m.score,
				})
			}
		}
	}
	return append(kept, loose...), drops
}

// layerTrace mirrors capByLayer (within-layer noisy-OR, then clamp to LayerCap)
// while recording each layer's raw vs capped probability.
func layerTrace(kept []scored, cap float64) (probs []float64, traces []LayerTrace) {
	byLayer := map[signals.Layer]float64{}
	var order []signals.Layer
	for _, s := range kept {
		if _, seen := byLayer[s.layer]; !seen {
			order = append(order, s.layer)
		}
		pcur := byLayer[s.layer]
		p := clamp01(s.score / 100)
		byLayer[s.layer] = pcur + p - pcur*p
	}
	capP := clamp01(cap / 100)
	for _, layer := range order {
		raw := byLayer[layer]
		capped, hit := raw, false
		if raw > capP {
			capped, hit = capP, true
		}
		probs = append(probs, capped)
		traces = append(traces, LayerTrace{Layer: string(layer), RawProb: raw, CappedProb: capped, CapHit: hit})
	}
	return
}

// tracePromotionRules evaluates every promotion rule (matched AND unmatched) in
// order. Winner = first enforce-mode match (SoT-39 continue semantics for monitor).
// Same table + mode rules as applyPromotionRules.
func tracePromotionRules(c ruleContext) []HardRuleEval {
	out := make([]HardRuleEval, 0, len(promotionRules))
	won := false
	for _, r := range promotionRules {
		matched := r.pred(c)
		mode := ruleModeOf(c.ruleModes, r.id)
		e := HardRuleEval{Rule: r.id, Verdict: r.verdict, Why: r.why, Matched: matched}
		if matched && mode == "enforce" && !won {
			e.Won = true
			won = true
		}
		// monitor match: Matched=true, Won=false (shadow only)
		out = append(out, e)
	}
	return out
}

// contribTraces converts the internal scored set to the exported trace shape.
func contribTraces(in []scored) []ContribTrace {
	out := make([]ContribTrace, 0, len(in))
	for _, s := range in {
		if s.score <= 0 {
			continue
		}
		out = append(out, ContribTrace{ID: s.id, Layer: string(s.layer), Group: s.group, Score: s.score})
	}
	return out
}

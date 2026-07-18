package scoring

import (
	"sort"

	"github.com/modootoday/humanymous/internal/signals"
)

// combine.go turns a set of scored signals into a single 0..100 risk value
// using a saturating (noisy-OR) combination with per-layer caps and
// correlated-group de-duplication (SoT-05 §2).

// scored is one contribution to the combination.
type scored struct {
	id    string
	layer signals.Layer
	group string
	score float64 // 0..100 (already weight*severity*confidence)
}

// dedupGroups keeps only the highest-scoring signal per correlation group,
// so several tells derived from the same root cause aren't double counted.
// Signals with an empty group are all kept (each is its own group).
func dedupGroups(in []scored) []scored {
	best := map[string]scored{}
	var loose []scored
	for _, s := range in {
		if s.group == "" {
			loose = append(loose, s)
			continue
		}
		key := string(s.layer) + "/" + s.group
		if cur, ok := best[key]; !ok || s.score > cur.score {
			best[key] = s
		}
	}
	out := make([]scored, 0, len(best)+len(loose))
	for _, s := range best {
		out = append(out, s)
	}
	return append(out, loose...)
}

// capByLayer clamps the summed contribution of each layer to policy.LayerCap.
// It returns per-layer probabilities suitable for the noisy-OR product.
func capByLayer(in []scored, cap float64) []float64 {
	// Group probabilities by layer, noisy-OR within a layer, then cap.
	byLayer := map[signals.Layer]float64{}
	for _, s := range in {
		p := clamp01(s.score / 100)
		cur := byLayer[s.layer] // 0 default
		// noisy-OR accumulate within layer
		byLayer[s.layer] = cur + p - cur*p
	}
	out := make([]float64, 0, len(byLayer))
	capP := clamp01(cap / 100)
	for _, p := range byLayer {
		if p > capP {
			p = capP
		}
		out = append(out, p)
	}
	return out
}

// noisyOR combines independent probabilities: 1 - Π(1-p_i).
func noisyOR(ps []float64) float64 {
	prod := 1.0
	for _, p := range ps {
		prod *= (1 - clamp01(p))
	}
	return 1 - prod
}

// Combine computes the final 0..100 risk score from scored contributions.
func Combine(in []scored, p Policy) float64 {
	deduped := dedupGroups(in)
	layerProbs := capByLayer(deduped, p.LayerCap)
	return 100 * noisyOR(layerProbs)
}

// topContributors returns the n highest individual contributions (SoT-05 §6).
func topContributors(in []scored, n int) []signals.Contributor {
	cp := append([]scored(nil), in...)
	sort.SliceStable(cp, func(i, j int) bool { return cp[i].score > cp[j].score })
	if n > len(cp) {
		n = len(cp)
	}
	out := make([]signals.Contributor, 0, n)
	for _, s := range cp[:n] {
		if s.score <= 0 {
			break
		}
		out = append(out, signals.Contributor{ID: s.id, Score: s.score})
	}
	return out
}

func clamp01(f float64) float64 {
	if f < 0 {
		return 0
	}
	if f > 1 {
		return 1
	}
	return f
}

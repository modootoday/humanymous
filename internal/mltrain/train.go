// Package mltrain is the shared, pure-Go training kernel for the behavioral model (SoT-42 Pillar A).
// Both cmd/ml-train (bootstrap) and cmd/mlcorrect-train (self-correcting retrain) build on it, so
// the training math lives in ONE place — no forked trainer. It has no ML-framework dependency: a
// one-hidden-layer MLP trained by mini-batch SGD on BCE, plus the evaluation primitives the
// self-correcting loop's promotion gates need (per-cohort false-positive rate, TESSERACT
// time-ordered split, and an Area-Under-Time metric).
package mltrain

import (
	"math"
	"math/rand"
	"sort"

	"github.com/modootoday/humanymous/internal/behavior"
	"github.com/modootoday/humanymous/internal/mlserve"
)

// Sample is one labeled behavioral observation. TS is an optional event time (unix seconds) used for
// the TESSERACT split; Cohort is an optional accessibility-cohort id used for the per-cohort human
// false-positive gate. Zero values are valid (TS=0 sorts earliest, Cohort="" ⇒ "default").
type Sample struct {
	X      behavior.FeatureVector
	Y      float32 // 1 = bot, 0 = human
	Human  bool
	TS     int64
	Cohort string
}

// Config controls training.
type Config struct {
	Hidden  int
	Epochs  int
	LR      float32
	Seed    int64
	Version string
}

// Standardize computes per-feature mean/std over the training set (std floored to avoid div-by-zero).
func Standardize(s []Sample) (mean, std []float32) {
	d := behavior.FeatureDim
	mean = make([]float32, d)
	std = make([]float32, d)
	if len(s) == 0 {
		for i := range std {
			std[i] = 1
		}
		return
	}
	for _, sm := range s {
		for i := 0; i < d; i++ {
			mean[i] += sm.X[i]
		}
	}
	n := float32(len(s))
	for i := range mean {
		mean[i] /= n
	}
	for _, sm := range s {
		for i := 0; i < d; i++ {
			dd := sm.X[i] - mean[i]
			std[i] += dd * dd
		}
	}
	for i := range std {
		std[i] = float32(math.Sqrt(float64(std[i]/n))) + 1e-6
	}
	return
}

// Train fits an MLP. When base is non-nil and shape-compatible (same hidden width and feature dim),
// weights are WARM-STARTED from it instead of He-initialized — the incremental-update path for the
// self-correcting loop. Warm-start alone does not prevent forgetting; the caller is expected to keep
// the frozen gold anchor in `train` so old knowledge is re-taught each retrain. (For this compact
// MLP that warm-start IS the "adapter"; a low-rank LoRA adapter only becomes meaningful once the base
// grows into a larger sequence model behind the same mlserve seam.)
func Train(cfg Config, train []Sample, mean, std []float32, base *mlserve.MLPBundle) mlserve.MLPBundle {
	rng := rand.New(rand.NewSource(cfg.Seed))
	d := behavior.FeatureDim
	hidden := cfg.Hidden
	if hidden <= 0 {
		hidden = 16
	}

	w1 := make([][]float32, hidden)
	b1 := make([]float32, hidden)
	w2 := make([]float32, hidden)
	var b2 float32
	warm := base != nil && base.Hidden == hidden && base.FeatureDim == d &&
		len(base.W1) == hidden && len(base.W2) == hidden && len(base.B1) == hidden
	if warm {
		for h := 0; h < hidden; h++ {
			w1[h] = append([]float32(nil), base.W1[h]...)
		}
		copy(b1, base.B1)
		copy(w2, base.W2)
		b2 = base.B2
	} else {
		for h := range w1 {
			w1[h] = make([]float32, d)
			scale := float32(math.Sqrt(2.0 / float64(d)))
			for i := range w1[h] {
				w1[h][i] = float32(rng.NormFloat64()) * scale
			}
			w2[h] = float32(rng.NormFloat64()) * float32(math.Sqrt(2.0/float64(hidden)))
		}
	}

	norm := func(x behavior.FeatureVector) []float32 {
		z := make([]float32, d)
		for i := range x {
			z[i] = (x[i] - mean[i]) / std[i]
		}
		return z
	}
	lr := cfg.LR
	if lr <= 0 {
		lr = 0.05
	}
	for ep := 0; ep < cfg.Epochs; ep++ {
		rng.Shuffle(len(train), func(i, j int) { train[i], train[j] = train[j], train[i] })
		for _, sm := range train {
			z := norm(sm.X)
			hpre := make([]float32, hidden)
			hact := make([]float32, hidden)
			logit := b2
			for h := 0; h < hidden; h++ {
				acc := b1[h]
				for i := 0; i < d; i++ {
					acc += w1[h][i] * z[i]
				}
				hpre[h] = acc
				if acc > 0 {
					hact[h] = acc
				}
				logit += w2[h] * hact[h]
			}
			p := Sigmoid(logit)
			dLogit := p - sm.Y // BCE gradient
			for h := 0; h < hidden; h++ {
				gW2 := dLogit * hact[h]
				dHact := dLogit * w2[h]
				var dHpre float32
				if hpre[h] > 0 {
					dHpre = dHact
				}
				for i := 0; i < d; i++ {
					w1[h][i] -= lr * dHpre * z[i]
				}
				b1[h] -= lr * dHpre
				w2[h] -= lr * gW2
			}
			b2 -= lr * dLogit
		}
	}
	return mlserve.MLPBundle{
		Version: cfg.Version, SchemaHash: behavior.SchemaHash(), FeatureDim: d, Hidden: hidden,
		Mean: mean, Std: std, W1: w1, B1: b1, W2: w2, B2: b2, CalA: 1, CalB: 0,
	}
}

// Sigmoid is the logistic function.
func Sigmoid(x float32) float32 { return float32(1 / (1 + math.Exp(float64(-x)))) }

// Forward runs the bundle's forward pass (mirrors mlserve.MLP.Predict) for in-process evaluation.
func Forward(b mlserve.MLPBundle, x behavior.FeatureVector) float32 {
	logit := b.B2
	for h := 0; h < b.Hidden; h++ {
		acc := b.B1[h]
		for i := 0; i < b.FeatureDim; i++ {
			s := b.Std[i]
			if s == 0 {
				s = 1
			}
			acc += b.W1[h][i] * ((x[i] - b.Mean[i]) / s)
		}
		if acc < 0 {
			acc = 0
		}
		logit += b.W2[h] * acc
	}
	return Sigmoid(b.CalA*logit + b.CalB)
}

// Evaluate returns overall accuracy, aggregate human FPR, and bot TPR at threshold 0.5.
func Evaluate(b mlserve.MLPBundle, val []Sample) (acc, humanFPR, botTPR float32) {
	var correct, humans, humanFP, bots, botTP int
	for _, s := range val {
		pred := 0
		if Forward(b, s.X) >= 0.5 {
			pred = 1
		}
		if float32(pred) == s.Y {
			correct++
		}
		if s.Human {
			humans++
			if pred == 1 {
				humanFP++
			}
		} else {
			bots++
			if pred == 1 {
				botTP++
			}
		}
	}
	if len(val) > 0 {
		acc = float32(correct) / float32(len(val))
	}
	if humans > 0 {
		humanFPR = float32(humanFP) / float32(humans)
	}
	if bots > 0 {
		botTPR = float32(botTP) / float32(bots)
	}
	return
}

// CohortStat is a per-cohort human false-positive tally.
type CohortStat struct {
	Humans int
	FP     int
	FPR    float32
}

// CohortFPR groups human samples by Cohort and returns each cohort's false-positive rate. This is
// what the promotion gate checks — human-FP must be ~0 PER COHORT, never merely in aggregate, so a
// model cannot be admitted by acing the majority while regressing an accessibility minority.
func CohortFPR(b mlserve.MLPBundle, val []Sample) map[string]CohortStat {
	out := map[string]CohortStat{}
	for _, s := range val {
		if !s.Human {
			continue
		}
		c := s.Cohort
		if c == "" {
			c = "default"
		}
		st := out[c]
		st.Humans++
		if Forward(b, s.X) >= 0.5 {
			st.FP++
		}
		out[c] = st
	}
	for c, st := range out {
		if st.Humans > 0 {
			st.FPR = float32(st.FP) / float32(st.Humans)
		}
		out[c] = st
	}
	return out
}

// TesseractSplit orders samples by time (TS ascending, stable for equal/zero TS) and holds out the
// most-recent testFrac as the test set — training on the past, evaluating on the future, per
// TESSERACT (Pendlebury et al., USENIX Sec'19). This exposes temporal decay that a random split
// hides. Frozen-anchor samples (TS=0) sort earliest and land in train.
func TesseractSplit(samples []Sample, testFrac float64) (train, test []Sample) {
	if testFrac <= 0 || testFrac >= 1 || len(samples) < 2 {
		return samples, nil
	}
	ordered := append([]Sample(nil), samples...)
	sort.SliceStable(ordered, func(i, j int) bool { return ordered[i].TS < ordered[j].TS })
	cut := len(ordered) - int(float64(len(ordered))*testFrac)
	if cut < 1 {
		cut = 1
	}
	return ordered[:cut], ordered[cut:]
}

// AUT is the Area-Under-Time of accuracy: the test window is split into `buckets` equal-count,
// time-ordered slices and their accuracies averaged. A model that decays across the test window
// scores below one that holds steady, which raw end-of-window accuracy would miss. Returns 0 for an
// empty test set.
func AUT(b mlserve.MLPBundle, test []Sample, buckets int) float32 {
	if len(test) == 0 {
		return 0
	}
	if buckets < 1 {
		buckets = 1
	}
	ordered := append([]Sample(nil), test...)
	sort.SliceStable(ordered, func(i, j int) bool { return ordered[i].TS < ordered[j].TS })
	var sum float32
	var used int
	n := len(ordered)
	for k := 0; k < buckets; k++ {
		lo := k * n / buckets
		hi := (k + 1) * n / buckets
		if lo >= hi {
			continue
		}
		acc, _, _ := Evaluate(b, ordered[lo:hi])
		sum += acc
		used++
	}
	if used == 0 {
		return 0
	}
	return sum / float32(used)
}

// Command ml-train trains the small policy-specific behavioral MLP (SoT-42 Pillar A) into a JSON
// bundle that internal/mlserve loads at the edge. Pure Go, no ML framework — the whole pipeline
// (feature transform → train → serve) runs in Docker with the Go toolchain only.
//
//	ml-train -gen 20000 -out model.json          # bootstrap on synthetic grounded data
//	ml-train -data labeled.jsonl -out model.json # real labeled traces (JSONL {label, behavior})
//
// The synthetic generator is a BOOTSTRAP: it draws human and bot behavioral summaries from
// distributions grounded in the red-team catalog's known bot values and the scoring test's human
// values, so the pipeline is exercisable and the model separates the obvious cases TODAY. It is NOT
// a substitute for real trace collection — that is the data-quality follow-up the design flags. Use
// -data for production training.
package main

import (
	"bufio"
	"encoding/json"
	"flag"
	"fmt"
	"math"
	"math/rand"
	"os"
	"time"

	"github.com/modootoday/humanymous/internal/behavior"
	"github.com/modootoday/humanymous/internal/mlserve"
	"github.com/modootoday/humanymous/internal/signals"
)

type labeled struct {
	Label    int                       `json:"label"` // 1 = bot, 0 = human
	Behavior signals.BehaviorSummary   `json:"behavior"`
}

type sample struct {
	x     behavior.FeatureVector
	y     float32
	human bool
}

func main() {
	gen := flag.Int("gen", 0, "generate N synthetic bootstrap samples instead of reading -data")
	data := flag.String("data", "", "path to labeled JSONL {label:0|1, behavior:{...}} (real traces)")
	out := flag.String("out", "model.json", "output bundle path")
	hidden := flag.Int("hidden", 16, "hidden units")
	epochs := flag.Int("epochs", 60, "training epochs")
	lr := flag.Float64("lr", 0.05, "learning rate")
	seed := flag.Int64("seed", 1, "rng seed")
	flag.Parse()

	rng := rand.New(rand.NewSource(*seed))
	var samples []sample
	if *data != "" {
		s, err := readData(*data)
		if err != nil {
			fatal(err)
		}
		samples = s
		fmt.Fprintf(os.Stderr, "ml-train: loaded %d labeled samples from %s\n", len(samples), *data)
	} else {
		n := *gen
		if n <= 0 {
			n = 20000
		}
		samples = genSynthetic(rng, n)
		fmt.Fprintf(os.Stderr, "ml-train: generated %d SYNTHETIC bootstrap samples (not production data)\n", len(samples))
	}
	if len(samples) < 100 {
		fatal(fmt.Errorf("need >=100 samples, got %d", len(samples)))
	}

	// train/val split
	rng.Shuffle(len(samples), func(i, j int) { samples[i], samples[j] = samples[j], samples[i] })
	nVal := len(samples) / 5
	val, train := samples[:nVal], samples[nVal:]

	mean, std := standardize(train)
	b := trainMLP(rng, train, mean, std, *hidden, *epochs, float32(*lr))

	// eval
	m := &mlpForward{b: b}
	acc, humanFPR, botTPR := evaluate(m, val)
	fmt.Fprintf(os.Stderr, "ml-train: val acc=%.3f  bot TPR=%.3f  human FPR=%.3f  (bundle %s, schema %s)\n",
		acc, botTPR, humanFPR, b.Version, b.SchemaHash)

	raw, _ := json.MarshalIndent(b, "", " ")
	if err := os.WriteFile(*out, raw, 0o644); err != nil {
		fatal(err)
	}
	fmt.Fprintf(os.Stderr, "ml-train: wrote %s (%d bytes)\n", *out, len(raw))
}

// --- data ---

func readData(path string) ([]sample, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	var out []sample
	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 1<<20), 1<<20)
	for sc.Scan() {
		line := sc.Bytes()
		if len(line) == 0 {
			continue
		}
		var l labeled
		if err := json.Unmarshal(line, &l); err != nil {
			return nil, fmt.Errorf("bad JSONL line: %w", err)
		}
		out = append(out, sample{x: behavior.Extract(l.Behavior), y: float32(l.Label), human: l.Label == 0})
	}
	return out, sc.Err()
}

// --- synthetic bootstrap (grounded in catalog bot values + test human values) ---

func genSynthetic(rng *rand.Rand, n int) []sample {
	out := make([]sample, 0, n)
	for i := 0; i < n; i++ {
		if i%2 == 0 {
			out = append(out, sample{x: behavior.Extract(humanBehavior(rng)), y: 0, human: true})
		} else {
			out = append(out, sample{x: behavior.Extract(botBehavior(rng)), y: 1, human: false})
		}
	}
	return out
}

func nz(rng *rand.Rand, mean, sd float64) float64 { // non-negative gaussian
	v := mean + rng.NormFloat64()*sd
	if v < 0 {
		return 0
	}
	return v
}

func humanBehavior(rng *rand.Rand) signals.BehaviorSummary {
	return signals.BehaviorSummary{
		Mouse: signals.MouseFeatures{
			Samples: int(nz(rng, 55, 18)) + 8, MeanCurvature: nz(rng, 0.35, 0.12), VelocityStdDev: nz(rng, 0.6, 0.18),
			AccelEntropy: nz(rng, 2.1, 0.3), StraightLineFrac: nz(rng, 0.18, 0.08), PauseCount: int(nz(rng, 3, 2)),
			MaxTeleportPx: nz(rng, 45, 20), MeanJerk: nz(rng, 0.4, 0.15), CoalescedRatio: nz(rng, 3.5, 1.0) + 1.5},
		Key: signals.KeyFeatures{Keystrokes: int(nz(rng, 16, 8)), MeanDwellMs: nz(rng, 95, 20),
			DwellStdDevMs: nz(rng, 28, 10) + 6, MeanFlightMs: nz(rng, 140, 35), FlightStdDev: nz(rng, 35, 12) + 8},
		Events:    signals.EventFeatures{TotalEvents: int(nz(rng, 60, 20)) + 5, ClickCount: int(nz(rng, 2, 1)), BurstVarMs: nz(rng, 400, 150), MinReactionMs: nz(rng, 280, 90) + 120},
		DurationS: nz(rng, 8, 3) + 1,
	}
}

func botBehavior(rng *rand.Rand) signals.BehaviorSummary {
	switch rng.Intn(4) {
	case 0: // scripted-smooth (humanizer): straight, low variance, synthetic coalesced
		return signals.BehaviorSummary{
			Mouse: signals.MouseFeatures{Samples: int(nz(rng, 40, 12)) + 5, StraightLineFrac: nz(rng, 0.9, 0.06),
				VelocityStdDev: nz(rng, 0.05, 0.04), MeanCurvature: nz(rng, 0.03, 0.02), MeanJerk: nz(rng, 0.05, 0.03),
				AccelEntropy: nz(rng, 0.6, 0.3), CoalescedRatio: nz(rng, 1.0, 0.1), MaxTeleportPx: nz(rng, 60, 30)},
			Key:       signals.KeyFeatures{Keystrokes: int(nz(rng, 12, 6)), MeanDwellMs: nz(rng, 90, 15), DwellStdDevMs: nz(rng, 2, 2), MeanFlightMs: nz(rng, 40, 10), FlightStdDev: nz(rng, 2, 2), ZeroDwellFrac: nz(rng, 0.6, 0.2)},
			Events:    signals.EventFeatures{TotalEvents: int(nz(rng, 50, 15)) + 5, ClickCount: int(nz(rng, 2, 1))},
			DurationS: nz(rng, 6, 2) + 1,
		}
	case 1: // no interaction (headless)
		return signals.BehaviorSummary{Events: signals.EventFeatures{TotalEvents: int(nz(rng, 2, 1))}, DurationS: nz(rng, 5, 2) + 2}
	case 2: // untrusted / CDP-injected events
		return signals.BehaviorSummary{
			Mouse:     signals.MouseFeatures{Samples: int(nz(rng, 20, 10)), CoalescedRatio: nz(rng, 1.0, 0.05), StraightLineFrac: nz(rng, 0.7, 0.15)},
			Events:    signals.EventFeatures{TotalEvents: int(nz(rng, 40, 15)) + 5, UntrustedFrac: nz(rng, 0.8, 0.2), NoMoveClicks: int(nz(rng, 3, 2)), SyntheticFlags: int(nz(rng, 4, 2)), ClickCount: int(nz(rng, 3, 2))},
			DurationS: nz(rng, 6, 2) + 1,
		}
	default: // AI-agent cadence: think-gaps + fast low-variance bursts
		return signals.BehaviorSummary{
			Mouse:     signals.MouseFeatures{Samples: int(nz(rng, 25, 10)), CoalescedRatio: nz(rng, 1.0, 0.1), MaxTeleportPx: nz(rng, 300, 120), StraightLineFrac: nz(rng, 0.85, 0.1)},
			Events:    signals.EventFeatures{TotalEvents: int(nz(rng, 30, 12)) + 5, LongGapCount: int(nz(rng, 5, 2)) + 1, BurstVarMs: nz(rng, 8, 5), MinReactionMs: nz(rng, 30, 15), ClickCount: int(nz(rng, 4, 2))},
			DurationS: nz(rng, 12, 4) + 2,
		}
	}
}

// --- standardization + MLP training (mini-batch SGD, BCE) ---

func standardize(s []sample) (mean, std []float32) {
	d := behavior.FeatureDim
	mean = make([]float32, d)
	std = make([]float32, d)
	for _, sm := range s {
		for i := 0; i < d; i++ {
			mean[i] += sm.x[i]
		}
	}
	n := float32(len(s))
	for i := range mean {
		mean[i] /= n
	}
	for _, sm := range s {
		for i := 0; i < d; i++ {
			dd := sm.x[i] - mean[i]
			std[i] += dd * dd
		}
	}
	for i := range std {
		std[i] = float32(math.Sqrt(float64(std[i]/n))) + 1e-6
	}
	return
}

func trainMLP(rng *rand.Rand, train []sample, mean, std []float32, hidden, epochs int, lr float32) mlserve.MLPBundle {
	d := behavior.FeatureDim
	// He-init
	w1 := make([][]float32, hidden)
	b1 := make([]float32, hidden)
	for h := range w1 {
		w1[h] = make([]float32, d)
		scale := float32(math.Sqrt(2.0 / float64(d)))
		for i := range w1[h] {
			w1[h][i] = float32(rng.NormFloat64()) * scale
		}
	}
	w2 := make([]float32, hidden)
	for h := range w2 {
		w2[h] = float32(rng.NormFloat64()) * float32(math.Sqrt(2.0/float64(hidden)))
	}
	var b2 float32

	norm := func(x behavior.FeatureVector) []float32 {
		z := make([]float32, d)
		for i := range x {
			z[i] = (x[i] - mean[i]) / std[i]
		}
		return z
	}

	for ep := 0; ep < epochs; ep++ {
		rng.Shuffle(len(train), func(i, j int) { train[i], train[j] = train[j], train[i] })
		for _, sm := range train {
			z := norm(sm.x)
			// forward
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
			p := sigmoid(logit)
			// backward (BCE): dL/dlogit = p - y
			dLogit := p - sm.y
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
		Version: "mlp-bootstrap-" + time.Now().UTC().Format("20060102") + "-h" + itoa(hidden),
		SchemaHash: behavior.SchemaHash(), FeatureDim: d, Hidden: hidden,
		Mean: mean, Std: std, W1: w1, B1: b1, W2: w2, B2: b2, CalA: 1, CalB: 0,
	}
}

func sigmoid(x float32) float32 { return float32(1 / (1 + math.Exp(float64(-x)))) }

// mlpForward is a minimal forward-only view for in-process eval (mirrors mlserve.MLP.Predict).
type mlpForward struct{ b mlserve.MLPBundle }

func (m *mlpForward) predict(x behavior.FeatureVector) float32 {
	b := m.b
	logit := b.B2
	for h := 0; h < b.Hidden; h++ {
		acc := b.B1[h]
		for i := 0; i < b.FeatureDim; i++ {
			acc += b.W1[h][i] * ((x[i] - b.Mean[i]) / b.Std[i])
		}
		if acc < 0 {
			acc = 0
		}
		logit += b.W2[h] * acc
	}
	return sigmoid(b.CalA*logit + b.CalB)
}

func evaluate(m *mlpForward, val []sample) (acc, humanFPR, botTPR float32) {
	var correct, humans, humanFP, bots, botTP int
	for _, s := range val {
		p := m.predict(s.x)
		pred := 0
		if p >= 0.5 {
			pred = 1
		}
		if float32(pred) == s.y {
			correct++
		}
		if s.human {
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
	acc = float32(correct) / float32(len(val))
	if humans > 0 {
		humanFPR = float32(humanFP) / float32(humans)
	}
	if bots > 0 {
		botTPR = float32(botTP) / float32(bots)
	}
	return
}

func itoa(i int) string { return fmt.Sprintf("%d", i) }
func fatal(err error)   { fmt.Fprintln(os.Stderr, "ml-train:", err); os.Exit(1) }

// Command ml-train trains the small policy-specific behavioral MLP (SoT-42 Pillar A) into a JSON
// bundle that internal/mlserve loads at the edge. Pure Go, no ML framework — the whole pipeline
// (feature transform → train → serve) runs in Docker with the Go toolchain only. The training math
// itself lives in internal/mltrain, shared with the self-correcting retrainer (cmd/mlcorrect-train).
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
	"math/rand"
	"os"
	"time"

	"github.com/modootoday/humanymous/internal/behavior"
	"github.com/modootoday/humanymous/internal/mltrain"
	"github.com/modootoday/humanymous/internal/signals"
)

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
	var samples []mltrain.Sample
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

	rng.Shuffle(len(samples), func(i, j int) { samples[i], samples[j] = samples[j], samples[i] })
	nVal := len(samples) / 5
	val, train := samples[:nVal], samples[nVal:]

	mean, std := mltrain.Standardize(train)
	version := "mlp-bootstrap-" + time.Now().UTC().Format("20060102") + fmt.Sprintf("-h%d", *hidden)
	b := mltrain.Train(mltrain.Config{Hidden: *hidden, Epochs: *epochs, LR: float32(*lr), Seed: *seed, Version: version}, train, mean, std, nil)

	acc, humanFPR, botTPR := mltrain.Evaluate(b, val)
	fmt.Fprintf(os.Stderr, "ml-train: val acc=%.3f  bot TPR=%.3f  human FPR=%.3f  (bundle %s, schema %s)\n",
		acc, botTPR, humanFPR, b.Version, b.SchemaHash)

	raw, _ := json.MarshalIndent(b, "", " ")
	if err := os.WriteFile(*out, raw, 0o644); err != nil {
		fatal(err)
	}
	fmt.Fprintf(os.Stderr, "ml-train: wrote %s (%d bytes)\n", *out, len(raw))
}

// --- data ---

func readData(path string) ([]mltrain.Sample, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	var out []mltrain.Sample
	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 1<<20), 1<<20)
	for sc.Scan() {
		line := sc.Bytes()
		if len(line) == 0 {
			continue
		}
		var rec mltrain.Record
		if err := json.Unmarshal(line, &rec); err != nil {
			return nil, fmt.Errorf("bad JSONL line: %w", err)
		}
		out = append(out, rec.Sample())
	}
	return out, sc.Err()
}

// --- synthetic bootstrap (grounded in catalog bot values + test human values) ---

func genSynthetic(rng *rand.Rand, n int) []mltrain.Sample {
	out := make([]mltrain.Sample, 0, n)
	for i := 0; i < n; i++ {
		if i%2 == 0 {
			out = append(out, mltrain.Sample{X: behavior.Extract(humanBehavior(rng)), Y: 0, Human: true})
		} else {
			out = append(out, mltrain.Sample{X: behavior.Extract(botBehavior(rng)), Y: 1, Human: false})
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

func fatal(err error) { fmt.Fprintln(os.Stderr, "ml-train:", err); os.Exit(1) }

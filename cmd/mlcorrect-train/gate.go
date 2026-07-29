package main

import (
	"fmt"
	"sort"

	"github.com/modootoday/humanymous/internal/mlserve"
	"github.com/modootoday/humanymous/internal/mltrain"
)

// gate.go is the testable core of the offline promotion gate (SoT-42 Pillar A, SELF-CORRECTION.md
// ⑤–⑥). runGate does the whole train → TESSERACT-evaluate → gate decision with NO file I/O and NO
// os.Exit, so the fail-closed invariant (a candidate is emitted ONLY IF every gate passes, and a
// regressed accessibility cohort blocks it) is a deterministic unit-testable property, not something
// only observable by running the binary. The binary (main.go) is a thin I/O shell over this.

// gateConfig is the promotion-gate policy.
type gateConfig struct {
	Version         string
	Hidden, Epochs  int
	LR              float64
	Seed            int64
	TestFrac        float64
	MaxHumanFPR     float64
	MinBotTPR       float64
	MinCohortHumans int
}

// gateResult is the outcome. Candidate is non-nil IFF Failures is empty.
type gateResult struct {
	Candidate     *mlserve.MLPBundle
	Failures      []string
	Acc           float32
	AggHumanFPR   float32
	BotTPR        float32
	AUT           float32
	Cohorts       map[string]mltrain.CohortStat
	NTrain, NTest int
}

// runGate trains a candidate from samples (warm-starting from base when non-nil) and evaluates it on
// a TESSERACT time-ordered holdout against the promotion gates: per-cohort human-FP ≈ 0 (BLOCKING, a
// cohort with too few humans fails CLOSED), and catalog/bot TPR no-regression. It returns an error
// only for a precondition it cannot evaluate at all (too few samples, empty test window).
func runGate(samples []mltrain.Sample, base *mlserve.MLPBundle, cfg gateConfig) (gateResult, error) {
	if len(samples) < 100 {
		return gateResult{}, fmt.Errorf("need >=100 samples, got %d", len(samples))
	}
	train, test := mltrain.TesseractSplit(samples, cfg.TestFrac)
	if len(test) == 0 {
		return gateResult{}, fmt.Errorf("empty test window — cannot validate a candidate; refusing to emit")
	}

	mean, std := mltrain.Standardize(train)
	cand := mltrain.Train(mltrain.Config{
		Hidden: cfg.Hidden, Epochs: cfg.Epochs, LR: float32(cfg.LR), Seed: cfg.Seed, Version: cfg.Version,
	}, train, mean, std, base)

	acc, aggFPR, botTPR := mltrain.Evaluate(cand, test)
	res := gateResult{
		Acc: acc, AggHumanFPR: aggFPR, BotTPR: botTPR,
		AUT:     mltrain.AUT(cand, test, 5),
		Cohorts: mltrain.CohortFPR(cand, test),
		NTrain:  len(train), NTest: len(test),
	}

	// per-cohort human-FP gate (BLOCKING, never aggregate) — evaluate in a stable order.
	names := make([]string, 0, len(res.Cohorts))
	for c := range res.Cohorts {
		names = append(names, c)
	}
	sort.Strings(names)
	for _, c := range names {
		st := res.Cohorts[c]
		if st.Humans < cfg.MinCohortHumans {
			res.Failures = append(res.Failures, fmt.Sprintf("cohort %q has only %d humans (<%d): cannot confirm human-FP≈0", c, st.Humans, cfg.MinCohortHumans))
		} else if float64(st.FPR) > cfg.MaxHumanFPR {
			res.Failures = append(res.Failures, fmt.Sprintf("cohort %q human FPR %.4f > %.4f", c, st.FPR, cfg.MaxHumanFPR))
		}
	}
	// catalog no-regression
	if float64(botTPR) < cfg.MinBotTPR {
		res.Failures = append(res.Failures, fmt.Sprintf("bot TPR %.3f < floor %.3f (catalog regression)", botTPR, cfg.MinBotTPR))
	}

	if len(res.Failures) == 0 {
		res.Candidate = &cand // emitted ONLY when every gate passed
	}
	return res, nil
}

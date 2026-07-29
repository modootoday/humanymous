// Command mlcorrect-train is the OFFLINE retrain step of the self-correcting loop (SoT-42 Pillar A,
// SELF-CORRECTION.md steps ⑤–⑥). It takes the frozen GOLD ANCHOR plus ORACLE-CONFIRMED labels
// (Pass→human, catalog/failed-CHALLENGE→bot; never the model's own outputs — that is model
// collapse), warm-starts from the current bundle, and produces a CANDIDATE bundle ONLY IF it clears
// the promotion gates:
//
//   - per-cohort human FPR ≈ 0 (BLOCKING) — a candidate that regresses ANY accessibility cohort is
//     discarded, never merely the aggregate;
//   - catalog/bot TPR no-regression vs a floor;
//   - reported on a TESSERACT time-ordered split (train past / test future) with an AUT metric, so
//     temporal decay a random split would hide is exposed.
//
// It does NOT deploy: a passing candidate is written to disk and its sha256 digest printed, to be
// signed and Staged/Promoted through internal/mlcorrect.BundleManager under dual control. A FAILED
// gate exits non-zero and writes NOTHING — fail-closed, the CI job stays red.
//
//	mlcorrect-train -anchor gold.jsonl -oracle confirmed.jsonl -base current.json -out candidate.json
//
// JSONL record: {label:0|1, behavior:{...}, ts:<unix>, cohort:"<id>", source:"catalog|pass|challenge"}.
package main

import (
	"bufio"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"sort"
	"time"

	"github.com/modootoday/humanymous/internal/mlcorrect"
	"github.com/modootoday/humanymous/internal/mlserve"
	"github.com/modootoday/humanymous/internal/mltrain"
)

func main() {
	anchor := flag.String("anchor", "", "frozen GOLD ANCHOR JSONL (catalog bots + confirmed humans) — required")
	oracle := flag.String("oracle", "", "oracle-confirmed labels JSONL from the self-correcting loop (optional)")
	base := flag.String("base", "", "current serving bundle to WARM-START from (optional; cold-start if absent)")
	out := flag.String("out", "candidate.json", "candidate bundle output path (written only if gates pass)")
	hidden := flag.Int("hidden", 16, "hidden units (must match base to warm-start)")
	epochs := flag.Int("epochs", 60, "training epochs")
	lr := flag.Float64("lr", 0.05, "learning rate")
	seed := flag.Int64("seed", 1, "rng seed")
	testFrac := flag.Float64("test-frac", 0.25, "TESSERACT time-ordered holdout fraction")
	maxHumanFPR := flag.Float64("max-human-fpr", 0.005, "BLOCKING: max per-cohort human false-positive rate")
	minBotTPR := flag.Float64("min-bot-tpr", 0.80, "BLOCKING: minimum bot TPR (catalog no-regression floor)")
	minCohortHumans := flag.Int("min-cohort-humans", 20, "min humans a cohort needs before its FPR gate is trusted")
	flag.Parse()

	if *anchor == "" {
		fatal(fmt.Errorf("-anchor is required (the frozen gold anchor prevents model collapse)"))
	}
	samples := mustRead(*anchor, "anchor")
	if *oracle != "" {
		samples = append(samples, mustRead(*oracle, "oracle")...)
	}

	// Warm-start from the current bundle when provided (incremental adapter continuity).
	var baseBundle *mlserve.MLPBundle
	if *base != "" {
		m, err := mlserve.LoadMLP(*base) // validates on-disk shape before we trust it
		if err != nil {
			fatal(fmt.Errorf("load base bundle: %w", err))
		}
		bb := bundleOf(*base)
		baseBundle = &bb
		fmt.Fprintf(os.Stderr, "mlcorrect-train: warm-starting from %s (%s)\n", *base, m.BundleVersion())
	}

	res, err := runGate(samples, baseBundle, gateConfig{
		Version:  "mlp-corrected-" + time.Now().UTC().Format("20060102-150405"),
		Hidden:   *hidden, Epochs: *epochs, LR: *lr, Seed: *seed, TestFrac: *testFrac,
		MaxHumanFPR: *maxHumanFPR, MinBotTPR: *minBotTPR, MinCohortHumans: *minCohortHumans,
	})
	if err != nil {
		fatal(err)
	}
	fmt.Fprintf(os.Stderr, "mlcorrect-train: %d train / %d test (TESSERACT time-split)\n", res.NTrain, res.NTest)
	fmt.Fprintf(os.Stderr, "mlcorrect-train: test acc=%.3f  botTPR=%.3f  aggHumanFPR=%.3f  AUT=%.3f\n", res.Acc, res.BotTPR, res.AggHumanFPR, res.AUT)
	for _, c := range sortedCohorts(res.Cohorts) {
		st := res.Cohorts[c]
		fmt.Fprintf(os.Stderr, "  cohort %-12s humans=%-4d FP=%-3d FPR=%.4f\n", c, st.Humans, st.FP, st.FPR)
	}

	if res.Candidate == nil {
		fmt.Fprintln(os.Stderr, "mlcorrect-train: PROMOTION GATES FAILED — candidate discarded, nothing written:")
		for _, f := range res.Failures {
			fmt.Fprintln(os.Stderr, "  ✗", f)
		}
		os.Exit(3)
	}

	raw, _ := json.MarshalIndent(res.Candidate, "", " ")
	if err := os.WriteFile(*out, raw, 0o644); err != nil {
		fatal(err)
	}
	digest := mlcorrect.Digest(raw)
	fmt.Fprintf(os.Stderr, "mlcorrect-train: GATES PASSED — wrote %s (%d bytes)\n", *out, len(raw))
	fmt.Fprintf(os.Stderr, "mlcorrect-train: bundle=%s digest=%s\n", res.Candidate.Version, digest)
	// stdout carries the digest for the signing/staging step to consume.
	fmt.Println(digest)
}

func sortedCohorts(m map[string]mltrain.CohortStat) []string {
	names := make([]string, 0, len(m))
	for c := range m {
		names = append(names, c)
	}
	sort.Strings(names)
	return names
}

// bundleOf recovers the on-disk MLPBundle by reloading it (mlserve.MLP does not expose its bundle;
// re-reading the file is the simplest honest path for the trainer to warm-start from it). The caller
// has already validated the same file via mlserve.LoadMLP.
func bundleOf(path string) mlserve.MLPBundle {
	raw, err := os.ReadFile(path)
	if err != nil {
		fatal(err)
	}
	var b mlserve.MLPBundle
	if err := json.Unmarshal(raw, &b); err != nil {
		fatal(fmt.Errorf("re-read base bundle: %w", err))
	}
	return b
}

func mustRead(path, kind string) []mltrain.Sample {
	f, err := os.Open(path)
	if err != nil {
		fatal(err)
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
		var r mltrain.Record
		if err := json.Unmarshal(line, &r); err != nil {
			fatal(fmt.Errorf("%s: bad JSONL line: %w", kind, err))
		}
		out = append(out, r.Sample())
	}
	if err := sc.Err(); err != nil {
		fatal(err)
	}
	return out
}

func fatal(err error) { fmt.Fprintln(os.Stderr, "mlcorrect-train:", err); os.Exit(1) }

package mltrain

import (
	"math/rand"
	"testing"

	"github.com/modootoday/humanymous/internal/behavior"
)

// synth builds a linearly-separable sample: humans cluster low on feature 0, bots high, with a
// little noise elsewhere, so the kernel has something learnable without real traces.
func synth(rng *rand.Rand, bot bool, ts int64, cohort string) Sample {
	x := make(behavior.FeatureVector, behavior.FeatureDim)
	for i := range x {
		x[i] = float32(rng.NormFloat64()) * 0.1
	}
	if bot {
		x[0] = 3 + float32(rng.NormFloat64())*0.2
	} else {
		x[0] = 0 + float32(rng.NormFloat64())*0.2
	}
	y := float32(0)
	if bot {
		y = 1
	}
	return Sample{X: x, Y: y, Human: !bot, TS: ts, Cohort: cohort}
}

func dataset(rng *rand.Rand, n int) []Sample {
	s := make([]Sample, 0, n)
	for i := 0; i < n; i++ {
		s = append(s, synth(rng, i%2 == 0, int64(i), "default"))
	}
	return s
}

func TestTrainAndEvaluate_Separable(t *testing.T) {
	rng := rand.New(rand.NewSource(1))
	train := dataset(rng, 800)
	val := dataset(rng, 200)
	mean, std := Standardize(train)
	b := Train(Config{Hidden: 8, Epochs: 40, LR: 0.1, Seed: 2, Version: "t1"}, train, mean, std, nil)
	acc, humanFPR, botTPR := Evaluate(b, val)
	if acc < 0.95 || humanFPR > 0.05 || botTPR < 0.95 {
		t.Fatalf("separable data should train cleanly: acc=%.3f humanFPR=%.3f botTPR=%.3f", acc, humanFPR, botTPR)
	}
	if b.SchemaHash != behavior.SchemaHash() {
		t.Fatalf("bundle must pin the engine schema, got %q", b.SchemaHash)
	}
}

func TestWarmStart_UsesBaseAndKeepsShape(t *testing.T) {
	rng := rand.New(rand.NewSource(3))
	train := dataset(rng, 400)
	mean, std := Standardize(train)
	base := Train(Config{Hidden: 6, Epochs: 30, LR: 0.1, Seed: 4, Version: "base"}, train, mean, std, nil)

	// warm-start from base: 0 epochs → weights must be copied from base verbatim (proves warm path).
	got := Train(Config{Hidden: 6, Epochs: 0, LR: 0.1, Seed: 5, Version: "warm"}, train, mean, std, &base)
	if got.Hidden != base.Hidden {
		t.Fatalf("warm-start must keep base shape: hidden %d != %d", got.Hidden, base.Hidden)
	}
	if got.B2 != base.B2 || got.W2[0] != base.W2[0] {
		t.Fatal("0-epoch warm-start must copy base weights (adapter continuity)")
	}
}

func TestTesseractSplit_TrainPastTestFuture(t *testing.T) {
	rng := rand.New(rand.NewSource(6))
	// scramble input order; split must still put earliest TS in train, latest in test.
	s := dataset(rng, 100)
	rng.Shuffle(len(s), func(i, j int) { s[i], s[j] = s[j], s[i] })
	train, test := TesseractSplit(s, 0.2)
	if len(test) == 0 || len(train)+len(test) != len(s) {
		t.Fatalf("bad split sizes train=%d test=%d", len(train), len(test))
	}
	var maxTrain, minTest int64 = -1, 1 << 62
	for _, x := range train {
		if x.TS > maxTrain {
			maxTrain = x.TS
		}
	}
	for _, x := range test {
		if x.TS < minTest {
			minTest = x.TS
		}
	}
	if maxTrain >= minTest {
		t.Fatalf("test window must be strictly after train: maxTrain=%d minTest=%d", maxTrain, minTest)
	}
}

func TestCohortFPR_IsPerCohort(t *testing.T) {
	rng := rand.New(rand.NewSource(7))
	train := dataset(rng, 600)
	mean, std := Standardize(train)
	b := Train(Config{Hidden: 8, Epochs: 40, LR: 0.1, Seed: 8, Version: "c"}, train, mean, std, nil)

	// two human cohorts: "typical" sits at the human cluster; "edge" is pushed toward the bot side
	// (a stand-in for an accessibility cohort the model handles worse).
	var val []Sample
	for i := 0; i < 100; i++ {
		val = append(val, synth(rng, false, int64(i), "typical"))
		edge := synth(rng, false, int64(i), "edge")
		edge.X[0] = 2.2 // shifted toward bots
		val = append(val, edge)
	}
	stats := CohortFPR(b, val)
	if stats["typical"].Humans == 0 || stats["edge"].Humans == 0 {
		t.Fatal("both cohorts must be tallied")
	}
	if stats["edge"].FPR <= stats["typical"].FPR {
		t.Fatalf("the harder cohort must show a higher FPR (per-cohort visibility): typical=%.3f edge=%.3f",
			stats["typical"].FPR, stats["edge"].FPR)
	}
}

func TestAUT_BoundedAndFull(t *testing.T) {
	rng := rand.New(rand.NewSource(9))
	train := dataset(rng, 600)
	mean, std := Standardize(train)
	b := Train(Config{Hidden: 8, Epochs: 40, LR: 0.1, Seed: 10, Version: "a"}, train, mean, std, nil)
	test := dataset(rng, 200)
	aut := AUT(b, test, 5)
	if aut < 0 || aut > 1 {
		t.Fatalf("AUT must be a bounded metric, got %v", aut)
	}
	if aut < 0.9 {
		t.Fatalf("stable separable data should hold high accuracy across time buckets, got %v", aut)
	}
	if AUT(b, nil, 5) != 0 {
		t.Fatal("AUT of an empty test set must be 0")
	}
}

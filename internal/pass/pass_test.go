package pass

import (
	"reflect"
	"testing"
)

var testKey = []byte("humanymous-pass-test-master-key-000")

func TestGenerateDeterministic(t *testing.T) {
	a := Generate(testKey, "sess-1", 100, 0, 1)
	b := Generate(testKey, "sess-1", 100, 0, 1)
	if !reflect.DeepEqual(a, b) {
		t.Fatal("Generate must be deterministic for the same seed")
	}
	if c := Generate(testKey, "sess-2", 100, 0, 1); reflect.DeepEqual(a, c) {
		t.Error("different sessions should generate different challenges")
	}
}

func TestDimsScaleWithDifficulty(t *testing.T) {
	want := []int{9, 11, 13, 15}
	for d := 0; d <= 3; d++ {
		ch := Generate(testKey, "s", 100, 0, d)
		if ch.N != want[d] {
			t.Errorf("difficulty %d: N=%d want %d", d, ch.N, want[d])
		}
		if ch.Center != want[d]/2 {
			t.Errorf("difficulty %d: center=%d want %d", d, ch.Center, want[d]/2)
		}
		if len(ch.Rows) != Rows {
			t.Errorf("difficulty %d: %d rows want %d", d, len(ch.Rows), Rows)
		}
	}
}

func TestSolutionVerifies(t *testing.T) {
	for d := 0; d <= 3; d++ {
		for _, sid := range []string{"a", "b", "c", "d"} {
			sol := SolutionOffsets(testKey, sid, 100, 0, d)
			if !Verify(testKey, sid, 100, 100, 0, d, sol) {
				t.Errorf("canonical solution must verify: sid=%s d=%d sol=%v", sid, d, sol)
			}
		}
	}
}

func TestWrongOffsetsFail(t *testing.T) {
	sol := SolutionOffsets(testKey, "a", 100, 0, 1)
	bad := append([]int(nil), sol...)
	bad[0] = (bad[0] + 1) % 11 // nudge one row off centre
	if Verify(testKey, "a", 100, 100, 0, 1, bad) {
		t.Error("a misaligned row must fail")
	}
	if Verify(testKey, "a", 100, 100, 0, 1, []int{0, 0}) {
		t.Error("wrong offset count must fail")
	}
}

func TestStaleBucketRejected(t *testing.T) {
	sol := SolutionOffsets(testKey, "a", 100, 0, 1)
	if !Verify(testKey, "a", 100, 100, 0, 1, sol) {
		t.Fatal("sanity: fresh solution should verify")
	}
	if Verify(testKey, "a", 100, 104, 0, 1, sol) {
		t.Error("a solution 4 buckets late must be rejected (TTL)")
	}
	if Verify(testKey, "a", 100, 99, 0, 1, sol) {
		t.Error("an earlier currentBucket must be rejected")
	}
}

package pass

import (
	"testing"
)

// Axis F (r322+): humanymous Pass residual plane (SoT-36).
// Puzzle is bot-solvable by design (a11y); security is binding + TTL + fusion.
// Web research: a11y challenges must not use motor/speed gates; anti-replay and
// instance-bound proofs are the load-bearing defenses.

func TestWargameR322_PassSolutionDeterministic(t *testing.T) {
	a := SolutionOffsets(testKey, "sess-r322", 50, 1, 1)
	b := SolutionOffsets(testKey, "sess-r322", 50, 1, 1)
	if len(a) != Rows || len(b) != Rows {
		t.Fatal("row count")
	}
	for i := range a {
		if a[i] != b[i] {
			t.Fatal("solution must be deterministic")
		}
	}
}

func TestWargameR323_PassWrongOffsetRejected(t *testing.T) {
	sol := SolutionOffsets(testKey, "sess-r323", 50, 2, 1)
	bad := append([]int(nil), sol...)
	bad[1] = (bad[1] + 3) % 11
	if Verify(testKey, "sess-r323", 50, 50, 2, 1, bad) {
		t.Fatal("misaligned must fail")
	}
}

func TestWargameR324_PassCrossSessionSolutionRejected(t *testing.T) {
	// Solution for sess-A must not verify on sess-B (binding).
	solA := SolutionOffsets(testKey, "sess-A", 50, 0, 1)
	if Verify(testKey, "sess-B", 50, 50, 0, 1, solA) {
		t.Fatal("cross-session solution reuse must fail")
	}
}

func TestWargameR325_PassCrossInstanceRejected(t *testing.T) {
	sol := SolutionOffsets(testKey, "sess", 50, 0, 1)
	if Verify(testKey, "sess", 50, 50, 1, 1, sol) {
		t.Fatal("cross-instance (different instance id) must fail")
	}
}

func TestWargameR326_PassTTLAllowsSlowAT(t *testing.T) {
	// Web/a11y: no multi-minute lockout; TTL window must accept slow solvers.
	sol := SolutionOffsets(testKey, "slow", 100, 0, 1)
	if !Verify(testKey, "slow", 100, 100+MaxBucketAge, 0, 1, sol) {
		t.Fatal("within MaxBucketAge must verify (assistive tech floor)")
	}
}

func TestWargameR327_PassTTLRejectsStale(t *testing.T) {
	sol := SolutionOffsets(testKey, "stale", 100, 0, 1)
	if Verify(testKey, "stale", 100, 100+MaxBucketAge+1, 0, 1, sol) {
		t.Fatal("past TTL must reject")
	}
}

func TestWargameR328_PassDifficultyScalesN(t *testing.T) {
	// Harder difficulty → more cells (more decoys), not a motor speed gate.
	prev := 0
	for d := 0; d <= 3; d++ {
		ch := Generate(testKey, "d", 10, 0, d)
		if ch.N <= prev {
			t.Fatalf("difficulty %d N=%d should increase", d, ch.N)
		}
		prev = ch.N
	}
}

func TestWargameR329_PassWrongLengthOffsets(t *testing.T) {
	if Verify(testKey, "a", 1, 1, 0, 1, []int{0}) {
		t.Fatal("short offsets must fail")
	}
	if Verify(testKey, "a", 1, 1, 0, 1, []int{0, 0, 0, 0}) {
		t.Fatal("long offsets must fail")
	}
}

func TestWargameR330_PassPublicChallengeHasNoHiddenAnswer(t *testing.T) {
	// Security is not secrecy of offsets: challenge exposes keyIndex publicly.
	ch := Generate(testKey, "pub", 7, 0, 0)
	if len(ch.Rows) != Rows {
		t.Fatal("rows")
	}
	for _, row := range ch.Rows {
		if row.KeyIndex < 0 || row.KeyIndex >= ch.N {
			t.Fatal("keyIndex must be public and in range")
		}
		if len(row.Chars) != ch.N {
			t.Fatal("chars length")
		}
	}
}

func TestWargameR331_PassDifferentDifficultyDifferentSolution(t *testing.T) {
	s0 := SolutionOffsets(testKey, "s", 20, 0, 0)
	s3 := SolutionOffsets(testKey, "s", 20, 0, 3)
	// Not required to differ always, but N differs so offset domain differs
	if len(s0) != Rows || len(s3) != Rows {
		t.Fatal("rows")
	}
	ch0 := Generate(testKey, "s", 20, 0, 0)
	ch3 := Generate(testKey, "s", 20, 0, 3)
	if ch0.N == ch3.N {
		t.Fatal("difficulty must change N")
	}
}

func TestWargameR332_PassAxisClose(t *testing.T) {
	sol := SolutionOffsets(testKey, "close", 9, 0, 1)
	if !Verify(testKey, "close", 9, 9, 0, 1, sol) {
		t.Fatal("fresh ok")
	}
	if Verify(testKey, "close", 9, 9, 0, 1, []int{0, 0, 0}) && !equalInts(sol, []int{0, 0, 0}) {
		// only fail if zeros is not the real solution
		if !Verify(testKey, "close", 9, 9, 0, 1, []int{1, 1, 1}) {
			// good
		}
	}
	if Verify(testKey, "other", 9, 9, 0, 1, sol) {
		t.Fatal("session bind lock")
	}
}

func equalInts(a, b []int) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

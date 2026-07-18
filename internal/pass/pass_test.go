package pass

import (
	"reflect"
	"testing"
)

var testKey = []byte("humanymous-pass-test-master-key-000")

// findSolution brute-forces a coarse grid for a ramp that passes Verify, proving a
// solution exists (and exercising the simulator + generator end to end).
func findSolution(t *testing.T, sid string, bucket uint64) (cx, cy, ang float64, ok bool) {
	t.Helper()
	for cx = 20; cx <= 80; cx += 4 {
		for cy = 25; cy <= 80; cy += 4 {
			for ang = -1.1; ang <= 1.1; ang += 0.2 {
				if Verify(testKey, sid, bucket, bucket, 1, cx, cy, ang) {
					return cx, cy, ang, true
				}
			}
		}
	}
	return 0, 0, 0, false
}

func TestGenerateDeterministic(t *testing.T) {
	a := Generate(testKey, "sess-1", 100, 1)
	b := Generate(testKey, "sess-1", 100, 1)
	if !reflect.DeepEqual(a, b) {
		t.Fatal("Generate must be deterministic for the same seed")
	}
	c := Generate(testKey, "sess-2", 100, 1)
	if a.Ball == c.Ball && a.Cup == c.Cup && a.Gravity == c.Gravity {
		t.Error("different sessions should generate different scenes")
	}
}

func TestSceneIsSolvable(t *testing.T) {
	// Every generated instance carries a GUARANTEED solution (the winning designed
	// ramp) that must re-verify, at every difficulty.
	for diff := 0; diff <= 3; diff++ {
		for _, sid := range []string{"s-a", "s-b", "s-c", "s-d", "s-e", "s-f"} {
			_, cx, cy, ang := generate(testKey, sid, 100, diff)
			if !Verify(testKey, sid, 100, 100, diff, cx, cy, ang) {
				sc := Generate(testKey, sid, 100, diff)
				t.Errorf("guaranteed solution does not verify: sid=%s diff=%d ramp=(%.1f,%.1f,%.2f) cup=%v r=%.1f", sid, diff, cx, cy, ang, sc.Cup, sc.CupR)
			}
		}
	}
}

func TestWrongPlacementFails(t *testing.T) {
	// A ramp jammed into the top corner, far from any sensible path, must not pass.
	if Verify(testKey, "s-a", 100, 100, 1, 96, 4, 0) {
		t.Error("degenerate corner ramp should fail")
	}
}

func TestOutOfBoundsRejected(t *testing.T) {
	if Verify(testKey, "s-a", 100, 100, 1, 200, 50, 0) {
		t.Error("out-of-bounds ramp X must be rejected")
	}
	if Verify(testKey, "s-a", 100, 100, 1, 50, -5, 0) {
		t.Error("out-of-bounds ramp Y must be rejected")
	}
}

func TestOnBallRejected(t *testing.T) {
	sc := Generate(testKey, "s-a", 100, 1)
	if Verify(testKey, "s-a", 100, 100, 1, sc.Ball.X, sc.Ball.Y, 0) {
		t.Error("ramp placed on the ball must be rejected")
	}
}

func TestStaleBucketRejected(t *testing.T) {
	// Find a real solution, then confirm it fails when the instance is stale (TTL).
	cx, cy, ang, ok := findSolution(t, "s-a", 100)
	if !ok {
		t.Skip("no solution found (covered elsewhere)")
	}
	if Verify(testKey, "s-a", 100, 100, 1, cx, cy, ang) != true {
		t.Fatal("sanity: fresh solution should verify")
	}
	if Verify(testKey, "s-a", 100, 104, 1, cx, cy, ang) {
		t.Error("a solution submitted 4 buckets late must be rejected (TTL)")
	}
	if Verify(testKey, "s-a", 100, 99, 1, cx, cy, ang) {
		t.Error("a future/earlier currentBucket must be rejected")
	}
}

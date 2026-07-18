package pow

import "testing"

var mk = []byte("master-key")

func TestSolveAndVerify(t *testing.T) {
	sid := "sess-1"
	bucket := uint64(1000)
	diff := 12 // small for a fast test
	ch := Issue(mk, sid, diff, bucket)

	nonce, ok := Solve(ch.Seed, diff, 1<<24)
	if !ok {
		t.Fatal("failed to solve PoW")
	}
	if !Verify(mk, sid, diff, bucket, bucket, nonce) {
		t.Fatal("valid solution rejected")
	}
	// wrong nonce fails.
	if Verify(mk, sid, diff, bucket, bucket, "not-a-solution-xyz") {
		t.Fatal("invalid nonce accepted")
	}
	// wrong session fails (seed differs).
	if Verify(mk, "other-sess", diff, bucket, bucket, nonce) {
		t.Fatal("solution accepted for wrong session")
	}
	// stale bucket fails.
	if Verify(mk, sid, diff, bucket, bucket+5, nonce) {
		t.Fatal("stale bucket accepted")
	}
}

func TestLeadingZeroBits(t *testing.T) {
	cases := []struct {
		b    []byte
		want int
	}{
		{[]byte{0xff}, 0},
		{[]byte{0x0f}, 4},
		{[]byte{0x00, 0xff}, 8},
		{[]byte{0x00, 0x0f}, 12},
		{[]byte{0x80}, 0},
		{[]byte{0x01}, 7},
	}
	for _, c := range cases {
		if got := leadingZeroBits(c.b); got != c.want {
			t.Errorf("leadingZeroBits(%x)=%d want %d", c.b, got, c.want)
		}
	}
}

func TestDifficultyScales(t *testing.T) {
	if DifficultyFor(10) != 0 {
		t.Error("trusted session should get no PoW")
	}
	if DifficultyFor(35) <= 0 || DifficultyFor(75) <= DifficultyFor(35) {
		t.Error("difficulty should increase with risk")
	}
}

func TestParseSolution(t *testing.T) {
	b, n, ok := ParseSolution("1000:12345")
	if !ok || b != 1000 || n != "12345" {
		t.Fatalf("parse failed: %d %q %v", b, n, ok)
	}
	if _, _, ok := ParseSolution("garbage"); ok {
		t.Error("garbage should not parse")
	}
}

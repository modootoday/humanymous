package audit

import "testing"

// SoT-38 P0-5: SelfVerify must include independent witness co-signatures, not only
// the writer key path (Verify).
func TestSelfVerifyRequiresWitness(t *testing.T) {
	w := NewWitness()
	l := NewLog(Config{NodeID: "n", HMACKey: []byte("k-p0-5"), CheckpointEvery: 4, Witness: w})
	for i := 0; i < 8; i++ {
		l.Append(sampleRecord(i))
	}
	l.Checkpoint()
	if res := l.SelfVerify(); !res.OK {
		t.Fatalf("witnessed chain must SelfVerify: %+v", res)
	}

	// Strip witness signatures — writer STH still valid, but independent attestation is gone.
	cps := l.Checkpoints()
	if len(cps) == 0 {
		t.Fatal("expected checkpoints")
	}
	for i := range cps {
		cps[i].WitnessSig = ""
	}
	l.mu.Lock()
	l.checkpoints = cps
	l.mu.Unlock()

	res := l.SelfVerify()
	if res.OK {
		t.Fatal("SelfVerify must fail when witness co-signatures are missing")
	}
	if res.Class != ClassWitnessBad {
		t.Fatalf("class=%s want %s detail=%s", res.Class, ClassWitnessBad, res.Detail)
	}
}

// Writer-only Verify still passes on stripped witness (documents the gap SelfVerify closes).
func TestVerifyAloneDoesNotCheckWitness(t *testing.T) {
	w := NewWitness()
	l := NewLog(Config{NodeID: "n", HMACKey: []byte("k-p0-5b"), CheckpointEvery: 4, Witness: w})
	for i := 0; i < 8; i++ {
		l.Append(sampleRecord(i))
	}
	l.Checkpoint()
	cps := l.Checkpoints()
	for i := range cps {
		cps[i].WitnessSig = ""
	}
	recs := l.Records()
	if res := Verify(recs, cps, l.hmacKey, l.PublicKey()); !res.OK {
		t.Fatalf("Verify (writer path) should still pass without witness: %+v", res)
	}
	if at, ok := VerifyWitness(cps, l.WitnessPublicKey()); ok || at == 0 && ok {
		t.Fatal("VerifyWitness must reject missing co-signatures")
	}
}

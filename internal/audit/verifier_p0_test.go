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

// SoT-38 P0-5: empty chain is a hard fail in the library (not only in CLI).
func TestVerifyEmptyChainFails(t *testing.T) {
	l := NewLog(Config{NodeID: "n", HMACKey: []byte("k-empty")})
	res := Verify(nil, nil, []byte("k-empty"), l.PublicKey())
	if res.OK || res.Class != ClassEmptyChain {
		t.Fatalf("empty Verify must fail empty-chain: %+v", res)
	}
	// SelfVerify on a log that never Append'd is the same vacuous-green trap.
	if res := l.SelfVerify(); res.OK || res.Class != ClassEmptyChain {
		t.Fatalf("empty SelfVerify must fail empty-chain: %+v", res)
	}
	// Nil HMAC must not rebrand empty as hmac-unchecked OK.
	if res := Verify(nil, nil, nil, l.PublicKey()); res.OK || res.Class != ClassEmptyChain {
		t.Fatalf("empty+nilHMAC must fail empty-chain: %+v", res)
	}
}

// Append refuses to seal without an HMAC key (writers need real material).
func TestAppendPanicsWithoutHMAC(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatal("Append with empty HMAC must panic")
		}
	}()
	l := NewLog(Config{NodeID: "n", HMACKey: nil})
	l.Append(sampleRecord(0))
}

// SoT-38 WS2: public-key-only auditors pass nil HMAC key.
func TestVerifyNilHMACSkipsSecondaryLayer(t *testing.T) {
	w := NewWitness()
	l := NewLog(Config{NodeID: "n", HMACKey: []byte("secret-hmac"), CheckpointEvery: 4, Witness: w})
	for i := 0; i < 8; i++ {
		l.Append(sampleRecord(i))
	}
	l.Checkpoint()
	recs, cps := l.Records(), l.Checkpoints()
	res := Verify(recs, cps, nil, l.PublicKey())
	if !res.OK || res.Class != ClassHMACUnchecked {
		t.Fatalf("nil HMAC should skip layer: %+v", res)
	}
	// Wrong non-nil HMAC still fails.
	if res := Verify(recs, cps, []byte("wrong"), l.PublicKey()); res.OK || res.Class != ClassHMACInvalid {
		t.Fatalf("wrong HMAC must fail: %+v", res)
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

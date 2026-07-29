package main

import (
	"crypto/ed25519"
	"crypto/rand"
	"crypto/x509"
	"encoding/json"
	"encoding/pem"
	"os"
	"path/filepath"
	"testing"

	"github.com/modootoday/humanymous/internal/behavior"
	"github.com/modootoday/humanymous/internal/mlserve"
)

// writeValidBundle writes a shape-valid MLP bundle matching the running feature schema and returns
// (path, rawBytes).
func writeValidBundle(t *testing.T, dir, version string) (string, []byte) {
	t.Helper()
	const hidden = 2
	d := behavior.FeatureDim
	b := mlserve.MLPBundle{
		Version: version, SchemaHash: behavior.SchemaHash(), FeatureDim: d, Hidden: hidden,
		Mean: make([]float32, d), Std: make([]float32, d),
		W1: make([][]float32, hidden), B1: make([]float32, hidden), W2: make([]float32, hidden),
		CalA: 1,
	}
	for i := range b.Std {
		b.Std[i] = 1
	}
	for h := range b.W1 {
		b.W1[h] = make([]float32, d)
	}
	raw, err := json.Marshal(b)
	if err != nil {
		t.Fatal(err)
	}
	p := filepath.Join(dir, version+".json")
	if err := os.WriteFile(p, raw, 0o600); err != nil {
		t.Fatal(err)
	}
	return p, raw
}

func resetMLSeam(t *testing.T) {
	t.Helper()
	t.Cleanup(func() {
		mlserve.Set(nil)
		mlserve.SetShadow(nil)
		mlserve.SetThresholdProvider(nil)
		mlserve.SetFeatureObserver(nil)
		mlserve.SetShadowObserver(nil)
	})
}

func TestSetupBehavioralML_AdmitsUnsignedWhenNoPubkey(t *testing.T) {
	resetMLSeam(t)
	p, _ := writeValidBundle(t, t.TempDir(), "v1")
	got := setupBehavioralML(mlFlags{bundle: p, fpBudget: 0.005})
	if got.ctrl == nil || got.bundles == nil {
		t.Fatal("a schema-valid bundle with no pubkey configured must be admitted")
	}
	if ver, _ := got.bundles.Active(); ver != "v1" {
		t.Fatalf("active bundle identity must be recorded, got %q", ver)
	}
	if mlserve.Score(behavior.FeatureVector(make([]float32, behavior.FeatureDim))).Abstain {
		t.Fatal("the admitted model must be serving (not abstaining)")
	}
}

func TestSetupBehavioralML_RefusesSignatureRequiredButMissing(t *testing.T) {
	resetMLSeam(t)
	dir := t.TempDir()
	p, _ := writeValidBundle(t, dir, "v1")
	// configure a pubkey → signature becomes REQUIRED; provide none.
	pub, _, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	der, _ := x509.MarshalPKIXPublicKey(pub)
	keyPath := filepath.Join(dir, "pub.pem")
	os.WriteFile(keyPath, pem.EncodeToMemory(&pem.Block{Type: "PUBLIC KEY", Bytes: der}), 0o600)

	got := setupBehavioralML(mlFlags{bundle: p, pubkey: keyPath, fpBudget: 0.005})
	if got.ctrl != nil || got.bundles != nil {
		t.Fatal("with a pubkey set and no signature, the served bundle must be REFUSED (fail to heuristics)")
	}
	if !mlserve.Score(behavior.FeatureVector(make([]float32, behavior.FeatureDim))).Abstain {
		t.Fatal("a refused bundle must leave the seam abstaining (heuristics baseline)")
	}
}

func TestSetupBehavioralML_AdmitsProperlySignedBundle(t *testing.T) {
	resetMLSeam(t)
	dir := t.TempDir()
	p, raw := writeValidBundle(t, dir, "signed-v1")
	pub, priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	der, _ := x509.MarshalPKIXPublicKey(pub)
	keyPath := filepath.Join(dir, "pub.pem")
	os.WriteFile(keyPath, pem.EncodeToMemory(&pem.Block{Type: "PUBLIC KEY", Bytes: der}), 0o600)
	sigPath := filepath.Join(dir, "sig.bin")
	os.WriteFile(sigPath, ed25519.Sign(priv, raw), 0o600)

	got := setupBehavioralML(mlFlags{bundle: p, pubkey: keyPath, sig: sigPath, fpBudget: 0.005})
	if got.ctrl == nil || got.bundles == nil {
		t.Fatal("a correctly-signed bundle must be admitted")
	}
	if ver, _ := got.bundles.Active(); ver != "signed-v1" {
		t.Fatalf("active identity must be the signed bundle, got %q", ver)
	}
}

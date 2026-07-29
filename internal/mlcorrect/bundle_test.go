package mlcorrect

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

// writeBundle writes a shape-valid MLP bundle to a temp file and returns (path, fileBytes). When
// schemaHash is "", the running engine's schema is used (a stageable bundle); pass a bogus value to
// exercise the stale-schema guard.
func writeBundle(t *testing.T, version, schemaHash string) (string, []byte) {
	t.Helper()
	if schemaHash == "" {
		schemaHash = behavior.SchemaHash()
	}
	const hidden = 2
	dim := behavior.FeatureDim
	b := mlserve.MLPBundle{
		Version:    version,
		SchemaHash: schemaHash,
		FeatureDim: dim,
		Hidden:     hidden,
		Mean:       make([]float32, dim),
		Std:        make([]float32, dim),
		W1:         make([][]float32, hidden),
		B1:         make([]float32, hidden),
		W2:         make([]float32, hidden),
		B2:         0,
		CalA:       1,
		CalB:       0,
	}
	for i := range b.Std {
		b.Std[i] = 1
	}
	for h := 0; h < hidden; h++ {
		b.W1[h] = make([]float32, dim)
	}
	raw, err := json.Marshal(b)
	if err != nil {
		t.Fatalf("marshal bundle: %v", err)
	}
	path := filepath.Join(t.TempDir(), version+".json")
	if err := os.WriteFile(path, raw, 0o600); err != nil {
		t.Fatalf("write bundle: %v", err)
	}
	return path, raw
}

// resetSeam returns the mlserve seam to the abstaining default so global state does not leak between
// tests that promote.
func resetSeam(t *testing.T) {
	t.Helper()
	mlserve.Set(nil)
	t.Cleanup(func() { mlserve.Set(nil) })
}

func TestStage_AcceptsValidBundle(t *testing.T) {
	m, err := NewBundleManager(nil)
	if err != nil {
		t.Fatal(err)
	}
	path, _ := writeBundle(t, "v1", "")
	if err := m.Stage(path, "", nil); err != nil {
		t.Fatalf("valid bundle must stage: %v", err)
	}
	v, d, ok := m.Staged()
	if !ok || v != "v1" || d == "" {
		t.Fatalf("Staged() = (%q,%q,%v), want v1 with digest", v, d, ok)
	}
}

func TestStage_RejectsStaleSchema(t *testing.T) {
	m, _ := NewBundleManager(nil)
	path, _ := writeBundle(t, "stale", "deadbeefcafe")
	if err := m.Stage(path, "", nil); err == nil {
		t.Fatal("a bundle whose schema hash differs from the engine must be refused")
	}
}

func TestStage_DigestMismatch(t *testing.T) {
	m, _ := NewBundleManager(nil)
	path, raw := writeBundle(t, "v1", "")
	good := Digest(raw)
	if err := m.Stage(path, good, nil); err != nil {
		t.Fatalf("correct digest must pass: %v", err)
	}
	if err := m.Stage(path, "0000", nil); err == nil {
		t.Fatal("a wrong pinned digest must be refused")
	}
}

func TestStage_SignatureRequiredWhenKeyConfigured(t *testing.T) {
	pub, priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	der, err := x509.MarshalPKIXPublicKey(pub)
	if err != nil {
		t.Fatal(err)
	}
	pemBytes := pem.EncodeToMemory(&pem.Block{Type: "PUBLIC KEY", Bytes: der})
	m, err := NewBundleManager(pemBytes)
	if err != nil {
		t.Fatalf("configure pubkey: %v", err)
	}
	path, raw := writeBundle(t, "signed", "")

	// no signature → refused
	if err := m.Stage(path, "", nil); err == nil {
		t.Fatal("with a key configured, an unsigned bundle must be refused")
	}
	// wrong signature → refused
	if err := m.Stage(path, "", []byte("not-a-sig")); err == nil {
		t.Fatal("a bad signature must be refused")
	}
	// correct signature over the file bytes → accepted
	sig := ed25519.Sign(priv, raw)
	if err := m.Stage(path, "", sig); err != nil {
		t.Fatalf("a valid signature must stage: %v", err)
	}
}

func TestPromoteAndRollback(t *testing.T) {
	resetSeam(t)
	m, _ := NewBundleManager(nil)

	// promote v1
	p1, _ := writeBundle(t, "v1", "")
	if err := m.Stage(p1, "", nil); err != nil {
		t.Fatal(err)
	}
	if err := m.Promote(); err != nil {
		t.Fatal(err)
	}
	if got := mlserve.Default().BundleVersion(); got != "v1" {
		t.Fatalf("after promote, seam serves %q want v1", got)
	}

	// promote v2, previous becomes v1
	p2, _ := writeBundle(t, "v2", "")
	if err := m.Stage(p2, "", nil); err != nil {
		t.Fatal(err)
	}
	if err := m.Promote(); err != nil {
		t.Fatal(err)
	}
	if got := mlserve.Default().BundleVersion(); got != "v2" {
		t.Fatalf("after 2nd promote, seam serves %q want v2", got)
	}

	// rollback → v1
	if err := m.Rollback(); err != nil {
		t.Fatal(err)
	}
	if got := mlserve.Default().BundleVersion(); got != "v1" {
		t.Fatalf("after rollback, seam serves %q want v1", got)
	}

	// rollback again → abstaining default (no previous)
	if err := m.Rollback(); err != nil {
		t.Fatal(err)
	}
	if got := mlserve.Default().BundleVersion(); got != "none" {
		t.Fatalf("after 2nd rollback, seam must abstain, got %q", got)
	}
}

func TestPromote_RequiresStaged(t *testing.T) {
	m, _ := NewBundleManager(nil)
	if err := m.Promote(); err == nil {
		t.Fatal("Promote without a staged candidate must error")
	}
}

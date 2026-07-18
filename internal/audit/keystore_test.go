package audit

import (
	"crypto/ed25519"
	"path/filepath"
	"testing"
)

// WS8: the keystore round-trips key material under the correct passphrase and
// rejects a wrong one.
func TestKeystoreSealOpen(t *testing.T) {
	path := filepath.Join(t.TempDir(), "keys.sealed")
	m := KeyMaterial{SigningSeed: make([]byte, 32), HMACKey: []byte("hmac"), Vault: []byte(`{"keys":{}}`)}
	m.SigningSeed[0] = 7
	if err := SealKeys(path, "unseal-secret", m); err != nil {
		t.Fatal(err)
	}
	got, err := OpenKeys(path, "unseal-secret")
	if err != nil {
		t.Fatal(err)
	}
	if got.SigningSeed[0] != 7 || string(got.HMACKey) != "hmac" {
		t.Fatal("material did not round-trip")
	}
	if _, err := OpenKeys(path, "WRONG"); err == nil {
		t.Fatal("wrong passphrase must fail to unseal")
	}
}

// WS8: LoadOrCreateKeys creates fresh material on first boot and loads the SAME
// material on the next boot — a restart resumes the same chain identity.
func TestKeystoreLoadOrCreate(t *testing.T) {
	path := filepath.Join(t.TempDir(), "keys.sealed")
	m1, created1, err := LoadOrCreateKeys(path, "pw")
	if err != nil || !created1 {
		t.Fatalf("first boot should create: created=%v err=%v", created1, err)
	}
	m2, created2, err := LoadOrCreateKeys(path, "pw")
	if err != nil || created2 {
		t.Fatalf("second boot should load, not create: created=%v err=%v", created2, err)
	}
	if string(m1.SigningSeed) != string(m2.SigningSeed) || string(m1.HMACKey) != string(m2.HMACKey) {
		t.Fatal("restart must resume the SAME keys")
	}
}

// WS8: a persisted signing seed yields the SAME verifier public key across
// restarts — prior Signed Tree Heads stay verifiable.
func TestPersistedSeedStablePubkey(t *testing.T) {
	seed := make([]byte, ed25519.SeedSize)
	seed[1] = 42
	l1 := NewLog(Config{NodeID: "n", HMACKey: []byte("k"), SigningSeed: seed})
	l2 := NewLog(Config{NodeID: "n", HMACKey: []byte("k"), SigningSeed: seed})
	if string(l1.PublicKey()) != string(l2.PublicKey()) {
		t.Fatal("same seed must give the same STH public key across restarts")
	}
}

// WS8: the vault survives a restart — pseudonyms stay LINKABLE (no accidental
// mass shred) and shred tombstones persist, via Snapshot/LoadVault.
func TestVaultPersistence(t *testing.T) {
	v := NewVault()
	p := v.Pseudonymize("subjA", "1.2.3.4")
	v.Pseudonymize("subjB", "5.6.7.8")
	v.Shred("subjC") // a tombstone

	// "restart": snapshot -> seal -> open -> reload.
	restored := LoadVault(v.Snapshot())

	// subjA's pseudonym must be identical (same linkage key preserved).
	if restored.Pseudonymize("subjA", "1.2.3.4") != p {
		t.Fatal("restart must preserve the per-subject linkage key (else = accidental mass shred)")
	}
	// subjC stays shredded (tombstone persisted) — never re-minted.
	if restored.subjectKey("subjC", true) != nil {
		t.Fatal("shred tombstone must survive restart")
	}
}

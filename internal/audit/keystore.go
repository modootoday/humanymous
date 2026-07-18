package audit

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"encoding/json"
	"errors"
	"os"

	"golang.org/x/crypto/scrypt"
)

// keystore.go persists node key material across restarts (SoT-28 WS8). Without
// it, every boot regenerates the audit signing key, the HMAC key, and the vault
// linkage keys — silently invalidating every prior Signed Tree Head (the
// verifier pubkey changes) AND rendering every prior pseudonym unlinkable (an
// accidental mass crypto-shred). The keystore seals the material in a local file
// with scrypt-derived AES-256-GCM (an operator-supplied unseal secret), so a
// restart resumes the SAME chain identity and the SAME pseudonym linkage.
//
// This is the local self-contained stand-in for a KMS/HSM (SoT-22 §4); the
// load/rotate interface is the same.

// KeyMaterial is the persisted node identity.
type KeyMaterial struct {
	SigningSeed []byte `json:"signing_seed"` // Ed25519 seed (32B) for the STH key
	HMACKey     []byte `json:"hmac_key"`     // per-record HMAC key
	Vault       []byte `json:"vault"`        // serialized per-subject linkage keys (optional)
}

type sealedBlob struct {
	Salt  []byte `json:"salt"`
	Nonce []byte `json:"nonce"`
	CT    []byte `json:"ct"`
}

// scrypt parameters (interactive-strong).
const (
	scryptN = 1 << 15
	scryptR = 8
	scryptP = 1
)

// SealKeys encrypts and writes key material under the unseal passphrase.
func SealKeys(path, passphrase string, m KeyMaterial) error {
	plain, err := json.Marshal(m)
	if err != nil {
		return err
	}
	salt := make([]byte, 16)
	if _, err := rand.Read(salt); err != nil {
		return err
	}
	dk, err := scrypt.Key([]byte(passphrase), salt, scryptN, scryptR, scryptP, 32)
	if err != nil {
		return err
	}
	block, err := aes.NewCipher(dk)
	if err != nil {
		return err
	}
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return err
	}
	nonce := make([]byte, gcm.NonceSize())
	if _, err := rand.Read(nonce); err != nil {
		return err
	}
	ct := gcm.Seal(nil, nonce, plain, nil)
	blob, _ := json.Marshal(sealedBlob{Salt: salt, Nonce: nonce, CT: ct})
	return os.WriteFile(path, blob, 0o600)
}

// OpenKeys reads and decrypts key material.
func OpenKeys(path, passphrase string) (KeyMaterial, error) {
	raw, err := os.ReadFile(path)
	if err != nil {
		return KeyMaterial{}, err
	}
	var b sealedBlob
	if err := json.Unmarshal(raw, &b); err != nil {
		return KeyMaterial{}, err
	}
	dk, err := scrypt.Key([]byte(passphrase), b.Salt, scryptN, scryptR, scryptP, 32)
	if err != nil {
		return KeyMaterial{}, err
	}
	block, err := aes.NewCipher(dk)
	if err != nil {
		return KeyMaterial{}, err
	}
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return KeyMaterial{}, err
	}
	plain, err := gcm.Open(nil, b.Nonce, b.CT, nil)
	if err != nil {
		return KeyMaterial{}, errUnseal // wrong passphrase or tampered file
	}
	var m KeyMaterial
	if err := json.Unmarshal(plain, &m); err != nil {
		return KeyMaterial{}, err
	}
	return m, nil
}

// LoadOrCreateKeys opens the keystore, or creates+seals fresh material on first
// boot. `created` is true only when new material was generated.
func LoadOrCreateKeys(path, passphrase string) (m KeyMaterial, created bool, err error) {
	if _, statErr := os.Stat(path); statErr == nil {
		m, err = OpenKeys(path, passphrase)
		return m, false, err
	}
	seed := make([]byte, 32)
	hkey := make([]byte, 32)
	if _, err = rand.Read(seed); err != nil {
		return
	}
	if _, err = rand.Read(hkey); err != nil {
		return
	}
	m = KeyMaterial{SigningSeed: seed, HMACKey: hkey}
	if err = SealKeys(path, passphrase, m); err != nil {
		return
	}
	return m, true, nil
}

var errUnseal = errors.New("keystore: cannot unseal (wrong passphrase or corrupt file)")

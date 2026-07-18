package audit

import (
	"crypto/hmac"
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"sync"

	"golang.org/x/crypto/scrypt"
)

// pseudonym.go implements write-time pseudonymization with PER-SUBJECT linkage
// keys (SoT-18 §5). This deliberately does NOT use a single shared pepper: a
// shared pepper over low-entropy identifiers (IPv4 = 32 bits, small JA4/SNI/UA
// cardinality) is brute-forcible if the pepper leaks, and — critically — makes
// GDPR/PIPA crypto-shred impossible (deleting a vault row leaves HMAC(pepper,IP)
// still recoverable). With a per-subject key, destroying that one key provably
// severs re-identification for exactly that subject while the hash chain stays
// verifiable.
//
// This is an in-process reference Vault. Production keeps the keys in an
// envelope-encrypted store (KMS) under dual control (SoT-22 §4); the interface
// is the same.

// Vault issues and holds per-subject linkage keys and performs crypto-shred.
type Vault struct {
	mu       sync.Mutex
	keys     map[string][]byte   // subjectID -> 32-byte linkage key
	shredded map[string]struct{} // subjects whose keys were destroyed (never re-minted)
	index    map[string]string   // handle (session pseudonym) -> subjectID (SoT-28 §6 reverse map)
	stretch  bool                // KDF-stretch pseudonyms (SoT-28 WS8 brute-force resistance)
}

// SetStretch enables scrypt key-stretching of pseudonyms (SoT-28 WS8). Bare HMAC
// over a low-entropy value (IPv4 = 2^32) is cheap to dictionary-invert if a
// linkage key leaks; a per-value KDF step raises each guess to a scrypt op while
// staying deterministic (same subject+value -> same pseudonym, preserving links).
func (v *Vault) SetStretch(on bool) *Vault {
	v.mu.Lock()
	v.stretch = on
	v.mu.Unlock()
	return v
}

// NewVault creates an empty re-identification vault.
func NewVault() *Vault {
	return &Vault{keys: map[string][]byte{}, shredded: map[string]struct{}{}, index: map[string]string{}}
}

// Link records a handle→subject mapping so an operator can request erasure by the
// pseudonym the console shows (never a raw client id, SoT-28 §6). This is the
// access-controlled re-identification index; it is destroyed on shred and is the
// re-identification data the design deliberately isolates.
func (v *Vault) Link(handle, subjectID string) {
	if handle == "" || subjectID == "" {
		return
	}
	v.mu.Lock()
	if _, shredded := v.shredded[subjectID]; !shredded {
		v.index[handle] = subjectID
	}
	v.mu.Unlock()
}

// Resolve maps a handle back to its subject id (audited re-identification).
func (v *Vault) Resolve(handle string) (string, bool) {
	v.mu.Lock()
	defer v.mu.Unlock()
	s, ok := v.index[handle]
	return s, ok
}

// subjectKey returns the subject's linkage key, minting one on first use. A
// nil return means the subject was crypto-shredded (key destroyed) — callers
// then emit a tombstone pseudonym that can never be linked back.
func (v *Vault) subjectKey(subjectID string, create bool) []byte {
	v.mu.Lock()
	defer v.mu.Unlock()
	if k, ok := v.keys[subjectID]; ok {
		return k
	}
	if _, shredded := v.shredded[subjectID]; shredded || !create {
		return nil
	}
	k := make([]byte, 32)
	_, _ = rand.Read(k)
	v.keys[subjectID] = k
	return k
}

// Pseudonymize returns HMAC(K_subj, value) as hex, keyed to the subject. Values
// for the same subject are linkable to each other but not to raw identifiers
// without the vault key. If the subject was erased, returns a stable tombstone.
func (v *Vault) Pseudonymize(subjectID, value string) string {
	k := v.subjectKey(subjectID, true)
	if k == nil {
		return "shredded:" + shortHash(subjectID)
	}
	v.mu.Lock()
	stretch := v.stretch
	v.mu.Unlock()
	if stretch {
		// Deterministic KDF-stretch: 32-byte output (64 hex, same shape as HMAC).
		dk, err := scrypt.Key(k, []byte(value), 1<<12, 8, 1, 32)
		if err == nil {
			return hex.EncodeToString(dk)
		}
	}
	m := hmac.New(sha256.New, k)
	m.Write([]byte(value))
	return hex.EncodeToString(m.Sum(nil))
}

// Shred destroys a subject's linkage key, permanently unlinking all their
// pseudonyms (GDPR Art.17 / PIPA erasure, SoT-18 §6). The hash chain and Merkle
// anchors are untouched and remain verifiable. Returns true if a key existed.
func (v *Vault) Shred(subjectID string) bool {
	v.mu.Lock()
	defer v.mu.Unlock()
	_, existed := v.keys[subjectID]
	delete(v.keys, subjectID)
	v.shredded[subjectID] = struct{}{}
	// Purge the reverse index so the subject is no longer resolvable (SoT-28 §6).
	for h, s := range v.index {
		if s == subjectID {
			delete(v.index, h)
		}
	}
	return existed
}

func shortHash(s string) string {
	h := sha256.Sum256([]byte(s))
	return hex.EncodeToString(h[:6])
}

// Snapshot serializes the vault's linkage keys and shred tombstones so they can
// be sealed into the keystore and survive a restart (SoT-28 WS8). Without this a
// restart would lose every per-subject key = accidental mass crypto-shred.
func (v *Vault) Snapshot() []byte {
	v.mu.Lock()
	defer v.mu.Unlock()
	type snap struct {
		Keys     map[string]string `json:"keys"`
		Shredded []string          `json:"shredded"`
		Index    map[string]string `json:"index"`
	}
	s := snap{Keys: map[string]string{}, Index: map[string]string{}}
	for id, k := range v.keys {
		s.Keys[id] = hex.EncodeToString(k)
	}
	for id := range v.shredded {
		s.Shredded = append(s.Shredded, id)
	}
	for h, id := range v.index {
		s.Index[h] = id
	}
	b, _ := json.Marshal(s)
	return b
}

// LoadVault rebuilds a vault from a Snapshot (nil/empty => fresh vault).
func LoadVault(blob []byte) *Vault {
	v := NewVault()
	if len(blob) == 0 {
		return v
	}
	var s struct {
		Keys     map[string]string `json:"keys"`
		Shredded []string          `json:"shredded"`
		Index    map[string]string `json:"index"`
	}
	if json.Unmarshal(blob, &s) != nil {
		return v
	}
	for id, hk := range s.Keys {
		if k, err := hex.DecodeString(hk); err == nil {
			v.keys[id] = k
		}
	}
	for _, id := range s.Shredded {
		v.shredded[id] = struct{}{}
	}
	for h, id := range s.Index {
		v.index[h] = id
	}
	return v
}

// Package attestation is a minimal self-hosted attestation issuer (SoT-36 §2, the
// axis-① upgrade). Where PoW is a CPU tax anyone can pay, an attestation token is
// rate-limited at ISSUANCE per fingerprint, so the crypto axis costs an IDENTITY /
// rate budget — not just CPU. A flooding fingerprint runs out of tokens and can no
// longer satisfy the crypto axis, closing the one-shot-per-fresh-cookie bypass that
// PoW leaves open (a bot rotates cookies but keeps one JA4|subnet fingerprint).
//
// This is a DEMO issuer: a keyed HMAC bound to (fingerprint, challenge nonce, time
// window). Binding to the one-time challenge nonce makes each token single-use for
// exactly one challenge instance, so a bot cannot mint once and replay across many
// fresh cookies. A production deployment would use unlinkable Privacy Pass / PAT
// (blind-signature VOPRF) so the token cannot be correlated to its issuance; here we
// keep a simple bound MAC to demonstrate the rate-limited-issuance property.
package attestation

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
)

// Window is the seconds per attestation validity bucket.
const Window = 60

// mac derives the token for a (subject, nonce, window) triple. subject is the STABLE
// identity the token is bound to (the session id) — not the request fingerprint, which
// varies across TCP connections (issuance and redemption may use different sockets).
// Issuance is rate-limited by fingerprint at the caller; the token itself binds to the
// session so it verifies regardless of which connection redeems it.
func mac(masterKey []byte, subject, nonce string, window uint64) string {
	m := hmac.New(sha256.New, masterKey)
	m.Write([]byte("attest"))
	m.Write([]byte(subject))
	m.Write([]byte("|"))
	m.Write([]byte(nonce))
	m.Write([]byte("|"))
	var b [8]byte
	binary.BigEndian.PutUint64(b[:], window)
	m.Write(b[:])
	return hex.EncodeToString(m.Sum(nil))
}

// Issue mints a token bound to the subject (session id) + challenge nonce + window.
func Issue(masterKey []byte, subject, nonce string, window uint64) string {
	return mac(masterKey, subject, nonce, window)
}

// Verify checks a token against the subject + nonce, accepting the current window or
// its immediate neighbours (clock skew / issuance-then-solve straddle).
func Verify(masterKey []byte, subject, nonce, token string, current uint64) bool {
	if token == "" {
		return false
	}
	for _, w := range []uint64{current, current - 1, current + 1} {
		if hmac.Equal([]byte(token), []byte(mac(masterKey, subject, nonce, w))) {
			return true
		}
	}
	return false
}

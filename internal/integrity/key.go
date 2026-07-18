// Package integrity implements the Rotating Integrity Token (RIT) anti-tamper
// scheme from sots/07-request-integrity.md. The server issues a rotating seed
// per response; the client signs a canonical view of its request with it, and
// the server re-derives and verifies. Header/fetch tampering breaks the HMAC.
//
// The canonical serialization (canonical.go) and token math (token.go) are
// platform-independent so the WASM client and the server share identical rules.
package integrity

import (
	"crypto/hmac"
	"crypto/sha256"
)

// SessionKey derives the per-session secret K_s = HMAC(masterKey, sessionId).
// The master key never leaves the server; K_s is used to seed the rotation.
func SessionKey(masterKey []byte, sessionID string) []byte {
	m := hmac.New(sha256.New, masterKey)
	m.Write([]byte(sessionID))
	return m.Sum(nil)
}

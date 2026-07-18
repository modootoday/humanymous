package integrity

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/binary"
)

// seed.go implements the rotating seed chain (SoT-07 §2.1). The server issues
// seed_n each response; the client signs request n+1 with seed_n. Seeds chain
// off K_s and the counter so a captured seed can't be projected forward without
// the session key.

// Seed derives seed_n = HMAC(K_s, "rit-seed" || n).
func Seed(sessionKey []byte, n uint64) []byte {
	m := hmac.New(sha256.New, sessionKey)
	m.Write([]byte("rit-seed"))
	var nb [8]byte
	binary.BigEndian.PutUint64(nb[:], n)
	m.Write(nb[:])
	return m.Sum(nil)
}

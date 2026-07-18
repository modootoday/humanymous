package integrity

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"encoding/binary"
)

// token.go computes and verifies the RIT token (SoT-07 §2.2). The token binds
// the canonical request to the rotating seed, the request counter n and the
// time bucket tb. Both client and server run ComputeToken; the server compares
// with constant time.

// ComputeToken returns base64url(HMAC(seed, canonical || n || tb)).
func ComputeToken(seed []byte, canonical string, n uint64, tb uint64) string {
	m := hmac.New(sha256.New, seed)
	m.Write([]byte(canonical))
	var nb [8]byte
	binary.BigEndian.PutUint64(nb[:], n)
	m.Write(nb[:])
	var tbb [8]byte
	binary.BigEndian.PutUint64(tbb[:], tb)
	m.Write(tbb[:])
	return base64.RawURLEncoding.EncodeToString(m.Sum(nil))
}

// VerifyToken recomputes the expected token and compares in constant time.
func VerifyToken(seed []byte, canonical string, n, tb uint64, token string) bool {
	expect := ComputeToken(seed, canonical, n, tb)
	return hmac.Equal([]byte(expect), []byte(token))
}

// TimeBucket returns floor(unixSeconds / window). The caller supplies the time
// so this package stays clock-free and unit-testable (SoT-05 §8 determinism).
func TimeBucket(unixSeconds, window int64) uint64 {
	if window <= 0 {
		window = 10
	}
	if unixSeconds < 0 {
		unixSeconds = 0
	}
	return uint64(unixSeconds / window)
}

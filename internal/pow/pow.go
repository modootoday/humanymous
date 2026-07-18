// Package pow implements a stateless hashcash-style proof-of-work challenge
// (SoT-13): the server issues a difficulty target to a suspicious session; the
// client must find a nonce so that SHA-256(seed||nonce) has `difficulty` leading
// zero bits, proving CPU work. This raises the per-request cost of mass
// automation. The seed is HMAC-bound to the session + time bucket, so challenges
// are unforgeable and verifiable without server-side storage.
package pow

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"strconv"
	"strings"
)

// Window is the seconds per PoW time bucket (challenge validity).
const Window = 120

// Challenge is what the server sends to the client.
type Challenge struct {
	Seed       string `json:"seed"`       // hex HMAC seed
	Difficulty int    `json:"difficulty"` // required leading zero bits
	Bucket     uint64 `json:"bucket"`     // time bucket
}

// seedBytes derives the HMAC-bound seed for a session + bucket.
func seedBytes(masterKey []byte, sessionID string, bucket uint64) []byte {
	m := hmac.New(sha256.New, masterKey)
	m.Write([]byte("pow"))
	m.Write([]byte(sessionID))
	var b [8]byte
	binary.BigEndian.PutUint64(b[:], bucket)
	m.Write(b[:])
	return m.Sum(nil)
}

// Issue builds a challenge for a session at the given time bucket. Difficulty
// scales with suspicion (caller passes a risk-derived value).
func Issue(masterKey []byte, sessionID string, difficulty int, bucket uint64) Challenge {
	return Challenge{
		Seed:       hex.EncodeToString(seedBytes(masterKey, sessionID, bucket)),
		Difficulty: difficulty,
		Bucket:     bucket,
	}
}

// Verify checks a solution: the client presents (bucket, nonce). The server
// re-derives the seed and checks the leading-zero-bit target. bucket must be
// within +-1 of the current bucket.
func Verify(masterKey []byte, sessionID string, difficulty int, bucket, currentBucket uint64, nonce string) bool {
	if !withinBucket(bucket, currentBucket, 1) {
		return false
	}
	seed := seedBytes(masterKey, sessionID, bucket)
	return meetsTarget(seed, nonce, difficulty)
}

// meetsTarget reports whether SHA-256(seed || nonce) has >= difficulty leading
// zero bits.
func meetsTarget(seed []byte, nonce string, difficulty int) bool {
	h := sha256.New()
	h.Write(seed)
	h.Write([]byte(nonce))
	sum := h.Sum(nil)
	return leadingZeroBits(sum) >= difficulty
}

// leadingZeroBits counts the leading zero bits of a byte slice.
func leadingZeroBits(b []byte) int {
	n := 0
	for _, by := range b {
		if by == 0 {
			n += 8
			continue
		}
		for bit := 7; bit >= 0; bit-- {
			if by&(1<<uint(bit)) == 0 {
				n++
			} else {
				return n
			}
		}
		return n
	}
	return n
}

// Solve finds a nonce meeting the target (used by tests and the client-parity
// solver). Bounded by maxIters to avoid runaway on absurd difficulty.
func Solve(seedHex string, difficulty, maxIters int) (string, bool) {
	seed, err := hex.DecodeString(seedHex)
	if err != nil {
		return "", false
	}
	for i := 0; i < maxIters; i++ {
		nonce := strconv.Itoa(i)
		if meetsTarget(seed, nonce, difficulty) {
			return nonce, true
		}
	}
	return "", false
}

// DifficultyFor maps a risk score (0..100) to a PoW difficulty in bits. Higher
// suspicion => more CPU work. Kept modest so humans never notice (SoT-13 §FP).
func DifficultyFor(risk float64) int {
	// Difficulty in leading zero BITS. Values kept demo-friendly for a Web-Crypto
	// (async subtle.digest) client solver; a production deployment with a native
	// or WASM solver would raise these (SoT-13 §2). Real hardware solves these in
	// well under a second, but the per-request cost taxes mass automation.
	switch {
	case risk >= 70:
		return 17 // ~130k hashes
	case risk >= 50:
		return 16 // ~65k hashes
	case risk >= 30:
		return 14 // ~16k hashes, well under 1s
	default:
		return 0 // no challenge for trusted sessions
	}
}

// Encode/Decode a challenge as a compact header value "seed:difficulty:bucket".
func (c Challenge) Encode() string {
	return fmt.Sprintf("%s:%d:%d", c.Seed, c.Difficulty, c.Bucket)
}

// ParseSolution parses the "bucket:nonce" header the client returns.
func ParseSolution(s string) (bucket uint64, nonce string, ok bool) {
	parts := strings.SplitN(s, ":", 2)
	if len(parts) != 2 {
		return 0, "", false
	}
	b, err := strconv.ParseUint(parts[0], 10, 64)
	if err != nil {
		return 0, "", false
	}
	return b, parts[1], true
}

func withinBucket(a, b, tol uint64) bool {
	if a > b {
		return a-b <= tol
	}
	return b-a <= tol
}

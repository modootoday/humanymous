// Package pass implements the server side of humanymous Pass (SoT-36): the
// accessible interactive challenge served on a CHALLENGE verdict. Canonical
// mechanic: 3-ROW ALIGNMENT. Three cyclic character rows, each with one key
// character; the user slides each row so its key sits in the centre column.
//
// The puzzle answer is intentionally derivable (accessibility: a blind/keyboard
// user must be able to solve it, so a bot that reads the DOM can too — SoT-36 §1).
// The security is NOT the puzzle: it is the real-event proof of the input + the
// attestation/engine fusion (verified in the handler). This file owns only the
// deterministic, seed-reproducible generation + the alignment check.
package pass

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/binary"
)

// alphabet is ambiguity-pruned alphanumeric (no 0/O, 1/I/l) for legibility + audio.
const alphabet = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789"

// Rows is the fixed number of rows in the mechanic.
const Rows = 3

// Row is one cyclic strip: its characters (public, rendered) and the index of its
// key character (public, highlighted/announced).
type Row struct {
	Chars    []string `json:"chars"`
	KeyIndex int      `json:"keyIndex"`
}

// Challenge is the public puzzle sent to the client (SoT-36 §4). No hidden answer.
type Challenge struct {
	N      int   `json:"n"`      // cells per row
	Center int   `json:"center"` // target column (0-based)
	Rows   []Row `json:"rows"`
}

// deriveSeed binds a per-instance 32-byte seed to session + time bucket; the master
// key never leaves the server.
func deriveSeed(masterKey []byte, sessionID string, bucket uint64) [32]byte {
	m := hmac.New(sha256.New, masterKey)
	var b [8]byte
	binary.BigEndian.PutUint64(b[:], bucket)
	m.Write([]byte("pass|"))
	m.Write([]byte(sessionID))
	m.Write([]byte{'|'})
	m.Write(b[:])
	var out [32]byte
	copy(out[:], m.Sum(nil))
	return out
}

// rng is a deterministic splitmix64 stream so an instance is fully reproducible.
type rng struct{ s uint64 }

func newRNG(seed [32]byte) *rng { return &rng{s: binary.BigEndian.Uint64(seed[:8]) | 1} }

func (r *rng) next() uint64 {
	r.s += 0x9E3779B97F4A7C15
	z := r.s
	z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9
	z = (z ^ (z >> 27)) * 0x94D049BB133111EB
	return z ^ (z >> 31)
}

func (r *rng) intn(n int) int {
	if n <= 0 {
		return 0
	}
	return int(r.next() % uint64(n))
}

// dims maps difficulty 0..3 to the row length N (9,11,13,15) and the centre column.
// Longer rows = more decoys and a wider search, i.e. harder for a suspicious session.
func dims(difficulty int) (n, center int) {
	if difficulty < 0 {
		difficulty = 0
	}
	if difficulty > 3 {
		difficulty = 3
	}
	n = 9 + difficulty*2
	return n, n / 2
}

// Generate builds the public 3-row alignment challenge from a seed.
func Generate(masterKey []byte, sessionID string, bucket uint64, difficulty int) Challenge {
	seed := deriveSeed(masterKey, sessionID, bucket)
	r := newRNG(seed)
	n, center := dims(difficulty)
	ch := Challenge{N: n, Center: center, Rows: make([]Row, 0, Rows)}
	for i := 0; i < Rows; i++ {
		chars := make([]string, n)
		for j := 0; j < n; j++ {
			chars[j] = string(alphabet[r.intn(len(alphabet))])
		}
		// key not already in the centre column (the user must move at least this row).
		key := r.intn(n)
		ch.Rows = append(ch.Rows, Row{Chars: chars, KeyIndex: key})
	}
	return ch
}

// Verify checks that the submitted per-row offsets align every key to the centre
// column. offsets[i] is how many cells row i was shifted; the key lands at
// (keyIndex+offset) mod N. TTL bounds staleness. This is the alignment gate ONLY —
// the handler additionally requires the real-event proof (SoT-36 §5) + attestation.
func Verify(masterKey []byte, sessionID string, bucket, currentBucket uint64, difficulty int, offsets []int) bool {
	if currentBucket < bucket || currentBucket-bucket > 2 { // TTL
		return false
	}
	if len(offsets) != Rows {
		return false
	}
	ch := Generate(masterKey, sessionID, bucket, difficulty)
	for i, row := range ch.Rows {
		col := ((row.KeyIndex+offsets[i])%ch.N + ch.N) % ch.N
		if col != ch.Center {
			return false
		}
	}
	return true
}

// SolutionOffsets returns the canonical offsets that solve an instance — used by
// tests and by the red harness's oracle (a bot can compute these; the security is
// the real-event proof, not the answer).
func SolutionOffsets(masterKey []byte, sessionID string, bucket uint64, difficulty int) []int {
	ch := Generate(masterKey, sessionID, bucket, difficulty)
	out := make([]int, len(ch.Rows))
	for i, row := range ch.Rows {
		out[i] = ((ch.Center-row.KeyIndex)%ch.N + ch.N) % ch.N
	}
	return out
}

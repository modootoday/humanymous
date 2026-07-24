// SPDX-License-Identifier: Apache-2.0

// Package origincloak implements the origin-cloaking token that humanymous Gate
// attaches to every request it forwards upstream, in the header X-Hmny-Origin-Auth.
//
// It exists so an origin application can VERIFY that a request actually came through
// Gate (and reject direct hits that bypass detection). Gate's own enforcement uses the
// identical construction internally (internal/gate); this is the public, importable copy
// an origin written in Go can use directly — the origin side is exactly:
//
//	if !origincloak.Valid(key, r.Header.Get("X-Hmny-Origin-Auth"), time.Now()) {
//		http.Error(w, "direct origin access is not allowed", http.StatusMisdirectedRequest) // 421
//		return
//	}
//
// The token is HMAC-SHA256(key, "hmny-origin|"+epoch), hex-encoded, where the epoch is a
// one-hour wall-clock bucket. Gate and the origin derive the epoch independently from the
// clock (no coordination), so both machines MUST keep reasonable time (NTP). The verifier
// accepts a ±1-bucket grace, so a leaked token is valid for at most ~2 hours, and an
// hour-boundary clock skew in either direction never blackholes traffic.
//
// key is the shared secret set on Gate with -origin-key (hex on the CLI; here it is the
// raw key bytes — for a hex CLI value, the key bytes are []byte(hexString), matching Gate).
package origincloak

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"strconv"
	"time"
)

// EpochWindow is the length of one origin-cloaking epoch bucket. A token is valid for at
// most one window plus the ±1-bucket grace (~2h total).
const EpochWindow = time.Hour

// Header is the request header Gate attaches the origin-cloaking token to.
const Header = "X-Hmny-Origin-Auth"

// Epoch returns the wall-clock origin epoch for t: "e" + floor(t.Unix()/3600). Gate and the
// origin compute this independently from their own clocks.
func Epoch(t time.Time) string {
	return "e" + strconv.FormatInt(t.Unix()/int64(EpochWindow/time.Second), 10)
}

// EpochGrace returns the epochs an origin MUST accept — next, current, and previous — to
// tolerate a bucket boundary crossed in EITHER direction. The grace is symmetric on purpose:
// if Gate's clock is ahead across an hour boundary it emits the NEXT bucket, so a past-only
// grace would reject every proxied request. ±1 bucket tolerates skew either way.
func EpochGrace(t time.Time) []string {
	return []string{
		Epoch(t.Add(EpochWindow)),
		Epoch(t),
		Epoch(t.Add(-EpochWindow)),
	}
}

// Token computes the X-Hmny-Origin-Auth value for a given epoch:
// hex(HMAC-SHA256(key, "hmny-origin|"+epoch)). This is what Gate sends; an origin does not
// normally need it (use Valid), but it is exposed for testing and non-Go re-implementations.
func Token(key []byte, epoch string) string {
	m := hmac.New(sha256.New, key)
	m.Write([]byte("hmny-origin|" + epoch))
	return hex.EncodeToString(m.Sum(nil))
}

// Valid reports whether header is a valid X-Hmny-Origin-Auth for key at time t, using the
// ±1-bucket grace and a constant-time comparison. This is the entire origin-side check:
// return 421 (StatusMisdirectedRequest) when it is false, then strip the header before
// handling the request.
func Valid(key []byte, header string, t time.Time) bool {
	return ValidAt(key, header, EpochGrace(t)...)
}

// ValidAt reports whether header matches Token(key, e) for any e in epochs (constant-time).
// Use Valid unless you need to supply a custom epoch set.
func ValidAt(key []byte, header string, epochs ...string) bool {
	if header == "" {
		return false
	}
	for _, e := range epochs {
		if hmac.Equal([]byte(header), []byte(Token(key, e))) {
			return true
		}
	}
	return false
}

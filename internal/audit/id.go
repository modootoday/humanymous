package audit

import (
	"crypto/rand"
	"encoding/hex"
	"time"
)

// id.go mints the per-record event id (PLAN-07 R15). Every record carries a
// UUIDv7 (RFC 9562): a 48-bit big-endian Unix-millisecond timestamp followed by 74
// random bits, with the version (7) and variant (10) fields set. The embedded
// timestamp makes ids naturally k-sortable for display and cross-system
// correlation — but the id is NEVER the chain's ordering or dedupe key (that is the
// monotonic seq). It is stamped once, at append time, into the canonical body.

// newEventID returns a UUIDv7 string for time t. A CSPRNG failure fails closed: an
// audit record must never carry a predictable or duplicated identifier.
func newEventID(t time.Time) string {
	var b [16]byte
	ms := uint64(t.UnixMilli())
	b[0] = byte(ms >> 40)
	b[1] = byte(ms >> 32)
	b[2] = byte(ms >> 24)
	b[3] = byte(ms >> 16)
	b[4] = byte(ms >> 8)
	b[5] = byte(ms)
	if _, err := rand.Read(b[6:]); err != nil {
		panic("audit: CSPRNG failed minting event id (fail-closed): " + err.Error())
	}
	b[6] = (b[6] & 0x0f) | 0x70 // version 7
	b[8] = (b[8] & 0x3f) | 0x80 // variant 10 (RFC 9562)
	return uuidString(b)
}

// uuidString renders the 16 bytes as canonical 8-4-4-4-12 hex.
func uuidString(b [16]byte) string {
	var dst [36]byte
	hex.Encode(dst[0:8], b[0:4])
	dst[8] = '-'
	hex.Encode(dst[9:13], b[4:6])
	dst[13] = '-'
	hex.Encode(dst[14:18], b[6:8])
	dst[18] = '-'
	hex.Encode(dst[19:23], b[8:10])
	dst[23] = '-'
	hex.Encode(dst[24:36], b[10:16])
	return string(dst[:])
}

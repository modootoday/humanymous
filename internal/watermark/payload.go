// Package watermark implements per-session forensic watermarking of served
// resources for leak tracing (SoT-08). A carrier = tagId || HMAC(masterKey,
// sid||assetId||bucket) is injected into a resource via a MIME-specific codec;
// on a leaked resource, the carrier is extracted and looked up in the ledger to
// identify the leaking session. The HMAC makes the tag unforgeable.
package watermark

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/binary"
	"fmt"
)

// TagLen is the truncated HMAC length carried in the watermark (bytes).
const TagLen = 10

// CarrierLen is tagId(4) + tag(TagLen).
const CarrierLen = 4 + TagLen

// Payload identifies a watermark issuance.
type Payload struct {
	TagID     uint32 // ledger lookup key
	SessionID string
	AssetID   string
	Bucket    uint64 // low-resolution issue time (e.g. unix day)
}

// Carrier builds the raw bytes embedded into a resource: tagId (big-endian) ++
// truncated HMAC(masterKey, sid||assetId||bucket).
func Carrier(masterKey []byte, p Payload) []byte {
	out := make([]byte, 0, CarrierLen)
	var idb [4]byte
	binary.BigEndian.PutUint32(idb[:], p.TagID)
	out = append(out, idb[:]...)
	out = append(out, tag(masterKey, p)...)
	return out
}

// tag computes the truncated HMAC binding the payload to the server secret.
func tag(masterKey []byte, p Payload) []byte {
	m := hmac.New(sha256.New, masterKey)
	m.Write([]byte(p.SessionID))
	m.Write([]byte{0})
	m.Write([]byte(p.AssetID))
	m.Write([]byte{0})
	var bb [8]byte
	binary.BigEndian.PutUint64(bb[:], p.Bucket)
	m.Write(bb[:])
	return m.Sum(nil)[:TagLen]
}

// SplitCarrier extracts the tagId and tag bytes from a recovered carrier.
func SplitCarrier(carrier []byte) (tagID uint32, tagBytes []byte, err error) {
	if len(carrier) < CarrierLen {
		return 0, nil, fmt.Errorf("watermark: carrier too short (%d)", len(carrier))
	}
	tagID = binary.BigEndian.Uint32(carrier[:4])
	tagBytes = carrier[4:CarrierLen]
	return tagID, tagBytes, nil
}

// Verify recomputes the tag for a known payload and checks it against the
// recovered tag bytes (constant time) — proving the carrier is not forged.
func Verify(masterKey []byte, p Payload, tagBytes []byte) bool {
	return hmac.Equal(tag(masterKey, p), tagBytes)
}

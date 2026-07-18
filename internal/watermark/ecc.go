package watermark

import (
	"bytes"
	"encoding/binary"
	"errors"
	"hash/crc32"
)

// ecc.go frames a carrier with a magic marker, length and CRC32 so an extractor
// can locate it inside a resource and validate integrity (SoT-08 §3.4). This is
// the shared, channel-agnostic envelope; per-channel codecs embed/recover the
// framed bytes in a format-appropriate way.

var magic = []byte{0x48, 0x4D, 0x57, 0x4D} // "HMWM"

// ErrNoFrame is returned when no valid framed carrier is found.
var ErrNoFrame = errors.New("watermark: no valid frame found")

// Frame wraps a carrier: magic ++ len(uint16) ++ carrier ++ crc32(carrier).
func Frame(carrier []byte) []byte {
	buf := make([]byte, 0, len(magic)+2+len(carrier)+4)
	buf = append(buf, magic...)
	var l [2]byte
	binary.BigEndian.PutUint16(l[:], uint16(len(carrier)))
	buf = append(buf, l[:]...)
	buf = append(buf, carrier...)
	var c [4]byte
	binary.BigEndian.PutUint32(c[:], crc32.ChecksumIEEE(carrier))
	buf = append(buf, c[:]...)
	return buf
}

// Unframe scans data for the first valid frame and returns the carrier.
func Unframe(data []byte) ([]byte, error) {
	for i := 0; i+len(magic)+2 <= len(data); {
		j := bytes.Index(data[i:], magic)
		if j < 0 {
			break
		}
		off := i + j
		p := off + len(magic)
		if p+2 > len(data) {
			break
		}
		n := int(binary.BigEndian.Uint16(data[p : p+2]))
		p += 2
		if p+n+4 > len(data) {
			i = off + 1
			continue
		}
		carrier := data[p : p+n]
		want := binary.BigEndian.Uint32(data[p+n : p+n+4])
		if crc32.ChecksumIEEE(carrier) == want {
			return append([]byte(nil), carrier...), nil
		}
		i = off + 1
	}
	return nil, ErrNoFrame
}

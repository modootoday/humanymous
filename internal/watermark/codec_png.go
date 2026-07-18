package watermark

import (
	"bytes"
	"encoding/base32"
	"encoding/binary"
	"errors"
	"hash/crc32"
)

// codec_png.go embeds the carrier in a PNG tEXt chunk (SoT-08 §3.1 IMG-meta).
// The chunk is inserted before IEND and does not affect the rendered image.
// Keyword "hmwm", text = base32(framed). Decode reads the chunk back.

type pngCodec struct{}

func (pngCodec) Name() string { return "png-text" }

var pngSig = []byte{0x89, 'P', 'N', 'G', 0x0d, 0x0a, 0x1a, 0x0a}

const pngKeyword = "hmwm"

func (pngCodec) Encode(src, framed []byte) ([]byte, error) {
	if !bytes.HasPrefix(src, pngSig) {
		return nil, errors.New("watermark: not a PNG")
	}
	// Find IEND to insert our tEXt chunk before it.
	idx := findChunk(src, "IEND")
	if idx < 0 {
		return nil, errors.New("watermark: PNG has no IEND")
	}
	text := pngKeyword + "\x00" + base32.StdEncoding.EncodeToString(framed)
	chunk := buildChunk("tEXt", []byte(text))
	out := make([]byte, 0, len(src)+len(chunk))
	out = append(out, src[:idx]...)
	out = append(out, chunk...)
	out = append(out, src[idx:]...)
	return out, nil
}

func (pngCodec) Decode(src []byte) ([]byte, error) {
	pos := len(pngSig)
	for pos+8 <= len(src) {
		clen := int(binary.BigEndian.Uint32(src[pos : pos+4]))
		ctype := string(src[pos+4 : pos+8])
		dstart := pos + 8
		if dstart+clen+4 > len(src) {
			break
		}
		data := src[dstart : dstart+clen]
		if ctype == "tEXt" {
			if k, v, ok := bytes.Cut(data, []byte{0}); ok && string(k) == pngKeyword {
				if raw, err := base32.StdEncoding.DecodeString(string(v)); err == nil {
					if c, err := Unframe(raw); err == nil {
						return c, nil
					}
				}
			}
		}
		pos = dstart + clen + 4
	}
	return nil, ErrNoFrame
}

// findChunk returns the byte offset of the chunk with the given type.
func findChunk(png []byte, typ string) int {
	pos := len(pngSig)
	for pos+8 <= len(png) {
		clen := int(binary.BigEndian.Uint32(png[pos : pos+4]))
		if string(png[pos+4:pos+8]) == typ {
			return pos
		}
		pos += 8 + clen + 4
	}
	return -1
}

// buildChunk assembles a PNG chunk with CRC over type+data.
func buildChunk(typ string, data []byte) []byte {
	out := make([]byte, 0, 12+len(data))
	var l [4]byte
	binary.BigEndian.PutUint32(l[:], uint32(len(data)))
	out = append(out, l[:]...)
	out = append(out, typ...)
	out = append(out, data...)
	crc := crc32.NewIEEE()
	crc.Write([]byte(typ))
	crc.Write(data)
	var c [4]byte
	binary.BigEndian.PutUint32(c[:], crc.Sum32())
	out = append(out, c[:]...)
	return out
}

// (registration handled by the composite in codec_png_multi.go)

package watermark

import (
	"bytes"
	"encoding/binary"
	"image"
	"image/color"
	"image/png"
)

// codec_lsb.go embeds the carrier in the least-significant bits of a PNG's pixel
// data (SoT-08 §3.1 IMG-lsb). Because PNG is lossless, the LSB payload survives
// a metadata strip and a PNG re-encode — the robust channel that outlives the
// tEXt chunk. Layout: a 32-bit big-endian length in the first 32 LSBs, then the
// framed carrier bits.

type lsbCodec struct{}

func (lsbCodec) Name() string { return "png-lsb" }

func (lsbCodec) Encode(src, framed []byte) ([]byte, error) {
	img, err := png.Decode(bytes.NewReader(src))
	if err != nil {
		return nil, err
	}
	rgba := toRGBA(img)
	// bits to write: 32-bit length + framed payload.
	var hdr [4]byte
	binary.BigEndian.PutUint32(hdr[:], uint32(len(framed)))
	payload := append(hdr[:], framed...)
	if err := writeLSB(rgba, payload); err != nil {
		return nil, err
	}
	var out bytes.Buffer
	enc := png.Encoder{CompressionLevel: png.DefaultCompression}
	if err := enc.Encode(&out, rgba); err != nil {
		return nil, err
	}
	return out.Bytes(), nil
}

func (lsbCodec) Decode(src []byte) ([]byte, error) {
	img, err := png.Decode(bytes.NewReader(src))
	if err != nil {
		return nil, err
	}
	rgba := toRGBA(img)
	payload := readLSB(rgba, 4)
	if len(payload) < 4 {
		return nil, ErrNoFrame
	}
	n := int(binary.BigEndian.Uint32(payload))
	if n <= 0 || n > 1<<16 {
		return nil, ErrNoFrame
	}
	full := readLSB(rgba, 4+n)
	if len(full) < 4+n {
		return nil, ErrNoFrame
	}
	return Unframe(full[4 : 4+n])
}

// toRGBA returns a mutable *image.RGBA copy of img.
func toRGBA(img image.Image) *image.RGBA {
	b := img.Bounds()
	dst := image.NewRGBA(b)
	for y := b.Min.Y; y < b.Max.Y; y++ {
		for x := b.Min.X; x < b.Max.X; x++ {
			dst.Set(x, y, color.RGBAModel.Convert(img.At(x, y)))
		}
	}
	return dst
}

// writeLSB writes payload bits into the R,G,B LSBs (skips alpha to avoid making
// transparent pixels visibly shift). Returns an error if capacity is exceeded.
func writeLSB(img *image.RGBA, payload []byte) error {
	cap := capacityBits(img)
	if len(payload)*8 > cap {
		return ErrNoFrame // caller falls back to another channel
	}
	bitIdx := 0
	forEachChannel(img, func(p *uint8) bool {
		if bitIdx >= len(payload)*8 {
			return false
		}
		bit := (payload[bitIdx/8] >> uint(7-bitIdx%8)) & 1
		*p = (*p &^ 1) | bit
		bitIdx++
		return true
	})
	return nil
}

// readLSB reads nBytes worth of bits from the R,G,B LSBs.
func readLSB(img *image.RGBA, nBytes int) []byte {
	need := nBytes * 8
	out := make([]byte, nBytes)
	bitIdx := 0
	forEachChannel(img, func(p *uint8) bool {
		if bitIdx >= need {
			return false
		}
		bit := *p & 1
		out[bitIdx/8] |= bit << uint(7-bitIdx%8)
		bitIdx++
		return true
	})
	if bitIdx < need {
		return out[:bitIdx/8]
	}
	return out
}

// forEachChannel iterates R,G,B bytes (skipping A) calling fn until it returns
// false. Pixel order is row-major, matching write/read.
func forEachChannel(img *image.RGBA, fn func(*uint8) bool) {
	pix := img.Pix
	for i := 0; i+3 < len(pix); i += 4 {
		if !fn(&pix[i]) {
			return
		}
		if !fn(&pix[i+1]) {
			return
		}
		if !fn(&pix[i+2]) {
			return
		}
		// skip alpha (i+3)
	}
}

func capacityBits(img *image.RGBA) int {
	return (len(img.Pix) / 4) * 3
}

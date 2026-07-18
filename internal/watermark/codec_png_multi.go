package watermark

// codec_png_multi.go registers the composite PNG codec: it embeds the carrier
// in BOTH the tEXt metadata chunk (fast, fragile) and the pixel LSBs (robust,
// survives metadata strip + PNG re-encode) — the multi-layer principle of
// SoT-08 §3.4. Decode tries the cheap channel first, then the robust one.

type pngMultiCodec struct {
	text pngCodec
	lsb  lsbCodec
}

func (pngMultiCodec) Name() string { return "png-multi" }

func (c pngMultiCodec) Encode(src, framed []byte) ([]byte, error) {
	out, err := c.text.Encode(src, framed)
	if err != nil {
		out = src // tEXt failed; still try LSB on the original
	}
	if lsbOut, e := c.lsb.Encode(out, framed); e == nil {
		return lsbOut, nil
	}
	// LSB failed (e.g. image too small) — return the tEXt-only result.
	if err != nil {
		return nil, err
	}
	return out, nil
}

func (c pngMultiCodec) Decode(src []byte) ([]byte, error) {
	if carrier, err := c.text.Decode(src); err == nil {
		return carrier, nil
	}
	return c.lsb.Decode(src)
}

func init() { Register("image/png", pngMultiCodec{}) }

package watermark

import "bytes"

// codec_binary.go is the generic trailer codec (SoT-08 §3.3 BIN-trailer): it
// appends the framed carrier after the resource bytes. Safe for opaque binary /
// octet-stream payloads where trailing bytes are ignored. Registered as the
// "binary" fallback for unknown MIME types.

type binaryCodec struct{}

func (binaryCodec) Name() string { return "bin-trailer" }

func (binaryCodec) Encode(src, framed []byte) ([]byte, error) {
	out := make([]byte, 0, len(src)+len(framed))
	out = append(out, src...)
	out = append(out, framed...)
	return out, nil
}

func (binaryCodec) Decode(src []byte) ([]byte, error) {
	// The framed carrier begins at the last magic marker.
	if idx := bytes.LastIndex(src, magic); idx >= 0 {
		if c, err := Unframe(src[idx:]); err == nil {
			return c, nil
		}
	}
	return Unframe(src)
}

func init() { Register("binary", binaryCodec{}) }

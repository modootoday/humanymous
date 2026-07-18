package watermark

// inject.go routes a resource to its MIME codec and embeds a framed carrier
// (SoT-08 §4). Thin orchestration only (SRP): carrier math is in payload.go,
// framing in ecc.go, per-format embedding in the codecs.

// Apply watermarks src for the given MIME using payload p and master key.
// It returns the watermarked bytes and the codec/channel name used.
func Apply(masterKey []byte, mime string, src []byte, p Payload) (out []byte, channel string, err error) {
	c := codecFor(mime)
	framed := Frame(Carrier(masterKey, p))
	out, err = c.Encode(src, framed)
	if err != nil {
		// Fall back to the generic trailer so a resource is never served
		// unwatermarked silently (SoT-08 §3.4).
		fb := registry["binary"]
		if o2, e2 := fb.Encode(src, framed); e2 == nil {
			return o2, fb.Name(), nil
		}
		return nil, c.Name(), err
	}
	return out, c.Name(), nil
}

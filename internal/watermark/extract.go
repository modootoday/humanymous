package watermark

// extract.go recovers a carrier from a (possibly leaked) resource (SoT-08 §4).
// It tries the MIME-appropriate codec first, then every other codec, so a
// resource whose type is unknown or was re-encoded can still be traced.

// Recover extracts the carrier bytes and the codec/channel that succeeded.
func Recover(mime string, src []byte) (carrier []byte, channel string, err error) {
	// Preferred codec for the declared MIME.
	primary := codecFor(mime)
	if c, e := primary.Decode(src); e == nil {
		return c, primary.Name(), nil
	}
	// Fall back to trying all registered codecs.
	seen := map[string]bool{primary.Name(): true}
	for _, cod := range registry {
		if seen[cod.Name()] {
			continue
		}
		seen[cod.Name()] = true
		if c, e := cod.Decode(src); e == nil {
			return c, cod.Name(), nil
		}
	}
	return nil, "", ErrNoFrame
}

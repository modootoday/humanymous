package watermark

import (
	"bytes"
	"encoding/base32"
	"regexp"
)

// codec_text.go is the semantic-preserving codec for text/code resources
// (SoT-08 §3.2). It appends a trailing marker chosen by content so the parsed /
// executed result is unchanged:
//   - HTML/XML/SVG -> HTML comment  <!--!hmwm:B32!-->
//   - JS/CSS/other -> block comment /*!hmwm:B32!*/
//   - JSON         -> trailing whitespace bits (RFC 8259 allows trailing ws)
// Decode tries every channel and validates via the framed CRC.

type textCodec struct{}

func (textCodec) Name() string { return "text-comment" }

var b32 = base32.StdEncoding
var reMarker = regexp.MustCompile(`!hmwm:([A-Z2-7=]+)!`)

func (textCodec) Encode(src, framed []byte) ([]byte, error) {
	enc := b32.EncodeToString(framed)
	trimmed := bytes.TrimSpace(src)
	switch {
	case looksJSON(trimmed):
		return append(src, whitespaceEncode(framed)...), nil
	case looksMarkup(trimmed):
		return append(src, []byte("\n<!--!hmwm:"+enc+"!-->\n")...), nil
	default:
		return append(src, []byte("\n/*!hmwm:"+enc+"!*/\n")...), nil
	}
}

func (textCodec) Decode(src []byte) ([]byte, error) {
	// 1) comment marker (HTML or block comment).
	if m := reMarker.FindSubmatch(src); m != nil {
		if raw, err := b32.DecodeString(string(m[1])); err == nil {
			if c, err := Unframe(raw); err == nil {
				return c, nil
			}
		}
	}
	// 2) trailing whitespace channel (JSON).
	if raw := whitespaceDecode(src); raw != nil {
		if c, err := Unframe(raw); err == nil {
			return c, nil
		}
	}
	return nil, ErrNoFrame
}

func looksJSON(b []byte) bool {
	return len(b) > 0 && (b[0] == '{' || b[0] == '[')
}

func looksMarkup(b []byte) bool {
	return len(b) > 0 && b[0] == '<'
}

// whitespaceEncode renders bytes as a trailing run: newline then 8 bits/byte,
// 0->space 1->tab. This is insignificant whitespace for JSON.
func whitespaceEncode(data []byte) []byte {
	out := make([]byte, 0, 1+len(data)*8)
	out = append(out, '\n')
	for _, by := range data {
		for i := 7; i >= 0; i-- {
			if (by>>uint(i))&1 == 1 {
				out = append(out, '\t')
			} else {
				out = append(out, ' ')
			}
		}
	}
	return out
}

// whitespaceDecode reads a trailing run of space/tab bits back into bytes.
func whitespaceDecode(src []byte) []byte {
	// collect the trailing run of space/tab (ignoring the final newline group).
	i := len(src)
	var bits []byte
	for i > 0 {
		c := src[i-1]
		if c == ' ' || c == '\t' {
			bits = append(bits, c)
			i--
			continue
		}
		break
	}
	if len(bits) < 8 {
		return nil
	}
	// bits are in reverse order; reverse them.
	for l, r := 0, len(bits)-1; l < r; l, r = l+1, r-1 {
		bits[l], bits[r] = bits[r], bits[l]
	}
	n := len(bits) / 8 * 8
	out := make([]byte, 0, n/8)
	for j := 0; j+8 <= n; j += 8 {
		var by byte
		for k := 0; k < 8; k++ {
			by <<= 1
			if bits[j+k] == '\t' {
				by |= 1
			}
		}
		out = append(out, by)
	}
	return out
}

func init() { Register("text", textCodec{}) }

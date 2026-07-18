package watermark

import (
	"bytes"
	"errors"
)

// codec_wasm.go embeds the carrier in a WASM custom section (SoT-08 §3.3
// BIN-section). Custom sections (id 0) are ignored by execution and by module
// validation, so the module still runs and its code/data hashes are unchanged.
// Section name "hmwm", payload = framed carrier.

type wasmCodec struct{}

func (wasmCodec) Name() string { return "wasm-section" }

var wasmMagic = []byte{0x00, 0x61, 0x73, 0x6d} // "\0asm"
const wasmSectionName = "hmwm"

func (wasmCodec) Encode(src, framed []byte) ([]byte, error) {
	if !bytes.HasPrefix(src, wasmMagic) || len(src) < 8 {
		return nil, errors.New("watermark: not a WASM module")
	}
	// Custom section body = name_len(uleb) ++ name ++ payload.
	var body []byte
	body = appendULEB(body, uint32(len(wasmSectionName)))
	body = append(body, wasmSectionName...)
	body = append(body, framed...)
	// Section = id(0) ++ size(uleb) ++ body.
	sect := []byte{0x00}
	sect = appendULEB(sect, uint32(len(body)))
	sect = append(sect, body...)
	// Appending a custom section after all sections is valid.
	out := make([]byte, 0, len(src)+len(sect))
	out = append(out, src...)
	out = append(out, sect...)
	return out, nil
}

func (wasmCodec) Decode(src []byte) ([]byte, error) {
	if !bytes.HasPrefix(src, wasmMagic) {
		return nil, errors.New("watermark: not a WASM module")
	}
	pos := 8 // magic + version
	for pos < len(src) {
		id := src[pos]
		pos++
		size, n := readULEB(src[pos:])
		if n == 0 || pos+n+int(size) > len(src) {
			break
		}
		pos += n
		body := src[pos : pos+int(size)]
		pos += int(size)
		if id == 0 { // custom section
			nameLen, m := readULEB(body)
			if m > 0 && m+int(nameLen) <= len(body) {
				name := string(body[m : m+int(nameLen)])
				if name == wasmSectionName {
					if c, err := Unframe(body[m+int(nameLen):]); err == nil {
						return c, nil
					}
				}
			}
		}
	}
	return nil, ErrNoFrame
}

// appendULEB appends v as unsigned LEB128.
func appendULEB(b []byte, v uint32) []byte {
	for {
		c := byte(v & 0x7f)
		v >>= 7
		if v != 0 {
			c |= 0x80
		}
		b = append(b, c)
		if v == 0 {
			return b
		}
	}
}

// readULEB reads an unsigned LEB128, returning the value and bytes consumed.
func readULEB(b []byte) (uint32, int) {
	var result uint32
	var shift uint
	for i, c := range b {
		result |= uint32(c&0x7f) << shift
		if c&0x80 == 0 {
			return result, i + 1
		}
		shift += 7
		if shift >= 32 {
			break
		}
	}
	return 0, 0
}

func init() { Register("application/wasm", wasmCodec{}) }

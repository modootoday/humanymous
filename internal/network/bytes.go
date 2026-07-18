package network

import "encoding/binary"

// bytes.go holds a tiny cursor-based byte parser and length-prefixed vector
// readers shared by the ClientHello and HTTP/2 parsers (SRP: parsing mechanics
// live here, format semantics live in their own files).

type parser struct {
	b   []byte
	pos int
}

func newParser(b []byte) *parser { return &parser{b: b} }

func (p *parser) remaining() int { return len(p.b) - p.pos }

func (p *parser) skip(n int) bool {
	if p.remaining() < n {
		return false
	}
	p.pos += n
	return true
}

func (p *parser) u8() uint8 {
	v := p.b[p.pos]
	p.pos++
	return v
}

func (p *parser) u16() uint16 {
	v := binary.BigEndian.Uint16(p.b[p.pos:])
	p.pos += 2
	return v
}

// readVec8 reads a 1-byte length-prefixed vector.
func (p *parser) readVec8() ([]byte, bool) {
	if p.remaining() < 1 {
		return nil, false
	}
	n := int(p.b[p.pos])
	p.pos++
	if p.remaining() < n {
		return nil, false
	}
	out := p.b[p.pos : p.pos+n]
	p.pos += n
	return out, true
}

// readVec16 reads a 2-byte length-prefixed vector.
func (p *parser) readVec16() ([]byte, bool) {
	if p.remaining() < 2 {
		return nil, false
	}
	n := int(binary.BigEndian.Uint16(p.b[p.pos:]))
	p.pos += 2
	if p.remaining() < n {
		return nil, false
	}
	out := p.b[p.pos : p.pos+n]
	p.pos += n
	return out, true
}

// readVec8Body / readVec16Body read a length-prefixed vector from a standalone
// slice (used inside extension bodies).
func readVec8Body(b []byte) ([]byte, bool)  { return newParser(b).readVec8() }
func readVec16Body(b []byte) ([]byte, bool) { return newParser(b).readVec16() }

// bytesToU16 interprets a byte slice as big-endian uint16 values.
func bytesToU16(b []byte) []uint16 {
	out := make([]uint16, 0, len(b)/2)
	for i := 0; i+1 < len(b); i += 2 {
		out = append(out, binary.BigEndian.Uint16(b[i:]))
	}
	return out
}

// bytesToU8asU16 widens each byte to a uint16 (for ec_point_formats).
func bytesToU8asU16(b []byte) []uint16 {
	out := make([]uint16, len(b))
	for i, v := range b {
		out[i] = uint16(v)
	}
	return out
}

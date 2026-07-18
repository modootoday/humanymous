package network

import (
	"encoding/binary"
	"errors"
)

// clienthello.go defines a structured view of a TLS ClientHello and a minimal
// raw parser. The Go standard library's tls.ClientHelloInfo does not expose the
// extension list/order (golang/go#32936), which JA4_c needs, so we parse the
// raw handshake bytes captured off the connection (SoT-02 §Go capture strategy).

// ClientHello is the parsed, order-preserving view used to compute JA3/JA4.
type ClientHello struct {
	LegacyVersion uint16   // record/handshake legacy_version (e.g. 0x0303)
	SupportedVers []uint16 // from supported_versions extension (0x002b), if present
	CipherSuites  []uint16 // in wire order (GREASE included; strip per-fingerprint)
	Extensions    []uint16 // extension types in wire order (GREASE included)
	Curves        []uint16 // supported_groups (0x000a)
	PointFormats  []uint16 // ec_point_formats (0x000b)
	SigAlgs       []uint16 // signature_algorithms (0x000d) in wire order
	ALPN          []string // application_layer_protocol_negotiation (0x0010)
	SNIPresent    bool     // server_name (0x0000) present with a host_name
}

var errShort = errors.New("clienthello: truncated")

// ParseClientHello parses a raw TLS handshake ClientHello message body.
//
// input must start at the Handshake struct (msg_type=1 client_hello). If the
// bytes include the 5-byte TLS record header, call ParseRecord instead.
func ParseClientHello(b []byte) (*ClientHello, error) {
	// Handshake: type(1) length(3) then body.
	if len(b) < 4 || b[0] != 0x01 {
		return nil, errShort
	}
	hlen := int(b[1])<<16 | int(b[2])<<8 | int(b[3])
	body := b[4:]
	if len(body) < hlen {
		// tolerate: use what we have
		hlen = len(body)
	}
	body = body[:hlen]

	ch := &ClientHello{}
	p := newParser(body)

	if !p.skip(2) { // client_version
		return nil, errShort
	}
	ch.LegacyVersion = binary.BigEndian.Uint16(body[:2])
	if !p.skip(32) { // random
		return nil, errShort
	}
	// session_id
	sid, ok := p.readVec8()
	if !ok {
		return nil, errShort
	}
	_ = sid
	// cipher_suites <2..2^16-1>
	cs, ok := p.readVec16()
	if !ok {
		return nil, errShort
	}
	ch.CipherSuites = bytesToU16(cs)
	// compression_methods <1..2^8-1>
	if _, ok := p.readVec8(); !ok {
		return nil, errShort
	}
	// extensions <0..2^16-1> (optional in very old hellos)
	if p.remaining() == 0 {
		return ch, nil
	}
	extBlock, ok := p.readVec16()
	if !ok {
		return nil, errShort
	}
	parseExtensions(extBlock, ch)
	return ch, nil
}

// ParseRecord parses a ClientHello that still has the 5-byte TLS record header
// (content_type=22 handshake). It returns the parsed hello or an error.
func ParseRecord(rec []byte) (*ClientHello, error) {
	if len(rec) < 5 || rec[0] != 0x16 {
		return nil, errShort
	}
	recLen := int(binary.BigEndian.Uint16(rec[3:5]))
	if len(rec) < 5+recLen {
		recLen = len(rec) - 5
	}
	return ParseClientHello(rec[5 : 5+recLen])
}

// parseExtensions walks the extension block preserving type order.
func parseExtensions(block []byte, ch *ClientHello) {
	p := newParser(block)
	for p.remaining() >= 4 {
		etype := p.u16()
		edata, ok := p.readVec16()
		if !ok {
			return
		}
		ch.Extensions = append(ch.Extensions, etype)
		switch etype {
		case 0x0000: // server_name
			ch.SNIPresent = len(edata) > 0
		case 0x000a: // supported_groups
			if v, ok := readVec16Body(edata); ok {
				ch.Curves = bytesToU16(v)
			}
		case 0x000b: // ec_point_formats
			if v, ok := readVec8Body(edata); ok {
				ch.PointFormats = bytesToU8asU16(v)
			}
		case 0x000d: // signature_algorithms
			if v, ok := readVec16Body(edata); ok {
				ch.SigAlgs = bytesToU16(v)
			}
		case 0x0010: // ALPN
			ch.ALPN = parseALPN(edata)
		case 0x002b: // supported_versions
			if v, ok := readVec8Body(edata); ok {
				ch.SupportedVers = bytesToU16(v)
			}
		}
	}
}

// parseALPN parses the ALPN extension body: ProtocolNameList<2..2^16-1> of
// opaque ProtocolName<1..2^8-1>.
func parseALPN(edata []byte) []string {
	list, ok := readVec16Body(edata)
	if !ok {
		return nil
	}
	var out []string
	p := newParser(list)
	for p.remaining() > 0 {
		name, ok := p.readVec8()
		if !ok {
			break
		}
		out = append(out, string(name))
	}
	return out
}

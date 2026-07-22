package network

import "testing"

// fuzz_test.go fuzzes the untrusted-input TLS parsers (PLAN-08 backlog): a malicious
// ClientHello / record must never panic or hang the parser, and the downstream JA3/JA4
// derivations must be safe on anything the parser accepts.

func FuzzParseClientHello(f *testing.F) {
	f.Add([]byte{0x03, 0x03})
	f.Add([]byte{0x03, 0x03, 0x00, 0x00, 0x00, 0x00})
	f.Fuzz(func(t *testing.T, b []byte) {
		if ch, err := ParseClientHello(b); err == nil && ch != nil {
			_, _ = JA3(ch) // downstream must also be panic-free
			_, _ = JA4(ch)
		}
	})
}

func FuzzParseRecord(f *testing.F) {
	f.Add([]byte{0x16, 0x03, 0x01, 0x00, 0x00})
	f.Fuzz(func(t *testing.T, b []byte) {
		if ch, err := ParseRecord(b); err == nil && ch != nil {
			_, _ = JA3(ch)
			_, _ = JA4(ch)
		}
	})
}

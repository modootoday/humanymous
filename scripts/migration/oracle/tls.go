package main

import (
	"encoding/hex"
	"encoding/json"
	"github.com/modootoday/humanymous/internal/network"
	"os"
)

func exportTLS() {
	type vector struct {
		Input  string `json:"input"`
		Record bool   `json:"record"`
		Hello  any    `json:"hello"`
		JA4    string `json:"ja4"`
		Raw    string `json:"raw"`
	}
	var vectors []vector
	normalize := func(ch *network.ClientHello) any {
		protocols := []string{}
		for _, p := range ch.ALPN {
			protocols = append(protocols, hex.EncodeToString([]byte(p)))
		}
		return map[string]any{
			"ja3":    func() []string { hash, text := network.JA3(ch); return []string{hash, text} }(),
			"engine": network.EngineFromClientHello(ch), "extension_order_hash": network.ExtOrderHash(ch),
			"legacy_version": ch.LegacyVersion, "supported_versions": append([]uint16{}, ch.SupportedVers...),
			"cipher_suites": append([]uint16{}, ch.CipherSuites...), "extensions": append([]uint16{}, ch.Extensions...),
			"curves": append([]uint16{}, ch.Curves...), "point_formats": append([]uint16{}, ch.PointFormats...),
			"signature_algorithms": append([]uint16{}, ch.SigAlgs...), "alpn_hex": protocols, "sni_present": ch.SNIPresent,
		}
	}
	vec16 := func(b []byte) []byte { return append([]byte{byte(len(b) >> 8), byte(len(b))}, b...) }
	extension := func(kind uint16, b []byte) []byte { return append([]byte{byte(kind >> 8), byte(kind)}, vec16(b)...) }
	for mode := 0; mode < 12; mode++ {
		body := append([]byte{3, 3}, make([]byte, 32)...)
		body = append(body, 0)
		body = append(body, vec16([]byte{0x0a, 0x0a, 0x13, 1, 0x13, 2, 0xc0, 0x2f, 0xff})...)
		body = append(body, 1, 0)
		exts := [][]byte{
			extension(0, []byte{0}), extension(10, vec16([]byte{0x1a, 0x1a, 0, 29, 0, 23})),
			extension(11, []byte{2, 0, 1}), extension(13, vec16([]byte{4, 3, 8, 4})),
			extension(16, vec16([]byte{2, 'h', '2', 3, 0xff, 0, 0x80})),
			extension(43, []byte{4, 0x2a, 0x2a, 3, 4}), extension(0x3a3a, nil),
		}
		if mode%2 == 1 {
			for i, j := 0, len(exts)-1; i < j; i, j = i+1, j-1 {
				exts[i], exts[j] = exts[j], exts[i]
			}
		}
		block := []byte{}
		for _, ext := range exts {
			block = append(block, ext...)
		}
		if mode >= 4 {
			block = append(block, extension(16, []byte{0, 1, 9})...)
		}
		if mode >= 8 {
			block = append(block, 0, 0, 0, 9, 0)
		}
		body = append(body, vec16(block)...)
		handshake := append([]byte{1, 0, byte(len(body) >> 8), byte(len(body))}, body...)
		if mode%3 == 1 {
			handshake[3] += 10
		}
		if mode%3 == 2 {
			handshake[3] -= 10
		}
		for _, record := range []bool{false, true} {
			data := handshake
			if record {
				data = append([]byte{22, 3, 1, byte(len(data) >> 8), byte(len(data))}, data...)
			}
			for length := 0; length <= len(data); length++ {
				input := data[:length]
				var ch *network.ClientHello
				var err error
				if record {
					ch, err = network.ParseRecord(input)
				} else {
					ch, err = network.ParseClientHello(input)
				}
				v := vector{Input: hex.EncodeToString(input), Record: record}
				if err == nil {
					v.Hello = normalize(ch)
					v.JA4, v.Raw = network.JA4(ch)
				}
				vectors = append(vectors, v)
			}
		}
	}
	if err := json.NewEncoder(os.Stdout).Encode(vectors); err != nil {
		panic(err)
	}
}

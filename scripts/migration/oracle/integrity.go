package main

import (
	"encoding/hex"
	"encoding/json"
	"os"

	"github.com/modootoday/humanymous/internal/integrity"
)

type integrityCase struct {
	Key        string            `json:"key"`
	Session    string            `json:"session"`
	Counter    uint64            `json:"counter"`
	Bucket     uint64            `json:"bucket"`
	Headers    map[string]string `json:"headers"`
	Lowered    map[string]string `json:"lowered"`
	Cookie     string            `json:"cookie"`
	BodyHex    string            `json:"body_hex"`
	Canonical  string            `json:"canonical"`
	SessionKey string            `json:"session_key"`
	Seed       string            `json:"seed"`
	Token      string            `json:"token"`
}

func exportIntegrity() {
	var cases []integrityCase
	for _, key := range []string{"", "master-secret", "비밀"} {
		for _, session := range []string{"", "sess-123", "세션/🦀"} {
			for _, counter := range []uint64{0, 1, ^uint64(0)} {
				for _, body := range [][]byte{nil, []byte("{\"x\":1}"), {0, 255, 128, 10}} {
					headers := map[string]string{"User-Agent": "discarded", "user-agent": "  Browser\t\u0085\u00a0  1  ", "accept": "text/html", "İ": "dotted", "ΟΣ": "sigma"}
					lowered := integrity.LowerHeaderMap(headers)
					canonical := integrity.Canonical(integrity.CanonicalInput{Headers: lowered, CookieID: session, Body: body})
					sessionKey := integrity.SessionKey([]byte(key), session)
					seed := integrity.Seed(sessionKey, counter)
					token := integrity.ComputeToken(seed, canonical, counter, 1000)
					cases = append(cases, integrityCase{key, session, counter, 1000, headers, lowered, session, hex.EncodeToString(body), canonical, hex.EncodeToString(sessionKey), hex.EncodeToString(seed), token})
				}
			}
		}
	}
	encoder := json.NewEncoder(os.Stdout)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(cases); err != nil {
		panic(err)
	}
}

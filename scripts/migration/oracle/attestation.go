package main

import (
	"encoding/json"
	"os"

	"github.com/modootoday/humanymous/internal/attestation"
)

func exportAttestation() {
	type vector struct {
		Key     string `json:"key"`
		Subject string `json:"subject"`
		Nonce   string `json:"nonce"`
		Window  uint64 `json:"window"`
		Token   string `json:"token"`
		AtZero  bool   `json:"at_zero"`
		AtMax   bool   `json:"at_max"`
	}
	var vectors []vector
	for _, key := range []string{"", "master", "비밀"} {
		for _, subject := range []string{"", "fp-alice", "세션🦀"} {
			for _, nonce := range []string{"", "nonce-a", "수신|도전"} {
				for _, window := range []uint64{0, 100, ^uint64(0)} {
					token := attestation.Issue([]byte(key), subject, nonce, window)
					vectors = append(vectors, vector{key, subject, nonce, window, token,
						attestation.Verify([]byte(key), subject, nonce, token, 0),
						attestation.Verify([]byte(key), subject, nonce, token, ^uint64(0))})
				}
			}
		}
	}
	encoder := json.NewEncoder(os.Stdout)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(vectors); err != nil {
		panic(err)
	}
}

package main

import (
	"encoding/json"
	"os"

	"github.com/modootoday/humanymous/internal/resource"
)

func exportResource() {
	type vector struct {
		Key       string `json:"key"`
		Session   string `json:"session"`
		Asset     string `json:"asset"`
		Expiry    int64  `json:"expiry"`
		Signature string `json:"signature"`
		URL       string `json:"url"`
		AtMax     bool   `json:"at_max"`
	}
	var vectors []vector
	for _, key := range []string{"", "key", "비밀"} {
		for _, session := range []string{"", "sess", "세션🦀"} {
			for _, asset := range []string{"asset.mp4", "//asset", "/리소스/동영상"} {
				for _, expiry := range []int64{-1 << 63, 2000, 1<<63 - 1} {
					signature := resource.Sign([]byte(key), session, asset, expiry)
					vectors = append(vectors, vector{key, session, asset, expiry, signature, resource.SignedURL([]byte(key), session, asset, expiry), resource.Verify([]byte(key), session, asset, expiry, 1<<63-1, signature)})
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

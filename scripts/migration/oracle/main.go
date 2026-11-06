// Command oracle exports frozen Go behavior for Rust differential tests.
// It is migration tooling, not a production dependency.
package main

import (
	"encoding/json"
	"os"
	"sort"

	"github.com/modootoday/humanymous/internal/pow"
	"github.com/modootoday/humanymous/internal/signals"
)

type workCase struct {
	Key        string `json:"key"`
	Session    string `json:"session"`
	Bucket     uint64 `json:"bucket"`
	Difficulty int    `json:"difficulty"`
	Seed       string `json:"seed"`
	Encoded    string `json:"encoded"`
	Nonce      string `json:"nonce"`
	Solved     bool   `json:"solved"`
	Verified   bool   `json:"verified"`
}

type parseCase struct {
	Input  string `json:"input"`
	Bucket uint64 `json:"bucket"`
	Nonce  string `json:"nonce"`
	OK     bool   `json:"ok"`
}

func main() {
	if len(os.Args) > 1 && os.Args[1] == "engine" {
		exportEngine()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "behavior-signals" {
		exportBehaviorSignals()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "mlp" {
		exportMLP()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "crosschecks" {
		exportCrossChecks()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "unicode" {
		exportUnicode()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "derivation" {
		exportDerivation()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "behavior" {
		exportBehavior()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "reports" {
		exportReports()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "resource" {
		exportResource()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "signals" {
		exportSignals()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "rit" {
		exportRIT()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "registry" {
		definitions := signals.All()
		sort.Slice(definitions, func(i, j int) bool { return definitions[i].ID < definitions[j].ID })
		encoder := json.NewEncoder(os.Stdout)
		encoder.SetIndent("", "  ")
		if err := encoder.Encode(definitions); err != nil {
			panic(err)
		}
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "integrity" {
		exportIntegrity()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "attestation" {
		exportAttestation()
		return
	}
	var work []workCase
	for _, key := range []string{"", "master-key", "비밀"} {
		for _, session := range []string{"", "sess-1", "세션/🦀"} {
			for _, bucket := range []uint64{0, 1000, ^uint64(0)} {
				for _, difficulty := range []int{-1, 0, 8, 257} {
					challenge := pow.Issue([]byte(key), session, difficulty, bucket)
					nonce, solved := pow.Solve(challenge.Seed, difficulty, 4096)
					work = append(work, workCase{key, session, bucket, difficulty,
						challenge.Seed, challenge.Encode(), nonce, solved,
						pow.Verify([]byte(key), session, difficulty, bucket, bucket, nonce)})
				}
			}
		}
	}
	var parses []parseCase
	for _, input := range []string{"1000:123", "+1000:x", "001000:a:b", "0:", "garbage", "-1:x", " 1:x", "18446744073709551615:x", "18446744073709551616:x", "１:x"} {
		bucket, nonce, ok := pow.ParseSolution(input)
		parses = append(parses, parseCase{input, bucket, nonce, ok})
	}
	encoder := json.NewEncoder(os.Stdout)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(struct {
		Work  []workCase  `json:"work"`
		Parse []parseCase `json:"parse"`
	}{work, parses}); err != nil {
		panic(err)
	}
}

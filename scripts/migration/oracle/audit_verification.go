package main

import (
	"crypto/ed25519"
	"encoding/hex"
	"encoding/json"
	"github.com/modootoday/humanymous/internal/audit"
	"os"
)

func exportAuditVerification() {
	type baseCase struct {
		Seed          string           `json:"seed"`
		Key           string           `json:"key"`
		Public        string           `json:"public"`
		WitnessPublic string           `json:"witness_public"`
		Checkpoint    audit.Checkpoint `json:"checkpoint"`
		Records       []audit.Record   `json:"records"`
	}
	raw, err := os.ReadFile("crates/sdk/humanymous-runtime/tests/fixtures/audit/checkpoints.json")
	if err != nil {
		panic(err)
	}
	var bases []baseCase
	if err = json.Unmarshal(raw, &bases); err != nil {
		panic(err)
	}
	type vector struct {
		Records     []audit.Record     `json:"records"`
		Checkpoints []audit.Checkpoint `json:"checkpoints"`
		Key         string             `json:"key"`
		Public      string             `json:"public"`
		Witness     string             `json:"witness"`
		Expected    audit.VerifyResult `json:"expected"`
	}
	vectors := []vector{}
	for index, base := range bases {
		if index%3 != 0 {
			continue
		}
		for mode := 0; mode < 14; mode++ {
			encoded, _ := json.Marshal(base)
			var input baseCase
			if err = json.Unmarshal(encoded, &input); err != nil {
				panic(err)
			}
			checkpoints := []audit.Checkpoint{input.Checkpoint}
			key, _ := hex.DecodeString(input.Key)
			public, _ := hex.DecodeString(input.Public)
			witness := ""
			resign := func(cp *audit.Checkpoint) {
				// Independent signed-invalid-STH test setup, using the frozen documented
				// struct field order. Genuine signatures are separately checked verbatim.
				bytes, _ := json.Marshal(struct {
					Node     string `json:"node_id"`
					Size     uint64 `json:"tree_size"`
					Root     string `json:"root"`
					Merkle   string `json:"merkle_root"`
					Previous string `json:"prev_checkpoint_hash"`
				}{cp.NodeID, cp.TreeSize, cp.Root, cp.MerkleRoot, cp.PrevCP})
				seed, _ := hex.DecodeString(input.Seed)
				cp.Sig = hex.EncodeToString(ed25519.Sign(ed25519.NewKeyFromSeed(seed), bytes))
			}
			switch mode {
			case 1:
				key = nil
				input.Key = ""
			case 2:
				if len(input.Records) > 0 {
					input.Records[0].Seq = 2
				}
			case 3:
				if len(input.Records) > 0 {
					input.Records[0].PrevHash = "broken"
				}
			case 4:
				if len(input.Records) > 0 {
					input.Records[0].Host = "altered"
				}
			case 5:
				if len(input.Records) > 0 {
					input.Records[0].HMAC = "invalid"
				}
			case 6:
				checkpoints[0].Sig = "invalid"
			case 7:
				checkpoints[0].PrevCP = "broken"
				resign(&checkpoints[0])
			case 8:
				checkpoints[0].Root = "broken"
				resign(&checkpoints[0])
			case 9:
				checkpoints[0].MerkleRoot = "broken"
				resign(&checkpoints[0])
			case 10:
				checkpoints[0].TreeSize += 100
				resign(&checkpoints[0])
			case 11:
				witness = input.WitnessPublic
			case 12:
				witness = input.WitnessPublic
				checkpoints[0].WitnessSig = "invalid"
			case 13:
				checkpoints = []audit.Checkpoint{}
				key = nil
				input.Key = ""
			}
			expected := audit.Verify(input.Records, checkpoints, key, public)
			if expected.OK && witness != "" {
				wpub, _ := hex.DecodeString(witness)
				if at, ok := audit.VerifyWitness(checkpoints, wpub); !ok {
					expected = audit.VerifyResult{Class: audit.ClassWitnessBad, AtSeq: at, Detail: "witness co-signature missing or invalid"}
				}
			}
			vectors = append(vectors, vector{input.Records, checkpoints, input.Key, input.Public, witness, expected})
		}
	}
	if err = json.NewEncoder(os.Stdout).Encode(vectors); err != nil {
		panic(err)
	}
}

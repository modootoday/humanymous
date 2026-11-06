package main

import (
	"encoding/hex"
	"encoding/json"
	"github.com/modootoday/humanymous/internal/audit"
	"os"
	"strings"
)

func exportCheckpoints() {
	type vector struct {
		Seed          string           `json:"seed"`
		WitnessSeed   string           `json:"witness_seed"`
		Key           string           `json:"key"`
		Public        string           `json:"public"`
		WitnessPublic string           `json:"witness_public"`
		Checkpoint    audit.Checkpoint `json:"checkpoint"`
		Records       []audit.Record   `json:"records"`
	}
	vectors := []vector{}
	for _, seedByte := range []byte{0, 7, 255} {
		seed := []byte(strings.Repeat(string([]byte{seedByte}), 32))
		witnessSeed := []byte(strings.Repeat(string([]byte{seedByte ^ 0x5a}), 32))
		for _, node := range []string{"", "node", "주체/e\u0301", "<&>\u2028\u2029", "line\nquote\"\\"} {
			for _, size := range []int{0, 1, 3, 8} {
				witness := audit.NewWitnessFromSeed(witnessSeed)
				key := []byte("oracle-hmac-key")
				log := audit.NewLog(audit.Config{NodeID: node, HMACKey: key, SigningSeed: seed, Witness: witness, CheckpointEvery: 1024})
				for index := 0; index < size; index++ {
					log.Append(audit.Record{EventID: "fixture-event", TS: "2026-11-07T00:00:00Z", EventType: audit.EventScoringEvaluated, TenantID: "fixture-tenant", RiskScore: index, Rules: []string{"z", "a"}, TopSignals: []audit.Signal{{ID: "e\u0301", Verdict: "UNKNOWN", Conf: 0.0005}}})
				}
				for repeat := 0; repeat < 3; repeat++ {
					cp := log.Checkpoint()
					records := log.Records()
					if records == nil {
						records = []audit.Record{}
					}
					vectors = append(vectors, vector{hex.EncodeToString(seed), hex.EncodeToString(witnessSeed), hex.EncodeToString(key), hex.EncodeToString(log.PublicKey()), hex.EncodeToString(witness.Public()), cp, records})
				}
			}
		}
	}
	if err := json.NewEncoder(os.Stdout).Encode(vectors); err != nil {
		panic(err)
	}
}

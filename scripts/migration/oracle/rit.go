package main

import (
	"encoding/hex"
	"encoding/json"
	"os"

	"github.com/modootoday/humanymous/internal/integrity"
	"github.com/modootoday/humanymous/internal/signals"
)

func exportRIT() {
	type vector struct {
		Key         string         `json:"key"`
		LastN       uint64         `json:"last_n"`
		PresentedN  uint64         `json:"presented_n"`
		PresentedTB uint64         `json:"presented_tb"`
		CurrentTB   uint64         `json:"current_tb"`
		Token       string         `json:"token"`
		Canonical   string         `json:"canonical"`
		FirstReq    bool           `json:"first_req"`
		Expected    signals.Signal `json:"expected"`
	}
	var vectors []vector
	key := integrity.SessionKey([]byte("master"), "session")
	for _, last := range []uint64{0, 5, ^uint64(0)} {
		for variant := 0; variant < 9; variant++ {
			p := integrity.VerifyParams{SessionKey: key, LastN: last, PresentedN: last + 1, PresentedTB: 1000, CurrentTB: 1000, Canonical: "observed"}
			p.Token = integrity.ComputeToken(integrity.Seed(key, last), p.Canonical, p.PresentedN, p.PresentedTB)
			switch variant {
			case 1:
				p.FirstReq = true
				p.Token = ""
			case 2:
				p.Token = ""
				p.PresentedN++
			case 3:
				p.PresentedN++
			case 4:
				p.CurrentTB += 5
			case 5:
				p.Token = "tampered"
			case 6:
				p.Canonical = "tampered"
			case 7:
				p.CurrentTB++
			case 8:
				p.PresentedTB = ^uint64(0)
				p.CurrentTB = 0
			}
			vectors = append(vectors, vector{hex.EncodeToString(key), p.LastN, p.PresentedN, p.PresentedTB, p.CurrentTB, p.Token, p.Canonical, p.FirstReq, integrity.Verify(p)})
		}
	}
	encoder := json.NewEncoder(os.Stdout)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(vectors); err != nil {
		panic(err)
	}
}

package main

import (
	"encoding/json"
	"github.com/modootoday/humanymous/internal/scoring"
	"github.com/modootoday/humanymous/internal/signals"
	"os"
)

// Exercise complete scoring over the independently generated probe/claim inputs.
// Expected values always come from the frozen Go implementation, never Rust.
func exportEngine() {
	type vector struct {
		Report   json.RawMessage     `json:"report"`
		Modes    map[string]string   `json:"modes"`
		Weights  map[string]float64  `json:"weights"`
		Net      map[string]string   `json:"net"`
		Expected signals.ScoreResult `json:"expected"`
		Trace    scoring.ScoreTrace  `json:"trace"`
	}
	var inputs []struct {
		Report json.RawMessage `json:"report"`
	}
	raw, err := os.ReadFile("crates/scoring/tests/fixtures/crosschecks.json")
	if err != nil {
		panic(err)
	}
	if err = json.Unmarshal(raw, &inputs); err != nil {
		panic(err)
	}
	// Start from the unmodified human freeze fixture so client execution and
	// coherent network claims do not mask individual promotion predicates.
	baseRaw, err := os.ReadFile("test/fixtures/scoring/human-chrome-allow.json")
	if err != nil {
		panic(err)
	}
	var base struct {
		Report signals.SessionReport `json:"report"`
	}
	if err = json.Unmarshal(baseRaw, &base); err != nil {
		panic(err)
	}
	add := func(ids []string, verdict signals.Verdict) {
		r := base.Report
		r.Client.Signals = append([]signals.Signal{}, base.Report.Client.Signals...)
		for _, id := range ids {
			r.Client.Signals = append(r.Client.Signals, signals.New(id, true, verdict, 1, signals.SourceWASM, "oracle"))
		}
		encoded, marshalErr := json.Marshal(r)
		if marshalErr != nil {
			panic(marshalErr)
		}
		inputs = append(inputs, struct {
			Report json.RawMessage `json:"report"`
		}{encoded})
	}
	for _, definition := range signals.All() {
		for _, verdict := range []signals.Verdict{signals.VerdictOK, signals.VerdictSuspicious, signals.VerdictBot} {
			add([]string{definition.ID}, verdict)
		}
	}
	for _, ids := range [][]string{
		{"l4.event.untrusted", "l1.navigator.webdriver"},
		{"l1.navigator.webdriver", "l3.integrity.native_tostring"},
		{"l3.guard.native_hooked", "l1.cdp.runtime_enable"},
		{"l1.ua.headless_token", "l1.window.outer_eq_inner"},
		{"l3.integrity.native_tostring", "l3.integrity.iframe_recheck"},
		{"l1.cdp.proxy_leak", "l4.event.no_interaction"},
		{"l3.guard.console_disabled", "l1.window.outer_eq_inner"},
		{"l4.mouse.click_no_trajectory", "l4.agent.burst_silence"},
		{"l5.traffic.ua_rotation", "l5.traffic.ip_hop"},
		{"l2.adv.no_widevine", "l2.adv.no_media_devices", "l2.adv.no_voices"},
		{"l5.header.xff_multi", "l5.traffic.ip_hop"},
		{"l2.canvas.tampered", "l2.webgl.param_consistency", "l2.env.privacy_mode"},
	} {
		add(ids, signals.VerdictBot)
	}
	for _, report := range []signals.SessionReport{{}, base.Report} {
		if report.Client.UserAgent != "" {
			report.Network.JA4Engine = "go"
			report.Client.Signals = []signals.Signal{signals.New("l5.rit.invalid_hmac", true, signals.VerdictBot, 1, signals.SourceServer, "oracle")}
		}
		encoded, marshalErr := json.Marshal(report)
		if marshalErr != nil {
			panic(marshalErr)
		}
		inputs = append(inputs, struct {
			Report json.RawMessage `json:"report"`
		}{encoded})
	}
	var vectors []vector
	for _, input := range inputs {
		for mode := 0; mode < 4; mode++ {
			var r signals.SessionReport
			if err = json.Unmarshal(input.Report, &r); err != nil {
				panic(err)
			}
			e := scoring.NewEngine()
			switch mode {
			case 1:
				e.RuleModes = map[string]string{"HR-10": "monitor", "HR-2": "monitor", "HR-3": "monitor"}
			case 2:
				e.WeightMultipliers = map[string]float64{"l6.ua_tls_mismatch": 0, "l6.ua_h2_mismatch": 2, "l1.navigator.webdriver": -1}
			case 3:
				e.NetPolicy = map[string]string{}
			}
			result, trace := e.ScoreWithTrace(&r)
			vectors = append(vectors, vector{input.Report, e.RuleModes, e.WeightMultipliers, e.NetPolicy, result, trace})
		}
	}
	if err = json.NewEncoder(os.Stdout).Encode(vectors); err != nil {
		panic(err)
	}
}

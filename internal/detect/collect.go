//go:build js && wasm

package detect

import (
	"encoding/json"
	"syscall/js"

	"github.com/modootoday/humanymous/internal/signals"
)

// collect.go assembles the L1-L3 client signals into a ClientReport and exposes
// the detection entry point to JS. Behavioral (L4) and environment (SoT-09)
// features are added by the JS loader before posting.

// EngineVersion identifies this detector build in the report.
const EngineVersion = "wasm-1.0.0"

// Detect runs all client-side layers and returns a partial ClientReport JSON.
// this/args are unused; JS calls window.__hmDetect().
func Detect(this js.Value, args []js.Value) any {
	var all []signals.Signal
	all = append(all, collectL1()...)
	all = append(all, collectL2()...)
	all = append(all, collectL3()...)

	report := signals.ClientReport{
		UserAgent:     navString("userAgent"),
		UAClientHints: readClientHints(),
		Signals:       all,
		EngineVersion: EngineVersion,
	}
	b, err := json.Marshal(report)
	if err != nil {
		return js.ValueOf("{}")
	}
	return js.ValueOf(string(b))
}

// readClientHints reads the synchronous low-entropy UA-CH from navigator.
func readClientHints() signals.UAClientHints {
	var ch signals.UAClientHints
	uad := safeGet(navigator, "userAgentData")
	if !uad.Truthy() {
		return ch
	}
	ch.Mobile = uad.Get("mobile").Truthy()
	if p := uad.Get("platform"); p.Type() == js.TypeString {
		ch.Platform = p.String()
	}
	brands := uad.Get("brands")
	if brands.Truthy() {
		n := brands.Length()
		for i := 0; i < n; i++ {
			b := brands.Index(i)
			ch.Brands = append(ch.Brands, signals.UABrand{
				Brand:   b.Get("brand").String(),
				Version: b.Get("version").String(),
			})
		}
	}
	return ch
}

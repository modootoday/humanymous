//go:build js && wasm

// SPDX-License-Identifier: Apache-2.0

// Command wasm is the browser-side detection engine (GOOS=js GOARCH=wasm). It
// registers window.__hmDetect() which the JS loader calls to collect L1-L3
// signals, then keeps the Go runtime alive. See sots/01-client-signals.md.
package main

import (
	"syscall/js"

	"github.com/modootoday/humanymous/internal/detect"
)

func main() {
	js.Global().Set("__hmDetect", js.FuncOf(detect.Detect))
	js.Global().Set("__hmVersion", js.ValueOf(detect.EngineVersion))
	// Keep the Go runtime alive so exported functions remain callable.
	select {}
}

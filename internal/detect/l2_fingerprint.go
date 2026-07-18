//go:build js && wasm

package detect

import (
	"strings"
	"syscall/js"

	"github.com/modootoday/humanymous/internal/signals"
)

// l2_fingerprint.go collects L2 fingerprint signals (SoT-01 §L2): WebGL
// renderer, hardware, screen metrics.

func collectL2() []signals.Signal {
	var out []signals.Signal
	add := func(id string, val any, v signals.Verdict, conf float64, notes string) {
		out = append(out, signals.New(id, val, v, conf, signals.SourceWASM, notes))
	}

	// WebGL renderer (SwiftShader software renderer is the strongest headless tell).
	vendor, renderer := webglVendorRenderer()
	if renderer != "" {
		low := strings.ToLower(renderer + " " + vendor)
		if strings.Contains(low, "swiftshader") || strings.Contains(low, "llvmpipe") || strings.Contains(low, "mesa offscreen") {
			add("l2.webgl.swiftshader", renderer, signals.VerdictBot, 1.0, "software renderer: "+renderer)
		}
	}

	// deviceMemory quantization. The spec caps at 8, but shipping Chromium/Edge
	// builds may report the real RAM (e.g. 32), so we only treat NON-power-of-two
	// values (3, 6, 7...) as a spoof tell to avoid false positives (SoT-06 §4).
	dm := safeGet(navigator, "deviceMemory")
	if dm.Type() == js.TypeNumber {
		if v := dm.Float(); v > 0 && !isPowerOfTwoish(v) {
			add("l2.hardware.devicememory", v, signals.VerdictSuspicious, 0.8, "deviceMemory not power-of-two (manual override)")
		}
	}

	// screen metrics: availHeight == height (no taskbar) is a headless hint.
	screen := safeGet(window, "screen")
	if screen.Truthy() {
		h := screen.Get("height").Int()
		ah := screen.Get("availHeight").Int()
		if h != 0 && h == ah {
			add("l2.screen.metrics", map[string]any{"height": h, "availHeight": ah},
				signals.VerdictSuspicious, 0.5, "availHeight==height (no taskbar)")
		}
	}

	return out
}

// webglVendorRenderer reads UNMASKED vendor/renderer via WEBGL_debug_renderer_info.
func webglVendorRenderer() (vendor, renderer string) {
	canvas := document.Call("createElement", "canvas")
	gl := canvas.Call("getContext", "webgl")
	if !gl.Truthy() {
		gl = canvas.Call("getContext", "experimental-webgl")
	}
	if !gl.Truthy() {
		return "", ""
	}
	dbg := gl.Call("getExtension", "WEBGL_debug_renderer_info")
	if !dbg.Truthy() {
		return "", ""
	}
	vendor = gl.Call("getParameter", dbg.Get("UNMASKED_VENDOR_WEBGL")).String()
	renderer = gl.Call("getParameter", dbg.Get("UNMASKED_RENDERER_WEBGL")).String()
	return vendor, renderer
}

// isPowerOfTwoish reports whether v is a plausible power-of-two device-memory
// value (fractional low end + integer powers up to 64). Non-matching values
// (3, 6, 7, ...) indicate a manual override.
func isPowerOfTwoish(v float64) bool {
	for _, ok := range []float64{0.25, 0.5, 1, 2, 4, 8, 16, 32, 64} {
		if v == ok {
			return true
		}
	}
	return false
}

//go:build js && wasm

package detect

import (
	"syscall/js"

	"github.com/modootoday/humanymous/internal/signals"
)

// l3_integrity.go collects L3 runtime-integrity signals (SoT-01 §L3): whether
// key getters/methods are still native, and iframe-realm re-checks.

func collectL3() []signals.Signal {
	var out []signals.Signal
	add := func(id string, val any, v signals.Verdict, notes string) {
		out = append(out, signals.New(id, val, v, 1.0, signals.SourceWASM, notes))
	}

	// native toString on the webdriver getter: if present and NOT native, it was
	// patched (a stealth tool hiding webdriver).
	if getter := webdriverGetter(); getter.Truthy() && !isNativeFn(getter) {
		add("l3.integrity.native_tostring", false, signals.VerdictBot, "webdriver getter is not native (patched)")
	}

	// iframe realm re-check: navigator.webdriver read from a fresh iframe realm.
	if topWD, ifWD, ok := iframeWebdriver(); ok && topWD != ifWD {
		add("l3.integrity.iframe_recheck", ifWD, signals.VerdictBot, "iframe realm disagrees on webdriver")
	}

	return out
}

// webdriverGetter returns the getter descriptor for Navigator.prototype.webdriver.
func webdriverGetter() js.Value {
	proto := navigator.Get("constructor").Get("prototype")
	desc := js.Global().Get("Object").Call("getOwnPropertyDescriptor", proto, "webdriver")
	if !desc.Truthy() {
		return js.Undefined()
	}
	return desc.Get("get")
}

// iframeWebdriver reads navigator.webdriver from a fresh same-origin iframe and
// compares with the top-level value. Returns ok=false if iframe creation fails.
func iframeWebdriver() (top, inner bool, ok bool) {
	body := document.Get("body")
	if !body.Truthy() {
		return false, false, false
	}
	frame := document.Call("createElement", "iframe")
	frame.Get("style").Set("display", "none")
	body.Call("appendChild", frame)
	defer body.Call("removeChild", frame)

	cw := frame.Get("contentWindow")
	if !cw.Truthy() {
		return false, false, false
	}
	inav := cw.Get("navigator")
	if !inav.Truthy() {
		return false, false, false
	}
	top = navBool("webdriver", false)
	iv := inav.Get("webdriver")
	if iv.Type() == js.TypeBoolean {
		inner = iv.Bool()
	}
	return top, inner, true
}

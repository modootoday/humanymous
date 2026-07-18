//go:build js && wasm

// Package detect is the in-browser WASM detection engine. It reads L1/L2/L3
// signals through syscall/js and returns them to the JS loader, which adds the
// behavioral/environment features and posts the report (SoT-01, plan/02 §4).
package detect

import "syscall/js"

// bridge.go isolates all syscall/js access (SRP): the rest of the package reads
// browser state through these safe helpers so the detection rules stay small.

var (
	global    = js.Global()
	navigator = global.Get("navigator")
	window    = global
	document  = global.Get("document")
)

// safeGet returns obj[key] guarding against undefined/null objects.
func safeGet(obj js.Value, key string) js.Value {
	if !obj.Truthy() {
		return js.Undefined()
	}
	return obj.Get(key)
}

// getBool reads a boolean property, defaulting to def when absent.
func navBool(key string, def bool) bool {
	v := safeGet(navigator, key)
	if v.Type() != js.TypeBoolean {
		return def
	}
	return v.Bool()
}

// navString reads a string property ("" when absent).
func navString(key string) string {
	v := safeGet(navigator, key)
	if v.Type() != js.TypeString {
		return ""
	}
	return v.String()
}

// hasKey reports whether obj has a truthy property key.
func hasKey(obj js.Value, key string) bool {
	return safeGet(obj, key).Truthy()
}

// isNativeFn reports whether fn's source ends with "{ [native code] }" — a
// cheap monkeypatch check (SoT-01 L3). Full CreepJS-grade checks would also
// verify toString itself; kept minimal here.
func isNativeFn(fn js.Value) bool {
	if fn.Type() != js.TypeFunction {
		return false
	}
	fp := global.Get("Function").Get("prototype").Get("toString")
	src := fp.Call("call", fn).String()
	return len(src) > 0 && containsNative(src)
}

func containsNative(s string) bool {
	// look for "[native code]"
	const marker = "[native code]"
	for i := 0; i+len(marker) <= len(s); i++ {
		if s[i:i+len(marker)] == marker {
			return true
		}
	}
	return false
}

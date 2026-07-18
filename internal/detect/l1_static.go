//go:build js && wasm

package detect

import (
	"strings"
	"syscall/js"

	"github.com/modootoday/humanymous/internal/signals"
)

// l1_static.go collects L1 static-client signals (SoT-01 §L1). Each check emits
// one signal; the caller aggregates them.

func collectL1() []signals.Signal {
	var out []signals.Signal
	add := func(id string, val any, v signals.Verdict, notes string) {
		out = append(out, signals.New(id, val, v, 1.0, signals.SourceWASM, notes))
	}

	// navigator.webdriver
	wd := navBool("webdriver", false)
	add("l1.navigator.webdriver", wd, boolVerdict(wd), "navigator.webdriver")

	// Headless UA token (Chrome: "HeadlessChrome", Edge: "HeadlessEdg", etc.)
	ua := navString("userAgent")
	if strings.Contains(ua, "Headless") {
		add("l1.ua.headless_token", true, signals.VerdictBot, "Headless token in UA")
	}

	// plugins empty (weak; Firefox/mobile legitimately empty)
	plugins := safeGet(navigator, "plugins")
	if plugins.Truthy() && plugins.Get("length").Int() == 0 {
		add("l1.navigator.plugins_empty", 0, signals.VerdictSuspicious, "empty plugins")
	}

	// languages empty
	langs := safeGet(navigator, "languages")
	if langs.Truthy() && langs.Get("length").Int() == 0 {
		add("l1.navigator.languages_empty", true, signals.VerdictBot, "empty languages")
	}

	// window.chrome missing (only meaningful if UA claims Chrome)
	if strings.Contains(ua, "Chrome") && !hasKey(window, "chrome") {
		add("l1.window.chrome_missing", true, signals.VerdictSuspicious, "window.chrome absent for Chrome UA")
	}

	// permission mismatch: Notification.permission vs permissions.query
	if v := permissionMismatch(); v {
		add("l1.permissions.mismatch", true, signals.VerdictBot, "Notification vs permissions.query mismatch")
	}

	// outerWidth == innerWidth (no browser chrome)
	if ow, iw := window.Get("outerWidth").Int(), window.Get("innerWidth").Int(); ow != 0 && ow == iw {
		add("l1.window.outer_eq_inner", true, signals.VerdictSuspicious, "outerWidth==innerWidth")
	}

	// CDP Runtime.enable leak (classic probe, mostly patched).
	if hasKey(window, "__hmCDP") && window.Get("__hmCDP").Truthy() {
		add("l1.cdp.runtime_enable", true, signals.VerdictBot, "CDP Runtime.enable leak")
	}
	// CDP prototype-Proxy preview leak (post-May-2025 probe) — catches current
	// Chrome driven over CDP.
	if hasKey(window, "__hmCDP2") && window.Get("__hmCDP2").Truthy() {
		add("l1.cdp.proxy_leak", true, signals.VerdictBot, "CDP prototype-Proxy preview leak")
	}

	// Automation framework artifacts
	if seleniumArtifacts() {
		add("l1.artifact.selenium", true, signals.VerdictBot, "selenium/cdc_ artifact present")
	}
	if playwrightArtifacts() {
		add("l1.artifact.playwright", true, signals.VerdictBot, "playwright global present")
	}
	if hasKey(window, "_phantom") || hasKey(window, "callPhantom") {
		add("l1.artifact.phantom", true, signals.VerdictBot, "phantomjs global present")
	}

	return out
}

// permissionMismatch is best-effort: Notification denied while query says prompt
// is the classic headless contradiction. Async permissions.query is skipped in
// this synchronous pass; we compare Notification.permission presence only.
func permissionMismatch() bool {
	notif := safeGet(window, "Notification")
	if !notif.Truthy() {
		return false
	}
	perm := notif.Get("permission")
	// A headless contradiction commonly shows "denied" without user action.
	return perm.Type() == js.TypeString && perm.String() == "denied" && navBool("webdriver", false)
}

// seleniumArtifacts scans document/window keys for cdc_/webdriver markers.
func seleniumArtifacts() bool {
	markers := []string{"$cdc_", "cdc_", "__webdriver_evaluate", "__selenium_evaluate",
		"__webdriver_script_function", "__fxdriver_evaluate", "__driver_evaluate",
		"domAutomation", "domAutomationController"}
	if keysContainAny(document, markers) {
		return true
	}
	return keysContainAny(window, markers)
}

func playwrightArtifacts() bool {
	for _, k := range []string{"__playwright", "__pw_manual", "__pwInitScripts", "__playwright__binding__"} {
		if hasKey(window, k) {
			return true
		}
	}
	return false
}

// keysContainAny reports whether any own-key of obj contains one of markers.
func keysContainAny(obj js.Value, markers []string) bool {
	if !obj.Truthy() {
		return false
	}
	keys := js.Global().Get("Object").Call("keys", obj)
	n := keys.Length()
	for i := 0; i < n; i++ {
		k := keys.Index(i).String()
		for _, m := range markers {
			if strings.Contains(k, m) {
				return true
			}
		}
	}
	return false
}

func boolVerdict(b bool) signals.Verdict {
	if b {
		return signals.VerdictBot
	}
	return signals.VerdictOK
}

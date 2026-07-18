package scoring

import "github.com/modootoday/humanymous/internal/signals"

// guard.go turns injector.js runtime-hardening results into L3 guard signals
// (SoT-11 §4). Weights favor tampering that targets our detection (native hook,
// prototype pollution); eval/injection alone are low-confidence to avoid FPs on
// legitimate third-party scripts.

// GuardSignals derives l3.guard.* signals from the client Guard report.
func GuardSignals(g signals.Guard) []signals.Signal {
	if !g.Reported {
		return nil
	}
	var out []signals.Signal
	add := func(id string, val any, v signals.Verdict, conf float64, notes string) {
		out = append(out, signals.New(id, val, v, conf, signals.SourceJS, notes))
	}
	if g.NativeHooked {
		add("l3.guard.native_hooked", true, signals.VerdictBot, 1.0, "fetch/XHR/JSON/toString monkeypatched")
	}
	if g.ConsoleDisabled {
		add("l3.guard.console_disabled", true, signals.VerdictBot, 0.9, "Console API disabled/non-native (Patchright signature)")
	}
	if g.ProtoPolluted {
		add("l3.guard.proto_polluted", true, signals.VerdictBot, 1.0, "core prototype mutated")
	}
	if g.ScriptInjected > 0 {
		add("l3.guard.script_injected", g.ScriptInjected, signals.VerdictSuspicious, 0.6, "no-nonce script injected")
	}
	if g.EvalUsed > 0 {
		add("l3.guard.eval_used", g.EvalUsed, signals.VerdictSuspicious, 0.4, "eval/indirect-eval used")
	}
	if g.FuncCtor > 0 {
		add("l3.guard.func_ctor", g.FuncCtor, signals.VerdictSuspicious, 0.4, "Function constructor used")
	}
	return out
}

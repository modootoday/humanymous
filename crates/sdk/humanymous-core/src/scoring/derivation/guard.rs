use crate::signals::{
    observation::{Signal, Source, Verdict},
    report::Guard,
};
use serde_json::json;

/// Derive runtime-hardening observations in the original order.
pub fn guard_signals(guard: &Guard) -> Vec<Signal> {
    if !guard.reported {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut add = |id, value, confidence, notes, verdict| {
        result.push(Signal::new(
            id,
            value,
            verdict,
            confidence,
            Source::Js,
            notes,
        ))
    };
    if guard.native_hooked {
        add(
            "l3.guard.native_hooked",
            json!(true),
            1.0,
            "fetch/XHR/JSON/toString monkeypatched",
            Verdict::Bot,
        );
    }
    if guard.console_disabled {
        add(
            "l3.guard.console_disabled",
            json!(true),
            0.9,
            "Console API disabled/non-native (Patchright signature)",
            Verdict::Bot,
        );
    }
    if guard.proto_polluted {
        add(
            "l3.guard.proto_polluted",
            json!(true),
            1.0,
            "core prototype mutated",
            Verdict::Bot,
        );
    }
    if guard.script_injected > 0 {
        add(
            "l3.guard.script_injected",
            json!(guard.script_injected),
            0.6,
            "no-nonce script injected",
            Verdict::Suspicious,
        );
    }
    if guard.eval_used > 0 {
        add(
            "l3.guard.eval_used",
            json!(guard.eval_used),
            0.4,
            "eval/indirect-eval used",
            Verdict::Suspicious,
        );
    }
    if guard.func_ctor > 0 {
        add(
            "l3.guard.func_ctor",
            json!(guard.func_ctor),
            0.4,
            "Function constructor used",
            Verdict::Suspicious,
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unreported_guard_is_absent_and_counter_threshold_is_strict() {
        assert!(
            guard_signals(&Guard {
                native_hooked: true,
                ..Default::default()
            })
            .is_empty()
        );
        assert!(
            guard_signals(&Guard {
                reported: true,
                script_injected: -1,
                ..Default::default()
            })
            .is_empty()
        );
        let signals = guard_signals(&Guard {
            reported: true,
            native_hooked: true,
            console_disabled: true,
            ..Default::default()
        });
        assert_eq!(
            signals
                .iter()
                .map(|signal| signal.id.as_str())
                .collect::<Vec<_>>(),
            ["l3.guard.native_hooked", "l3.guard.console_disabled"]
        );
    }
}

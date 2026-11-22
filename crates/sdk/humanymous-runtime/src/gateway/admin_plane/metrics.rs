//! Admin `metrics` route of legacy internal/gate/admin.go (`adminMetrics`,
//! settings_runtime.go `prometheusLabel`) at baseline 981d25c, owner decision
//! #5 (issue #75 comment 7017558784): Go's Prometheus names, HELP and TYPE
//! text, label sets and order, served on the admin listener only.
//!
//! Pinned against `tests/fixtures/gateway/admin-metrics-go-golden.json`, a
//! capture of `adminMetrics` from the oracle (go1.26.0, offline module cache)
//! for a fresh settings server and after one applied overlay.
//!
//! Kept from Go: the `text/plain; version=0.0.4; charset=utf-8` content type;
//! any role may read it (Go `adminAnyRole`); uptime as whole seconds
//! (`%.0f`); `hmn_gate_audit_projection_dropped_total` is 0 because no
//! projection sink exists (Go's unwired getter); `killswitch` and `monitor`
//! from the live kill-switch flag and the boot global monitor;
//! `settings_overlay_active`, the oldest pending settings approval age, the
//! four apply outcomes and store errors; `config_version` with the version as
//! a `prometheusLabel`-escaped label.
//!
//! Families not emitted, each recorded in the ledger:
//! - `audit_records_total`, `audit_checkpoints_total`,
//!   `audit_integrity_ok`, `audit_witnessed`: the admin listener has no
//!   audit chain attached (`AdminPlane::with_chain`), and a value without the
//!   chain would be invented.
//! - the four `operational_log_*` families: the binary's operational logger
//!   is not shared with the admin plane; Go's unwired defaults (zeros,
//!   healthy) would claim a health that is not measured.
//! - `goroutines`, `heap_alloc_bytes`, `sys_bytes`: Go runtime figures with
//!   no Rust counterpart.
//!
//! Without a policy attached `killswitch` and `monitor` are 0 and
//! `config_version` is omitted; without settings the settings families are 0.
//! The route only reads; it changes no score, threshold or signal.
use super::{AdminPlane, Clock, Response};
use std::fmt::Write as _;

/// Go's metrics content type.
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// Go `prometheusLabel`: escape backslash, newline and double quote.
pub fn label(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('"', "\\\"")
}

fn family(out: &mut String, name: &str, help: &str, kind: &str) {
    let _ = write!(out, "# HELP {name} {help}\n# TYPE {name} {kind}\n");
}

fn single(out: &mut String, name: &str, help: &str, kind: &str, value: impl std::fmt::Display) {
    family(out, name, help, kind);
    let _ = writeln!(out, "{name} {value}");
}

impl AdminPlane {
    pub(super) fn metrics_view(&self, clock: Clock) -> Response {
        let mut out = String::new();
        single(
            &mut out,
            "hmn_gate_uptime_seconds",
            "Seconds since this gate process started.",
            "gauge",
            format_args!("{:.0}", clock.monotonic.as_secs_f64()),
        );
        let bans = self.store().list(clock.monotonic).len();
        single(
            &mut out,
            "hmn_gate_bans_active",
            "Currently active IP/fingerprint bans.",
            "gauge",
            bans,
        );
        single(
            &mut out,
            "hmn_gate_audit_projection_dropped_total",
            "Audit records dropped by Tier-1/2 projection sinks (the WAL remains the durability authority).",
            "counter",
            0,
        );
        let (kill, monitor) = self
            .policy
            .as_ref()
            .map_or((false, false), |policy| policy.switches());
        single(
            &mut out,
            "hmn_gate_killswitch",
            "Kill switch engaged (1) or not (0).",
            "gauge",
            u8::from(kill),
        );
        single(
            &mut out,
            "hmn_gate_monitor",
            "Effective global monitor mode (1) or enforcing (0).",
            "gauge",
            u8::from(monitor),
        );
        let overlay = self
            .settings
            .as_ref()
            .is_some_and(|settings| settings.active().is_some());
        single(
            &mut out,
            "hmn_gate_settings_overlay_active",
            "Runtime Settings overlay active (1) or empty/freeze defaults (0).",
            "gauge",
            u8::from(overlay),
        );
        single(
            &mut out,
            "hmn_gate_settings_overlay_pending_age_seconds",
            "Age in seconds of the oldest pending Settings approval (0 when none).",
            "gauge",
            format_args!(
                "{:.0}",
                self.settings_pending_age(clock.monotonic).as_secs_f64()
            ),
        );
        let stats = self.settings_stats().unwrap_or_default();
        family(
            &mut out,
            "hmn_gate_settings_apply_total",
            "Settings apply outcomes.",
            "counter",
        );
        for (result, count) in [
            ("applied", stats.applied),
            ("rolled_back", stats.rolled_back),
            ("rejected", stats.rejected),
            ("error", stats.errors),
        ] {
            let _ = writeln!(
                out,
                "hmn_gate_settings_apply_total{{result=\"{result}\"}} {count}"
            );
        }
        single(
            &mut out,
            "hmn_gate_settings_store_errors_total",
            "Settings store load or persistence errors.",
            "counter",
            stats.store_errors,
        );
        if let Some(policy) = &self.policy {
            family(
                &mut out,
                "hmn_gate_config_version",
                "Effective signed Settings configuration version.",
                "gauge",
            );
            let _ = writeln!(
                out,
                "hmn_gate_config_version{{version=\"{}\"}} 1",
                label(&policy.version())
            );
        }
        Response {
            status: 200,
            content_type: CONTENT_TYPE,
            body: out.into_bytes(),
        }
    }
}

#[cfg(test)]
mod tests;

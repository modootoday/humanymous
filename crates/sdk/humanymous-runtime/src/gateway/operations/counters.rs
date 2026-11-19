//! `/api/counters` operator rollup (port of legacy cmd/server/ops.go
//! `handleCounters`).
//!
//! The behavioral-model block is the on-call tell for a canary auto-rollback:
//! a tripped canary is visible here without reading logs. It is nil-safe:
//! without a controller it reports `enabled: false`. The fleet block is always
//! present; `redisFallbackTotal` is the monotonic count of fleet observations
//! that degraded to per-node correlation.
//!
//! `fireThreshold` keeps Go's float32 number text.
//!
//! Deliberate difference: the Go-runtime fields `goroutines`, `heapAllocBytes`
//! and `numGC` have no Rust runtime equivalent and are not emitted.
use humanymous_core::model::correction::Controller;
use serde_json::{Map, Value, json};

/// Inputs for one rollup.
pub struct Inputs<'a> {
    /// Product version string.
    pub version: &'a str,
    /// Seconds since process start.
    pub uptime_sec: u64,
    /// Loaded lab correction controller, if any.
    pub controller: Option<&'a Controller>,
    /// Active bundle version, if a bundle manager is loaded.
    pub active_bundle: Option<&'a str>,
    /// Oracle traces written by the trace sink; 0 without a sink.
    pub oracle_traces: u64,
    /// `Registry::fleet_fallbacks`; 0 without fleet correlation.
    pub fleet_fallbacks: u64,
}

/// Build the rollup document.
pub fn rollup(inputs: &Inputs<'_>) -> Value {
    let mut ml = Map::new();
    ml.insert("enabled".into(), Value::Bool(inputs.controller.is_some()));
    if let Some(controller) = inputs.controller {
        let snapshot = controller.snapshot();
        ml.insert("canaryState".into(), json!(snapshot.canary.state.as_str()));
        if !snapshot.canary.reason.is_empty() {
            ml.insert("canaryReason".into(), json!(snapshot.canary.reason));
        }
        ml.insert(
            "fireThreshold".into(),
            go_float32(controller.proposed_fire_threshold()),
        );
        if let Some(bundle) = inputs.active_bundle {
            ml.insert("activeBundle".into(), json!(bundle));
        }
    }
    ml.insert("oracleTraces".into(), json!(inputs.oracle_traces));
    json!({
        "version": inputs.version,
        "uptimeSec": inputs.uptime_sec,
        "ml": Value::Object(ml),
        "fleet": { "redisFallbackTotal": inputs.fleet_fallbacks },
    })
}

/// Go encodes the float32 threshold with 32-bit shortest digits (`0.8999963`);
/// widening the bits to f64 would print `0.8999962806701660`. The shortest
/// float32 decimal is carried as the f64 nearest to it, whose shortest text is
/// the same digits. A non-finite value (which Go refuses to encode) is `null`.
fn go_float32(value: f32) -> Value {
    value
        .to_string()
        .parse::<f64>()
        .ok()
        .and_then(serde_json::Number::from_f64)
        .map_or(Value::Null, Value::Number)
}

#[cfg(test)]
mod tests;

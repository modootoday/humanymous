//! Frozen signal predicates over observations; no DOM, time, I/O or engine copy.
mod fingerprint;
mod integrity;
mod snapshot;
mod static_client;
use humanymous_signals::{
    observation::{Signal, Source, Verdict},
    report::ClientReport,
};
use serde_json::Value;
pub use snapshot::{Getter, Snapshot};

fn add(
    out: &mut Vec<Signal>,
    id: &str,
    value: Value,
    verdict: Verdict,
    confidence: f64,
    notes: &str,
) {
    out.push(Signal::new(
        id,
        value,
        verdict,
        confidence,
        Source::Wasm,
        notes,
    ));
}
/// Evaluate the existing L1-L3 conditions. The result deliberately leaves all
/// unmeasured collector planes at defaults and does not create a session ID.
pub fn detect(snapshot: &Snapshot) -> Result<ClientReport, &'static str> {
    if snapshot.widths.is_none() {
        return Err("missing viewport observation");
    }
    if snapshot.memory.is_some_and(|v| v > 0.0 && !v.is_finite()) {
        return Err("nonserializable memory observation");
    }
    let mut signals = Vec::new();
    static_client::collect(snapshot, &mut signals);
    fingerprint::collect(snapshot, &mut signals);
    integrity::collect(snapshot, &mut signals);
    Ok(ClientReport {
        user_agent: snapshot.user_agent.clone(),
        ua_client_hints: snapshot.hints.clone(),
        signals: Some(signals),
        engine_version: concat!("rust-wasm-", env!("CARGO_PKG_VERSION")).into(),
        ..Default::default()
    })
}
#[cfg(test)]
mod tests;

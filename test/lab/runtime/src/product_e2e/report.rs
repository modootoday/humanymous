//! The collect report the product drivers submit (unit 35 increment 4).
//!
//! It is the exact entity the real Rust WASM collector
//! (`BrowserCollector::snapshot_json`) posted from Firefox ESR in the browser
//! topology, as received at the browser Gate frontend. The runner hands it to
//! each driver as one stdin line. The only change a driver makes is to set
//! `sessionId` to the session it collects for, because the native collect
//! route binds the report by that field. There is no fixture fallback: without
//! a captured report every collected session fails.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{BufRead, Read};

/// Wrapper schema the runner writes on the driver's stdin.
pub const SCHEMA: &str = "humanymous.product-e2e-collector-report/1";
/// Upper bound on one wrapper line.
const LINE_MAX: u64 = 128 * 1024;

/// A verified captured report plus its receipt-safe provenance.
pub struct Captured {
    report: Value,
    pub provenance: Value,
}

impl Captured {
    /// Collect entity for one session: the captured report, `sessionId` bound.
    pub fn bind(&self, session: &str) -> Vec<u8> {
        let mut report = self.report.clone();
        report["sessionId"] = json!(session);
        serde_json::to_vec(&report).expect("captured report reserializes")
    }
}

/// Verify the wrapper against its own digest and the collector's identity.
pub fn decode(line: &str) -> Result<Captured, String> {
    let wrapper: Value = serde_json::from_str(line.trim_end()).map_err(|_| "report wrapper")?;
    if wrapper["schema"] != SCHEMA {
        return Err("report wrapper schema".into());
    }
    let bytes = STANDARD
        .decode(wrapper["base64"].as_str().ok_or("report bytes absent")?)
        .map_err(|_| "report bytes encoding")?;
    let digest = hex::encode(Sha256::digest(&bytes));
    if wrapper["sha256"] != digest.as_str() || wrapper["bytes"] != bytes.len() {
        return Err("report digest or length mismatch".into());
    }
    let report: Value = serde_json::from_slice(&bytes).map_err(|_| "report JSON")?;
    let engine = report["engineVersion"].as_str().unwrap_or("");
    if !report.is_object()
        || !engine.starts_with("rust-wasm-")
        || !report["sessionId"].as_str().is_some_and(str::is_empty)
        || !report["signals"].is_array()
    {
        return Err("not an unbound Rust WASM collector report".into());
    }
    let signals: Vec<Value> = report["signals"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| json!({"id":s["id"],"verdict":s["verdict"],"collected":s["collected"]}))
        .collect();
    let provenance = json!({"source":wrapper["source"],"sha256":digest,"bytes":bytes.len(),
        "engine_version":engine,"user_agent":report["userAgent"],"signals":signals,
        "guard_reported":report["guard"]["reported"],
        "binding":"sessionId set to each collected session; every other field as captured",
        "fixture_fallback":false});
    Ok(Captured { report, provenance })
}

/// Read the one wrapper line the runner supplies on stdin.
pub fn from_stdin() -> Result<Captured, String> {
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .take(LINE_MAX)
        .read_line(&mut line)
        .map_err(|_| "report stdin")?;
    if line.is_empty() {
        return Err("no captured collector report supplied".into());
    }
    decode(&line)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The runner's wrapper shape (`test/lab/runner/src/product_e2e/browser.rs`).
    fn wrapper(base64: &str, sha256: &str, bytes: u64, source: &str) -> String {
        json!({"schema":SCHEMA,"base64":base64,"sha256":sha256,"bytes":bytes,"source":source})
            .to_string()
    }
    fn line(report: &Value) -> String {
        let bytes = serde_json::to_vec(report).unwrap();
        wrapper(
            &STANDARD.encode(&bytes),
            &hex::encode(Sha256::digest(&bytes)),
            bytes.len() as u64,
            "test",
        )
    }
    fn collector() -> Value {
        json!({"sessionId":"","engineVersion":"rust-wasm-0.1.0","userAgent":"ua",
            "signals":[{"id":"l1.navigator.webdriver","verdict":"BOT","collected":"wasm"}],
            "guard":{"reported":true}})
    }
    #[test]
    fn decode_verifies_digest_and_collector_identity() {
        let captured = decode(&line(&collector())).unwrap();
        assert_eq!(captured.provenance["signals"][0]["verdict"], "BOT");
        let bound: Value = serde_json::from_slice(&captured.bind("s")).unwrap();
        assert_eq!(bound["sessionId"], "s");
        assert_eq!(bound["signals"], collector()["signals"]);
        let mut tampered: Value = serde_json::from_str(&line(&collector())).unwrap();
        tampered["sha256"] = json!("00");
        assert!(decode(&tampered.to_string()).is_err());
        let mut fixture = collector();
        fixture["engineVersion"] = json!("fixture");
        assert!(decode(&line(&fixture)).is_err());
        let mut bound = collector();
        bound["sessionId"] = json!("already-bound");
        assert!(decode(&line(&bound)).is_err());
        assert!(decode("").is_err());
    }
}

//! Recover only manifest-bound completed rows; interruption can never publish success.
use humanymous_platform::testing::json::decode_strict_json;
use serde_json::{Value, json};
use std::{
    io::{self, Read},
    path::Path,
};
const MAX_BYTES: usize = 8 * 1024 * 1024;

pub(crate) fn read(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err(io::Error::other("coverage sidecar exceeds bound"));
    }
    Ok(bytes)
}
pub(crate) fn recover(bytes: &[u8], manifest: &Value) -> io::Result<Value> {
    let mut value = decode_strict_json(bytes, MAX_BYTES)
        .map_err(|_| io::Error::other("invalid coverage sidecar"))?;
    let entries = manifest["harnesses"]
        .as_array()
        .ok_or_else(|| io::Error::other("recovery manifest absent"))?;
    let rows = value["harnesses"]
        .as_array()
        .ok_or_else(|| io::Error::other("recovery rows absent"))?;
    if value["required_harnesses"].as_u64() != Some(entries.len() as u64)
        || value["executed_harnesses"].as_u64() != Some(rows.len() as u64)
        || rows.len() > entries.len()
    {
        return Err(io::Error::other("recovery denominator mismatch"));
    }
    for (index, row) in rows.iter().enumerate() {
        let entry = &entries[index];
        if row["index"].as_u64() != Some(index as u64)
            || ["target", "package_id"]
                .iter()
                .any(|key| row[*key] != entry[*key])
            || row["executable_sha256"] != entry["sha256"]
            || !row["validated"].is_boolean()
            || ["discovered", "passed", "failed", "ignored", "filtered"]
                .iter()
                .any(|key| !row[*key].is_null() && !row[*key].is_u64())
        {
            return Err(io::Error::other("recovery row identity or count mismatch"));
        }
    }
    value["complete"] = json!(false);
    value["execution_complete"] = json!(false);
    value["fatal_stop"] = json!(true);
    value["recovered_after_missing_final_report"] = json!(true);
    value["failures"] = json!([{"index":null,"target":null,"code":"final-report-absent"}]);
    Ok(value)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Value, Value) {
        let entry = json!({"target":"native_process","package_id":"owned","sha256":"a".repeat(64)});
        let manifest = json!({"harnesses":[entry]});
        let row = json!({"index":0,"target":"native_process","package_id":"owned","executable_sha256":"a".repeat(64),"validated":true,"discovered":2,"passed":1,"failed":null,"ignored":null,"filtered":0});
        (
            manifest,
            json!({"required_harnesses":1,"executed_harnesses":1,"harnesses":[row],"complete":true,"execution_complete":true}),
        )
    }
    #[test]
    fn preserves_known_counts_and_unknowns_but_never_recovers_success() {
        let (manifest, value) = fixture();
        let recovered = recover(&serde_json::to_vec(&value).unwrap(), &manifest).unwrap();
        assert_eq!(recovered["harnesses"][0]["passed"], 1);
        assert!(recovered["harnesses"][0]["failed"].is_null());
        assert_eq!(recovered["complete"], false);
        assert_eq!(recovered["execution_complete"], false);
        assert_eq!(recovered["fatal_stop"], true);
    }
    #[test]
    fn rejects_duplicate_keys_wrong_identity_and_inconsistent_denominators() {
        let (manifest, value) = fixture();
        for field in [
            "index",
            "target",
            "package_id",
            "executable_sha256",
            "passed",
        ] {
            let mut bad = value.clone();
            bad["harnesses"][0][field] = json!("invalid");
            assert!(recover(&serde_json::to_vec(&bad).unwrap(), &manifest).is_err());
        }
        let mut bad = value;
        bad["executed_harnesses"] = json!(0);
        assert!(recover(&serde_json::to_vec(&bad).unwrap(), &manifest).is_err());
        assert!(recover(br#"{"harnesses":[],"harnesses":[]}"#, &manifest).is_err());
    }
}

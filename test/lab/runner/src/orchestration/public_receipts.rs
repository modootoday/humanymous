//! Small public projections; no private actor/session/process identity or body.
use crate::{docker::Result, snapshot};
use serde_json::{Value, json};
pub(super) fn wargame(
    receipt: &Value,
    report: &Value,
    source: &Value,
    source_hash: &str,
    raw: &[u8],
) -> Result<Value> {
    let contract: Value = serde_json::from_str(include_str!("../../public-receipts.json"))?;
    if receipt["schema"] != "humanymous.lab-wargame-byte-receipt/1"
        || report["schema"] != contract["wargame_report_schema"]
        || receipt["verification"]["schema"] != contract["verifier_schema"]
        || receipt["verification"]["mode"] != contract["verifier_mode"]
        || receipt["verification"]["internally_consistent"] != true
        || receipt["verification"]["full_product_qualified"] != false
        || report["full_product_qualified"] != false
        || receipt["verification"]["scoped_passed"] != report["scoped_passed"]
        || receipt["raw_sha256"] != snapshot::digest(raw)
    {
        return Err("public wargame validation boundary".into());
    }
    if source_hash.len() != 64
        || !source_hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("public source-manifest SHA256".into());
    }
    let scoped = receipt["verification"]["scoped_passed"]
        .as_bool()
        .ok_or("public scoped status")?;
    let mut counts = [0u64; 6];
    let mut statuses = Vec::new();
    for (index, row) in report["rows"]
        .as_array()
        .ok_or("public wargame ledger missing")?
        .iter()
        .enumerate()
    {
        if index >= 512 {
            return Err("public wargame ledger bound".into());
        }
        counts[0] += 1;
        let status = row["status"].as_str().ok_or("public wargame status")?;
        let expected = match status {
            "evaluated" => {
                counts[1] += 1;
                let matches = row["expectation_matches"]
                    .as_bool()
                    .ok_or("evaluated public expectation")?;
                counts[5] += u64::from(!matches);
                json!(matches)
            }
            "error" | "skipped" | "unsupported" => {
                if !row["expectation_matches"].is_null() {
                    return Err("unexecuted public expectation must be null".into());
                }
                counts[match status {
                    "error" => 2,
                    "skipped" => 3,
                    _ => 4,
                }] += 1;
                Value::Null
            }
            _ => return Err("unknown public terminal status".into()),
        };
        statuses.push(json!({"index":index,"status":status,"expectation_matches":expected}));
    }
    let totals = json!({"scheduled":counts[0],"evaluated":counts[1],"errors":counts[2],"skipped":counts[3],"unsupported":counts[4],"expectation_mismatches":counts[5]});
    if totals != receipt["verification"]["totals"]
        || totals != report["totals"]
        || scoped != (counts[1] > 0 && counts[2] == 0 && counts[3] == 0 && counts[5] == 0)
    {
        return Err("public wargame ledger totals/scoped mismatch".into());
    }
    let head = source["head"]
        .as_str()
        .filter(|s| {
            matches!(s.len(), 40 | 64)
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        .ok_or("public source SHA")?;
    Ok(
        json!({"schema":contract["wargame_schema"],"report_schema":contract["wargame_report_schema"],"raw_sha256":snapshot::digest(raw),"source_sha":head,"source_manifest_sha256":source_hash,"verifier":{"schema":contract["verifier_schema"],"mode":contract["verifier_mode"],"internally_consistent":true,"scoped_passed":scoped,"full_product_qualified":false,"totals":totals},"case_statuses":statuses,"raw_contents_included":false,"full_product_qualified":false}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_drops_identity_and_body_and_rejects_false_accounting() {
        let totals = json!({"scheduled":2,"evaluated":1,"errors":0,"skipped":0,"unsupported":1,"expectation_mismatches":0});
        let report = json!({"schema":"humanymous.wargame-network.v2","scoped_passed":true,"full_product_qualified":false,"totals":totals,"rows":[{"status":"evaluated","expectation_matches":true,"id":"PRIVATE_SESSION","body":"PRIVATE_BODY"},{"status":"unsupported","expectation_matches":null,"processes":"PRIVATE_PROCESS"}]});
        let raw = serde_json::to_vec(&report).unwrap();
        let mut receipt = json!({"schema":"humanymous.lab-wargame-byte-receipt/1","raw_sha256":snapshot::digest(&raw),"verification":{"schema":"humanymous.wargame-verification.v1","mode":"owned_process_network","internally_consistent":true,"scoped_passed":true,"full_product_qualified":false,"totals":totals}});
        let source = json!({"head":"a".repeat(40)});
        let manifest = "b".repeat(64);
        let public = wargame(&receipt, &report, &source, &manifest, &raw).unwrap();
        assert_eq!(public["case_statuses"].as_array().unwrap().len(), 2);
        assert!(!public.to_string().contains("PRIVATE"));
        receipt["verification"]["totals"]["unsupported"] = json!(0);
        assert!(wargame(&receipt, &report, &source, &manifest, &raw).is_err());
        assert!(wargame(&receipt, &report, &source, &manifest, b"different raw").is_err());
    }
}

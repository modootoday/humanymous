//! Validate the lab's sole public receipt contract before exposing any fields.
use crate::{Result, require};
use serde_json::{Value, json};

pub(crate) fn fields(value: &Value, names: &Value) -> Result<()> {
    let object = value.as_object().ok_or("receipt object absent")?;
    let names = names.as_array().ok_or("receipt field contract absent")?;
    require(
        object.len() == names.len()
            && object
                .keys()
                .all(|key| names.iter().any(|name| name.as_str() == Some(key.as_str()))),
        "receipt fields mismatch",
    )
}
pub(crate) fn digest(value: &Value, size: usize) -> bool {
    value.as_str().is_some_and(|s| {
        s.len() == size
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
pub fn validate(report: &Value, head: &str, manifest: &str) -> Result<Option<Value>> {
    let Some(receipts) = report.get("public_receipts") else {
        return Ok(None);
    };
    let contract: Value = serde_json::from_str(include_str!(
        "../../../../test/lab/runner/public-receipts.json"
    ))
    .map_err(|e| e.to_string())?;
    require(
        contract["schema"] == "humanymous.lab-public-receipts-contract/1"
            && receipts["schema"] == contract["report_schema"],
        "receipt schema mismatch",
    )?;
    require(
        receipts
            .as_object()
            .is_some_and(|o| o.keys().all(|k| matches!(k.as_str(), "schema" | "wargame"))),
        "unknown public receipt field",
    )?;
    let Some(value) = receipts.get("wargame") else {
        return Ok(None);
    };
    fields(value, &contract["wargame_fields"])?;
    require(
        value["schema"] == contract["wargame_schema"]
            && value["report_schema"] == contract["wargame_report_schema"]
            && value["source_sha"] == head
            && value["source_manifest_sha256"] == manifest
            && digest(&value["source_sha"], head.len())
            && matches!(head.len(), 40 | 64)
            && digest(&value["source_manifest_sha256"], 64)
            && digest(&value["raw_sha256"], 64)
            && value["raw_contents_included"] == false
            && value["full_product_qualified"] == false,
        "receipt binding mismatch",
    )?;
    let verifier = &value["verifier"];
    fields(verifier, &contract["verifier_fields"])?;
    fields(&verifier["totals"], &contract["totals_fields"])?;
    require(
        verifier["schema"] == contract["verifier_schema"]
            && verifier["mode"] == contract["verifier_mode"]
            && verifier["internally_consistent"] == true
            && verifier["full_product_qualified"] == false,
        "receipt verifier mismatch",
    )?;
    let rows = value["case_statuses"]
        .as_array()
        .ok_or("receipt ledger absent")?;
    // This required hosted profile is the current 26-case owned deployment catalog.
    require(
        rows.len() == 26
            && rows.len() as u64
                <= contract["max_cases"]
                    .as_u64()
                    .ok_or("receipt bound absent")?,
        "receipt denominator mismatch",
    )?;
    let mut counts = [0u64; 6];
    for (index, row) in rows.iter().enumerate() {
        fields(row, &contract["case_fields"])?;
        require(
            row["index"].as_u64() == Some(index as u64),
            "receipt index mismatch",
        )?;
        counts[0] += 1;
        match row["status"].as_str() {
            Some("evaluated") => {
                counts[1] += 1;
                counts[5] += u64::from(
                    !row["expectation_matches"]
                        .as_bool()
                        .ok_or("evaluated expectation absent")?,
                );
            }
            Some(status @ ("error" | "skipped" | "unsupported")) => {
                require(
                    row["expectation_matches"].is_null(),
                    "unexecuted expectation must be null",
                )?;
                counts[match status {
                    "error" => 2,
                    "skipped" => 3,
                    _ => 4,
                }] += 1;
            }
            _ => return Err("unknown receipt terminal status".into()),
        }
    }
    for (index, name) in contract["totals_fields"]
        .as_array()
        .ok_or("receipt totals absent")?
        .iter()
        .enumerate()
    {
        require(
            verifier["totals"][name.as_str().ok_or("receipt total name")?].as_u64()
                == Some(counts[index]),
            "receipt totals mismatch",
        )?;
    }
    let scoped = counts[1] > 0 && counts[2] == 0 && counts[3] == 0 && counts[5] == 0;
    require(
        verifier["scoped_passed"].as_bool() == Some(scoped),
        "receipt scoped status mismatch",
    )?;
    Ok(Some(value.clone()))
}
pub fn public(report: &Value, head: &str, manifest: &str) -> Value {
    match validate(report, head, manifest) {
        Ok(Some(value)) => value,
        Ok(None) => Value::Null,
        Err(_) => json!({"status":"rejected","error_code":"invalid_lab_public_receipt"}),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        json!({"public_receipts":{"schema":"humanymous.lab-public-receipts/1","wargame":{
            "schema":"humanymous.lab-wargame-public/1","report_schema":"humanymous.wargame-network.v2",
            "raw_sha256":"c".repeat(64),"source_sha":"a".repeat(40),"source_manifest_sha256":"b".repeat(64),
            "verifier":{"schema":"humanymous.wargame-verification.v1","mode":"owned_process_network","internally_consistent":true,"scoped_passed":true,"full_product_qualified":false,"totals":{"scheduled":26,"evaluated":26,"errors":0,"skipped":0,"unsupported":0,"expectation_mismatches":0}},
            "case_statuses":(0..26).map(|i| json!({"index":i,"status":"evaluated","expectation_matches":true})).collect::<Vec<_>>(),
            "raw_contents_included":false,"full_product_qualified":false
        }}})
    }
    #[test]
    fn bound_receipt_survives_outer_failure_without_raw_fields() {
        let mut report = fixture();
        report["status"] = json!("failed");
        report["private_body"] = json!("PRIVATE_CANARY");
        let result = public(&report, &"a".repeat(40), &"b".repeat(64));
        assert_eq!(result["case_statuses"].as_array().unwrap().len(), 26);
        assert_eq!(result["full_product_qualified"], false);
        assert!(!result.to_string().contains("PRIVATE_CANARY"));
        assert!(validate(&report, &"d".repeat(40), &"b".repeat(64)).is_err());
    }
    #[test]
    fn malformed_ledgers_and_added_fields_fail_closed() {
        for mutation in 0..7 {
            let mut report = fixture();
            let receipt = &mut report["public_receipts"]["wargame"];
            match mutation {
                0 => receipt["case_statuses"][1]["index"] = json!(0),
                1 => receipt["case_statuses"][0]["status"] = json!("not_run"),
                2 => receipt["case_statuses"][0]["expectation_matches"] = Value::Null,
                3 => receipt["verifier"]["totals"]["evaluated"] = json!(25),
                4 => receipt["verifier"]["scoped_passed"] = json!(false),
                5 => receipt["case_statuses"]
                    .as_array_mut()
                    .unwrap()
                    .truncate(25),
                _ => receipt["private_body"] = json!("PRIVATE_CANARY"),
            }
            assert!(validate(&report, &"a".repeat(40), &"b".repeat(64)).is_err());
            assert!(
                !public(&report, &"a".repeat(40), &"b".repeat(64))
                    .to_string()
                    .contains("PRIVATE_CANARY")
            );
        }
    }
}

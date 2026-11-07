use serde_json::{Value, json};
use std::io;
mod observations;

pub(super) fn config() -> io::Result<Value> {
    let value = super::super::desktop::contract()?["bootstrap_rejection"].clone();
    if value["observation_schema"] != 1
        || value["case_count"] != 4
        || value["test_name"] != super::super::CASE
        || value["max_record_bytes"] != 1024
        || value["expected_exit_code"] != 78
        || value["expected_profile_changed"] != false
        || value["deadline_ms"] != 10000
    {
        return Err(io::Error::other(
            "unsupported bootstrap observation contract",
        ));
    }
    Ok(value)
}
pub(super) fn observations(bytes: &[u8], truncated: bool) -> Value {
    observations::read(bytes, truncated)
}
pub(super) fn public_observations(value: &Value) -> Value {
    observations::public(value)
}
pub(super) fn selector() -> io::Result<String> {
    let profile = config()?["protocol_preflight"].clone();
    if profile["selector"] != "negative_pilot::tests::"
        || profile["required_test_count"] != 3
        || profile["classification"] != "synthetic_bootstrap_observation_protocol"
        || profile["initializes_cef"] != false
        || profile["starts_processes"] != false
    {
        return Err(io::Error::other("unsupported bootstrap protocol preflight"));
    }
    Ok("negative_pilot::tests::".into())
}
pub(super) fn verify_protocol(text: &str) -> io::Result<Value> {
    selector()?;
    let profile = config()?["protocol_preflight"].clone();
    let names: Vec<_> = profile["test_names"]
        .as_array()
        .ok_or_else(|| io::Error::other("bootstrap protocol names absent"))?
        .iter()
        .filter_map(|name| name.as_str().map(str::to_owned))
        .collect();
    if names.len() != 3
        || names
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != 3
        || names.iter().any(|name| {
            !name.starts_with("negative_pilot::tests::")
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
        })
    {
        return Err(io::Error::other("invalid bootstrap protocol names"));
    }
    let mut result = super::super::desktop::verify_names(text, &names)?;
    result["classification"] = profile["classification"].clone();
    Ok(result)
}

pub(super) fn outcome(bytes: &[u8], exit: Option<i32>, truncated: bool) -> Value {
    let observed = (|| -> Option<Value> {
        config().ok()?;
        if truncated {
            return None;
        }
        let text = std::str::from_utf8(bytes).ok()?;
        let footers: Vec<_> = text
            .lines()
            .filter(|line| line.starts_with("test result:"))
            .collect();
        if footers.len() != 1 {
            return None;
        }
        let success = footers[0].starts_with("test result: ok. ");
        let tail = footers[0]
            .strip_prefix("test result: ok. ")
            .or_else(|| footers[0].strip_prefix("test result: FAILED. "))?;
        let columns: Vec<_> = tail.split(';').take(5).collect();
        if columns.len() != 5 {
            return None;
        }
        let mut counts = Vec::new();
        for (column, suffix) in columns.iter().zip([
            " passed",
            " failed",
            " ignored",
            " measured",
            " filtered out",
        ]) {
            let number = column.trim().strip_suffix(suffix)?.parse::<u64>().ok()?;
            if number > 1_000_000 {
                return None;
            }
            counts.push(number);
        }
        let pass_line = format!("test {} ... ok", super::super::CASE);
        let fail_line = format!("test {} ... FAILED", super::super::CASE);
        let passed = text.lines().filter(|line| *line == pass_line).count();
        let failed = text.lines().filter(|line| *line == fail_line).count();
        let coherent = counts[2] == 0
            && counts[3] == 0
            && if success {
                passed == 1 && failed == 0 && counts[0] == 1 && counts[1] == 0 && exit == Some(0)
            } else {
                passed == 0
                    && failed == 1
                    && counts[0] == 0
                    && counts[1] == 1
                    && exit.is_some_and(|exit| exit != 0)
            };
        if !coherent {
            return None;
        }
        Some(
            json!({"required_case_status":if success{"passed"}else{"failed"},"executed_tests":1,
            "passed":counts[0],"failed":counts[1],"ignored":0,"filtered":counts[4]}),
        )
    })();
    observed.unwrap_or_else(|| {
        json!({"required_case_status":"unverified","executed_tests":null,
        "passed":null,"failed":null,"ignored":null,"filtered":null})
    })
}

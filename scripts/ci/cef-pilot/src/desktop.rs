//! Independent count checks; process identities and native reports stay private.
use serde_json::{Value, json};
use std::io;
fn contract() -> io::Result<Value> {
    Ok(serde_json::from_str(include_str!(
        "../../../../crates/browser/native/desktop/pilot-contract.json"
    ))?)
}
pub(super) fn verify(log: &str) -> io::Result<Value> {
    let contract = contract()?;
    if contract["required_test_count"] != 2
        || contract["cases"]
            .as_object()
            .is_none_or(|cases| cases.len() != 2)
    {
        return Err(io::Error::other("unsupported desktop pilot contract"));
    }
    let cases = ["positive", "cancelled"]
        .map(|label| contract["cases"][label]["test_name"].as_str().unwrap_or(""));
    let summaries: Vec<_> = log
        .lines()
        .filter(|line| line.starts_with("test result:"))
        .collect();
    if summaries.len() != 1
        || !summaries[0].starts_with("test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured;")
        || cases
            .iter()
            .any(|name| name.is_empty() || log.matches(&format!("test {name} ... ")).count() != 1)
    {
        return Err(io::Error::other(
            "exact two required desktop cases were not executed",
        ));
    }
    let rows: Vec<Value> = log
        .lines()
        .filter_map(|line| {
            line.find('{')
                .and_then(|start| serde_json::from_str(&line[start..]).ok())
        })
        .filter(|row: &Value| row.get("case").is_some())
        .collect();
    if rows.len() != 2 {
        return Err(io::Error::other(
            "desktop case receipts absent or duplicated",
        ));
    }
    let mut safe = Vec::new();
    for label in ["positive", "cancelled"] {
        let found: Vec<_> = rows.iter().filter(|row| row["case"] == label).collect();
        if found.len() != 1 {
            return Err(io::Error::other("desktop case identity mismatch"));
        }
        let row = found[0];
        let expected = &contract["cases"][label];
        if row["summary_schema"] != contract["summary_schema"]
            || row["full_runtime_qualified"] != false
            || !row["host_pid"]
                .as_u64()
                .is_some_and(|pid| pid > 0 && u32::try_from(pid).is_ok())
            || expected["observed"].as_u64().is_none()
            || row["observed"] != expected["observed"]
            || !row["joined_renderer_probes"].as_u64().is_some_and(|count| {
                expected["joined_renderer_probes_min"]
                    .as_u64()
                    .is_some_and(|min| count >= min)
                    && expected["joined_renderer_probes_max"]
                        .as_u64()
                        .is_some_and(|max| count <= max)
            })
        {
            return Err(io::Error::other("invalid bounded desktop receipt"));
        }
        safe.push(json!({"case":label,"observed":row["observed"],"joined_renderer_probes":row["joined_renderer_probes"],"full_runtime_qualified":false}));
    }
    Ok(json!(safe))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_actual_cases_and_exact_denominator_are_required() {
        let contract = contract().unwrap();
        let log = format!(
            "test {} ... {{\"case\":\"positive\",\"host_pid\":42,\"observed\":7,\"joined_renderer_probes\":1,\"full_runtime_qualified\":false}}\nok\ntest {} ... {{\"case\":\"cancelled\",\"host_pid\":43,\"observed\":0,\"joined_renderer_probes\":0,\"full_runtime_qualified\":false}}\nok\ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out;",
            contract["cases"]["positive"]["test_name"].as_str().unwrap(),
            contract["cases"]["cancelled"]["test_name"]
                .as_str()
                .unwrap()
        );
        let log = log.replace("\"case\":", "\"summary_schema\":1,\"case\":");
        let safe = verify(&log).unwrap();
        assert!(!safe.to_string().contains("host_pid"));
        assert!(verify(&log.replace("2 passed", "0 passed")).is_err());
        assert!(verify(&log.replace("0 ignored", "2 ignored")).is_err());
        assert!(verify(&format!("{log}\n{log}")).is_err());
        assert!(verify(&log.replace("cancelled", "positive")).is_err());
        assert!(verify(&log.replace("\"observed\":7", "\"observed\":0")).is_err());
        assert!(verify(&log.replace("\"observed\":0", "\"observed\":7")).is_err());
        assert!(
            verify(&log.replace(
                "\"joined_renderer_probes\":1",
                "\"joined_renderer_probes\":0"
            ))
            .is_err()
        );
    }
}

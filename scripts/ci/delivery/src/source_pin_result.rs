//! Required moved-source gate cardinality; exit status remains a separate gate.
use crate::{Result, require};
use std::{io::Read, path::Path};

const CASE: &str =
    "lab_runner::snapshot::source_pin::tests::actual_collector_source_matches_reviewed_pin";

fn accepted(text: &str) -> bool {
    let summaries: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("test result:"))
        .collect();
    let [summary] = summaries.as_slice() else {
        return false;
    };
    let Some(rest) =
        summary.strip_prefix("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; ")
    else {
        return false;
    };
    let Some((filtered, elapsed)) = rest.split_once(" filtered out; finished in ") else {
        return false;
    };
    filtered.parse::<u64>().is_ok_and(|n| n <= 100_000)
        && elapsed
            .strip_suffix('s')
            .and_then(|n| n.parse::<f64>().ok())
            .is_some_and(|n| n.is_finite() && n >= 0.0)
        && text
            .lines()
            .filter(|line| *line == "running 1 test")
            .count()
            == 1
        && text
            .lines()
            .filter(|line| *line == format!("test {CASE} ... ok"))
            .count()
            == 1
        && text
            .lines()
            .filter(|line| line.starts_with("test ") && !line.starts_with("test result:"))
            .count()
            == 1
}

pub fn check(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| "source-pin result missing")?;
    require(
        metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= 1_048_576,
        "source-pin result file refused",
    )?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| "source-pin result open failed")?
        .take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|_| "source-pin result read failed")?;
    require(bytes.len() <= 1_048_576, "source-pin result exceeds bound")?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "source-pin result UTF-8 refused")?;
    require(
        accepted(text),
        "required source-pin test cardinality/result refused",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> String {
        format!(
            "running 1 test\ntest {CASE} ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 45 filtered out; finished in 0.00s\n"
        )
    }
    #[test]
    fn one_actual_required_case_is_distinct_from_successful_empty_selection() {
        assert!(accepted(&valid()));
        for (before, after) in [
            ("running 1 test", "running 0 tests"),
            ("1 passed", "0 passed"),
            (
                CASE,
                "snapshot::source_pin::tests::actual_collector_source_matches_reviewed_pin",
            ),
            ("... ok", "... ignored"),
            ("0 failed", "1 failed"),
            ("0 ignored", "1 ignored"),
            ("45 filtered", "unknown filtered"),
            ("0.00s", "NaNs"),
            ("0.00s", "-1s"),
        ] {
            assert!(!accepted(&valid().replace(before, after)));
        }
    }
    #[test]
    fn duplicate_summaries_cases_and_other_executed_tests_are_refused() {
        let good = valid();
        assert!(!accepted(&format!("{good}{good}")));
        assert!(!accepted(&format!("{good}test {CASE} ... ok\n")));
        assert!(!accepted(&format!("{good}test unrelated ... ok\n")));
        assert!(!accepted(""));
    }
}

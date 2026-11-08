//! Required moved-source gate cardinality; exit status remains a separate gate.
use crate::{Result, require};
use std::{io::Read, path::Path};

const CASE: &str =
    "lab_runner::snapshot::source_pin::tests::actual_collector_source_matches_reviewed_pin";
const PROXY_CASES: &[&str] = &[
    "runtime_pilot::browser_proxy::tests::origin_classes_initialize_zeroes_and_never_retain_foreign_values",
    "runtime_pilot::browser_proxy::tests::wire_observations_count_only_fixed_classes_and_treat_empty_referer_as_present",
    "runtime_pilot::browser_proxy::tests::only_exact_owned_root_is_classified_as_private",
    "runtime_pilot::browser_proxy::tests::owned_authority_and_api_dispatch_cannot_be_retargeted",
];

fn accepted(text: &str) -> bool {
    accepted_cases(text, &[CASE])
}

fn accepted_cases(text: &str, cases: &[&str]) -> bool {
    let summaries: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("test result:"))
        .collect();
    let [summary] = summaries.as_slice() else {
        return false;
    };
    let prefix = format!(
        "test result: ok. {} passed; 0 failed; 0 ignored; 0 measured; ",
        cases.len()
    );
    let Some(rest) = summary.strip_prefix(&prefix) else {
        return false;
    };
    let Some((filtered, elapsed)) = rest.split_once(" filtered out; finished in ") else {
        return false;
    };
    !cases.is_empty()
        && filtered.bytes().all(|b| b.is_ascii_digit())
        && filtered.parse::<u64>().is_ok_and(|n| n <= 100_000)
        && elapsed
            .strip_suffix('s')
            .and_then(|n| n.parse::<f64>().ok())
            .is_some_and(|n| n.is_finite() && n >= 0.0)
        && text
            .lines()
            .filter(|line| {
                *line
                    == format!(
                        "running {} test{}",
                        cases.len(),
                        if cases.len() == 1 { "" } else { "s" }
                    )
            })
            .count()
            == 1
        && cases.iter().all(|case| {
            text.lines()
                .filter(|line| *line == format!("test {case} ... ok"))
                .count()
                == 1
        })
        && text
            .lines()
            .filter(|line| line.starts_with("test ") && !line.starts_with("test result:"))
            .count()
            == cases.len()
}

pub fn check(path: &Path) -> Result<()> {
    check_log(path, accepted)
}

pub fn check_proxy(path: &Path) -> Result<()> {
    check_log(path, |text| accepted_cases(text, PROXY_CASES))
}

fn check_log(path: &Path, accepts: impl FnOnce(&str) -> bool) -> Result<()> {
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
        accepts(text),
        "required source-pin test cardinality/result refused",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_four_proxy_cases_are_required_without_substitution_or_duplicates() {
        let cases: String = PROXY_CASES
            .iter()
            .map(|name| format!("test {name} ... ok\n"))
            .collect();
        let text = format!(
            "running 4 tests\n{cases}test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s\n"
        );
        assert!(accepted_cases(&text, PROXY_CASES));
        assert!(!accepted(&text));
        for case in PROXY_CASES {
            assert!(!accepted_cases(
                &text.replace(case, "unowned_case"),
                PROXY_CASES
            ));
            assert!(!accepted_cases(
                &format!("{text}test {case} ... ok\n"),
                PROXY_CASES
            ));
        }
        assert!(!accepted_cases(
            &text.replace("4 passed", "0 passed"),
            PROXY_CASES
        ));
    }
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

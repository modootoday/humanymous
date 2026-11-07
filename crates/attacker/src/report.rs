//! Complete attempt ledger; unavailable/error outcomes retain their denominators.
pub mod legacy;
use crate::catalog::Case;
use humanymous_defender::Evaluation;
use humanymous_signals::report::SessionReport;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Terminal status for every scheduled attempt.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Real engine produced a verdict (regression may still mismatch).
    Evaluated,
    /// Input or real engine failed, with no manufactured verdict.
    Error,
    /// A supported attempt was deliberately not executed.
    Skipped,
    /// Required runtime capability does not exist in this slice.
    Unsupported,
}

/// One auditable scheduled attempt.
#[derive(Serialize)]
pub struct Row {
    /// Stable catalog identity.
    pub id: String,
    /// T0–T4 or explicit control/runtime tier.
    pub tier: String,
    /// Independent ground-truth class.
    pub class: String,
    /// Source and scope of the observation.
    pub provenance: String,
    /// Exact original input, before engine mutation.
    pub input: Option<SessionReport>,
    /// Regression fields, never used to manufacture actual results.
    pub expected: Option<Value>,
    /// Exactly one terminal status.
    pub status: Status,
    /// Actual engine explanation and derived report, if evaluated.
    pub actual: Option<Evaluation>,
    /// Only evaluated attempts have a regression comparison.
    pub regression_matches: Option<bool>,
    /// Unavailability or failure detail; synthetic fixtures have no live PII.
    pub reason: Option<String>,
}

pub(crate) fn execute(case: Case) -> Row {
    let mut row = Row {
        id: case.id,
        tier: case.tier.into(),
        class: case.class.into(),
        provenance: case.provenance.into(),
        input: None,
        expected: None,
        status: Status::Error,
        actual: None,
        regression_matches: None,
        reason: None,
    };
    match case.input {
        Err(reason) => {
            row.status = if case.unsupported {
                Status::Unsupported
            } else {
                Status::Error
            };
            row.reason = Some(reason);
        }
        Ok((input, expected)) => {
            row.input = Some(input.clone());
            row.expected = Some(expected.clone());
            match humanymous_defender::evaluate(input) {
                Err(error) => row.reason = Some(error.to_string()),
                Ok(actual) => {
                    let encoded =
                        serde_json::to_value(&actual.result).expect("finite engine output");
                    row.regression_matches = Some(expected.as_object().is_some_and(|fields| {
                        !fields.is_empty()
                            && fields.iter().all(|(key, value)| {
                                encoded
                                    .get(key)
                                    .is_some_and(|actual| match (actual, value) {
                                        (Value::Number(left), Value::Number(right)) => {
                                            left.as_f64() == right.as_f64()
                                        }
                                        _ => actual == value,
                                    })
                            })
                    }));
                    row.status = Status::Evaluated;
                    row.actual = Some(actual);
                }
            }
        }
    }
    row
}

/// Counts always use scheduled attempts as denominator; verdicts stay separate.
#[derive(Debug, Default, Serialize)]
pub struct Counts {
    /// All scheduled attempts, including unavailable/error attempts.
    pub scheduled: usize,
    /// Real engine outcomes.
    pub evaluated: usize,
    /// Input/engine failures.
    pub errors: usize,
    /// Deliberately unexecuted attempts.
    pub skipped: usize,
    /// Missing capabilities.
    pub unsupported: usize,
    /// Explicit engine ALLOW.
    pub allow: usize,
    /// Explicit engine CHALLENGE (never counted as ALLOW).
    pub challenge: usize,
    /// Explicit engine DENY.
    pub deny: usize,
    /// Actual outcomes differing from regression expectations.
    pub regression_mismatches: usize,
}
impl Counts {
    fn add(&mut self, row: &Row) {
        self.scheduled += 1;
        match row.status {
            Status::Error => self.errors += 1,
            Status::Skipped => self.skipped += 1,
            Status::Unsupported => self.unsupported += 1,
            Status::Evaluated => {
                self.evaluated += 1;
                match row
                    .actual
                    .as_ref()
                    .expect("evaluated row has engine output")
                    .result
                    .verdict
                    .as_str()
                {
                    "ALLOW" => self.allow += 1,
                    "CHALLENGE" => self.challenge += 1,
                    "DENY" => self.deny += 1,
                    _ => unreachable!("engine verdict vocabulary"),
                }
                self.regression_mismatches += usize::from(row.regression_matches != Some(true));
            }
        }
    }
}

/// Machine-readable observation evidence; not a runtime enforcement claim.
#[derive(Serialize)]
pub struct RunReport {
    /// Stable output contract.
    pub schema: &'static str,
    /// Versioned catalog semantics.
    pub catalog: &'static str,
    /// Always pure_engine_simulation for this implementation.
    pub mode: &'static str,
    /// Ordering seed; no clock or random device is consumed.
    pub seed: u64,
    /// Explicit detection ceiling disclosure.
    pub t4_ceiling: &'static str,
    /// False until real Gate/identity/browser/egress tests exist.
    pub runtime_coverage_complete: bool,
    /// True only if every scheduled human/accessibility control explicitly ALLOWs.
    pub human_controls_all_allow: bool,
    /// All attempts.
    pub totals: Counts,
    /// Separate human, accessibility, attack, ceiling and runtime denominators.
    pub by_class: BTreeMap<String, Counts>,
    /// Separate cost-tier coverage.
    pub by_tier: BTreeMap<String, Counts>,
    /// Full scheduled ledger.
    pub rows: Vec<Row>,
}
impl RunReport {
    pub(crate) fn new(seed: u64, rows: Vec<Row>) -> Self {
        let mut report = Self {
            schema: "humanymous.wargame.v1",
            catalog: "observation-replay-v1",
            mode: "pure_engine_simulation",
            seed,
            t4_ceiling: "Coherent real-browser/human observations are intentionally not separable; replay is not a live T4 measurement",
            runtime_coverage_complete: false,
            human_controls_all_allow: false,
            totals: Counts::default(),
            by_class: BTreeMap::new(),
            by_tier: BTreeMap::new(),
            rows,
        };
        for row in &report.rows {
            report.totals.add(row);
            report
                .by_class
                .entry(row.class.clone())
                .or_default()
                .add(row);
            report.by_tier.entry(row.tier.clone()).or_default().add(row);
        }
        let controls: Vec<_> = report
            .rows
            .iter()
            .filter(|row| matches!(row.class.as_str(), "human" | "accessibility"))
            .collect();
        report.human_controls_all_allow = !controls.is_empty()
            && controls.iter().all(|row| {
                row.status == Status::Evaluated
                    && row
                        .actual
                        .as_ref()
                        .is_some_and(|actual| actual.result.verdict == "ALLOW")
            });
        report
    }
    /// Regression readiness only; runtime coverage is reported independently.
    pub fn regressions_pass(&self) -> bool {
        self.totals.evaluated > 0
            && self.totals.errors == 0
            && self.totals.skipped == 0
            && self.totals.regression_mismatches == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_and_failed_attempts_remain_in_denominator() {
        let mut rows = crate::run(1).rows;
        rows.push(execute(Case {
            id: "broken-fixture".into(),
            tier: "T0",
            class: "attack",
            provenance: "test",
            input: Err("invalid input".into()),
            unsupported: false,
        }));
        rows.push(Row {
            id: "skipped".into(),
            tier: "T0".into(),
            class: "attack".into(),
            provenance: "test".into(),
            input: None,
            expected: None,
            status: Status::Skipped,
            actual: None,
            regression_matches: None,
            reason: Some("test skip".into()),
        });
        let report = RunReport::new(1, rows);
        assert_eq!(report.totals.scheduled, 14);
        assert_eq!(report.totals.unsupported, 4);
        assert_eq!(report.totals.errors, 1);
        assert_eq!(report.totals.skipped, 1);
        assert_eq!(
            report.totals.evaluated
                + report.totals.errors
                + report.totals.skipped
                + report.totals.unsupported,
            report.totals.scheduled
        );
        assert!(!report.regressions_pass());
    }
    #[test]
    fn human_challenge_and_ceiling_allow_are_not_attack_successes() {
        let report = crate::run(42);
        assert_eq!(report.by_class["human"].allow, 1);
        assert_eq!(report.by_class["accessibility"].challenge, 1);
        assert_eq!(report.by_class["ceiling"].allow, 1);
        assert_eq!(report.by_class["attack"].scheduled, 7);
        assert_eq!(report.by_class["accessibility"].unsupported, 1);
        assert!(!report.human_controls_all_allow);
        assert!(report.regressions_pass());
        assert!(!report.runtime_coverage_complete);
    }
    #[test]
    fn wrong_expected_verdict_fails_without_changing_actual() {
        let mut case = crate::catalog::cases(0)
            .into_iter()
            .find(|case| case.id == "human-zero-motor")
            .unwrap();
        case.input.as_mut().unwrap().1["verdict"] = Value::String("DENY".into());
        let row = execute(case);
        assert_eq!(row.actual.as_ref().unwrap().result.verdict, "ALLOW");
        assert_eq!(row.regression_matches, Some(false));
        assert!(!RunReport::new(0, vec![row]).regressions_pass());
    }
}

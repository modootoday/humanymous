use super::cases;
use crate::report::Status;
use humanymous_defender::lab::{LabError, ProcessReceipt, SnapshotDelta};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// One actual request observation; no raw request/capability body is serialized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Scenario-local identity.
    pub id: String,
    /// Setup/control/attack are distinct even within a replay scenario.
    pub role: String,
    /// Actual HTTP status, absent on transport failure.
    pub http_status: Option<u16>,
    /// Explicit response verdict when present; never inferred from HTTP403.
    pub response_verdict: Option<String>,
    /// Safe local failure code; no response/request body.
    pub error: Option<String>,
}

/// One scheduled runtime case with actual origin/audit effects.
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkRow {
    /// Stable versioned catalog identity.
    pub id: String,
    /// Cost tier or control/runtime scope.
    pub tier: String,
    /// Synthetic control/attack/accessibility/ceiling class, not a human claim.
    pub class: String,
    /// Exactly one scheduled terminal status.
    pub status: Status,
    /// Every attempted case step, including failed transport.
    pub steps: Vec<Step>,
    /// Actual binary and installation observations for each spawned child.
    pub processes: Vec<ProcessReceipt>,
    /// Authenticated audit and independently observed origin delta after stop.
    pub effects: Option<SnapshotDelta>,
    /// True/false only for evaluated cases; unavailable/error is not a pass.
    pub expectation_matches: Option<bool>,
    /// Missing capability or execution failure, with no secret data.
    pub reason: Option<String>,
}

/// Complete scheduled denominator and explicit failure categories.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkTotals {
    /// All scheduled cases.
    pub scheduled: usize,
    /// Real process/network cases whose effects were observed.
    pub evaluated: usize,
    /// Setup/transport/audit failures.
    pub errors: usize,
    /// Supported cases deliberately not attempted (none in this catalog).
    pub skipped: usize,
    /// Missing runtime/participant capabilities.
    pub unsupported: usize,
    /// Evaluated results that differ from immutable case expectations.
    pub expectation_mismatches: usize,
}
impl NetworkTotals {
    fn add(&mut self, row: &NetworkRow) {
        self.scheduled += 1;
        match row.status {
            Status::Evaluated => {
                self.evaluated += 1;
                self.expectation_mismatches += usize::from(row.expectation_matches != Some(true));
            }
            Status::Error => self.errors += 1,
            Status::Skipped => self.skipped += 1,
            Status::Unsupported => self.unsupported += 1,
        }
    }
}

/// Network entropy and audit hashes are measured, not made deterministic by seed.
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkReport {
    /// Versioned machine-readable contract.
    pub schema: String,
    /// Explicitly narrow process trust boundary.
    pub evidence_type: String,
    /// Catalog ordering seed, not a source of runtime session keys.
    pub seed: u64,
    /// Synthetic controls never claim actual participant/browser execution.
    pub human_executed: bool,
    /// No real assistive technology participant has run in this catalog.
    pub assistive_technology_executed: bool,
    /// False until all real Gate/browser/Pass/human/AT gates are satisfied.
    pub full_product_qualified: bool,
    /// Separate scoped regression acceptance, recomputed by the verifier.
    pub scoped_passed: bool,
    /// Explicit ceiling disclosure.
    pub t4_ceiling: String,
    /// All scheduled terminal statuses.
    pub totals: NetworkTotals,
    /// Separate attack/synthetic/accessibility/unavailable denominators.
    pub by_class: BTreeMap<String, NetworkTotals>,
    /// Policy verdict observations from real audit only, excluding COLLECTED.
    pub audited_policy_verdicts: BTreeMap<String, usize>,
    /// Whole scheduled ledger, not only successful cases.
    pub rows: Vec<NetworkRow>,
}
impl NetworkReport {
    pub(super) fn new(seed: u64, rows: Vec<NetworkRow>) -> Self {
        let mut report = Self { schema:"humanymous.wargame-network.v2".into(), evidence_type:"supervisor-owned-runtime-child-http1".into(), seed, human_executed:false, assistive_technology_executed:false, full_product_qualified:false, scoped_passed:false, t4_ceiling:"Coherent real browser/human behavior remains an intentional detection ceiling; synthetic ALLOW is not a human or live T4 measurement".into(), totals:NetworkTotals::default(), by_class:BTreeMap::new(), audited_policy_verdicts:BTreeMap::new(), rows };
        for row in &report.rows {
            report.totals.add(row);
            report
                .by_class
                .entry(row.class.clone())
                .or_default()
                .add(row);
            if let Some(effects) = &row.effects {
                for event in &effects.audit {
                    if event.event_type == "runtime.decision" {
                        *report
                            .audited_policy_verdicts
                            .entry(event.verdict.clone())
                            .or_default() += 1;
                    }
                }
            }
        }
        report.scoped_passed = report.totals.evaluated > 0
            && report.totals.errors == 0
            && report.totals.skipped == 0
            && report.totals.expectation_mismatches == 0;
        report
    }

    /// Independently recompute case expectations, accounting and binary continuity.
    /// A valid failure report is accepted as internally consistent, not as passed.
    pub fn validate(&self) -> Result<(), LabError> {
        let fail = LabError("network report inconsistent");
        if self.schema != "humanymous.wargame-network.v2"
            || self.evidence_type != "supervisor-owned-runtime-child-http1"
            || self.human_executed
            || self.assistive_technology_executed
            || self.full_product_qualified
            || self.t4_ceiling.is_empty()
        {
            return Err(fail);
        }
        let mut specs = cases::catalog();
        crate::catalog::shuffle(&mut specs, self.seed);
        if specs.len() != self.rows.len() {
            return Err(fail);
        }
        let mut totals = NetworkTotals::default();
        let mut by_class = BTreeMap::<String, NetworkTotals>::new();
        let mut verdicts = BTreeMap::<String, usize>::new();
        let mut source = None;
        let mut seen_sequences = BTreeSet::new();
        for (row, spec) in self.rows.iter().zip(specs) {
            if row.id != spec.id || row.class != spec.class || row.tier != spec.tier {
                return Err(fail);
            }
            if spec.kind.is_none() && row.status != Status::Unsupported
                || spec.kind.is_some() && row.status == Status::Unsupported
            {
                return Err(fail);
            }
            match row.status {
                Status::Evaluated => {
                    if row.expectation_matches != Some(cases::matches(&spec, row))
                        || row.effects.is_none()
                        || row.processes.is_empty()
                        || row.reason.is_some()
                    {
                        return Err(fail);
                    }
                }
                Status::Unsupported => {
                    if row.effects.is_some()
                        || !row.steps.is_empty()
                        || !row.processes.is_empty()
                        || row.expectation_matches.is_some()
                        || row.reason.as_deref() != spec.reason
                    {
                        return Err(fail);
                    }
                }
                Status::Error | Status::Skipped => {
                    if row.expectation_matches.is_some()
                        || row.reason.as_ref().is_none_or(String::is_empty)
                    {
                        return Err(fail);
                    }
                }
            }
            for process in &row.processes {
                if process.binding
                    != "supervisor-owned-child/private-pipe/same-loopback-only-namespace"
                    || process.installation.is_empty()
                    || process.revision.is_empty()
                    || [&process.runtime_sha256, &process.supervisor_sha256]
                        .into_iter()
                        .any(|hash| {
                            hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit())
                        })
                {
                    return Err(fail);
                }
                if source.as_ref().is_some_and(|prior| prior != process) {
                    return Err(LabError("runtime source or scope changed within report"));
                }
                source = Some(process.clone());
            }
            for step in &row.steps {
                if step.http_status.is_none() != step.error.is_some() {
                    return Err(fail);
                }
            }
            if let Some(effects) = &row.effects {
                for event in &effects.audit {
                    if !seen_sequences.insert(event.seq)
                        || event.record_hash.len() != 64
                        || !event.record_hash.bytes().all(|b| b.is_ascii_hexdigit())
                    {
                        return Err(fail);
                    }
                    if event.event_type == "runtime.decision" {
                        if !matches!(event.verdict.as_str(), "ALLOW" | "CHALLENGE" | "DENY") {
                            return Err(fail);
                        }
                        *verdicts.entry(event.verdict.clone()).or_default() += 1;
                    }
                }
            }
            totals.add(row);
            by_class.entry(row.class.clone()).or_default().add(row);
        }
        let passed = totals.evaluated > 0
            && totals.errors == 0
            && totals.skipped == 0
            && totals.expectation_mismatches == 0;
        if totals != self.totals
            || by_class != self.by_class
            || verdicts != self.audited_policy_verdicts
            || passed != self.scoped_passed
        {
            return Err(fail);
        }
        Ok(())
    }
}

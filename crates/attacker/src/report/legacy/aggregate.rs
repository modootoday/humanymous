use super::{Error, MAX_ROWS, Record};
use serde::Serialize;
use std::collections::BTreeMap;

/// Exclusive terminal buckets plus an explicitly overlapping error-flag count.
#[derive(Default, Debug, Serialize)]
pub struct Accounting {
    /// Every input row, including missing labels and unavailable attempts.
    pub scheduled: usize,
    /// Exact frozen Go metric denominator population.
    pub eligible: usize,
    /// Legacy skip precedence, including skipped rows also marked error.
    pub skipped: usize,
    /// Non-skipped rows carrying an error.
    pub errors: usize,
    /// Non-skipped, error-free rows with no label.
    pub missing_label: usize,
    /// All nonempty error fields, including skipped rows; overlaps other buckets.
    pub error_marked: usize,
    /// Legacy input does not encode this count. Never manufacture zero.
    pub unsupported_count: Option<usize>,
}

/// An unavailable row's display-safe metadata, with free-form diagnostics omitted.
#[derive(Debug, Serialize)]
pub struct Notice {
    /// Imported profile name, rendered only as escaped text.
    pub profile: String,
    /// Imported label, not authenticated identity.
    pub label: String,
    /// Fixed diagnostic code; no imported reason/error is retained here.
    pub code: &'static str,
}

/// One first-seen eligible label's frozen legacy aggregates.
#[derive(Debug, Serialize)]
pub struct ProfileSummary {
    /// Declared grouping label.
    pub label: String,
    /// True exactly for the legacy bot: prefix.
    pub is_bot: bool,
    /// Eligible observations in this group.
    pub runs: usize,
    /// All verdict spellings, including unknown/no-response values.
    pub verdicts: BTreeMap<String, usize>,
    /// Arithmetic mean of imported risks, never newly scored.
    pub mean_risk: f64,
    /// Rule occurrences.
    pub rules: BTreeMap<String, usize>,
    /// Signal occurrences; repeated IDs in one row count repeatedly.
    pub top_hits: BTreeMap<String, usize>,
    /// Bot DENY+CHALLENGE count.
    pub blocked: usize,
}
impl ProfileSummary {
    /// Legacy frequency descending, then signal ID ascending.
    pub fn sorted_top_hits(&self) -> Vec<(&str, usize)> {
        let mut hits: Vec<_> = self
            .top_hits
            .iter()
            .map(|(id, count)| (id.as_str(), *count))
            .collect();
        hits.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
        hits
    }
}

/// Legacy declared-label metrics and the denominators the old report hid.
#[derive(Debug, Serialize)]
pub struct Summary {
    /// First eligible occurrence order, matching the Go oracle.
    pub profiles: Vec<ProfileSummary>,
    /// Blocked bots / eligible declared-bot rows; zero when population is empty.
    pub bot_tpr: f64,
    /// Denied humans / eligible declared-human rows; zero when population is empty.
    pub human_fpr: f64,
    /// Eligible bot rows.
    pub bot_runs: usize,
    /// Eligible non-bot rows, not verified human participants.
    pub human_runs: usize,
    /// Human CHALLENGE is tracked separately from denied-only FPR.
    pub human_challenges: usize,
    /// Every human verdict other than ALLOW, including NO_RESPONSE/unknown.
    pub human_non_allow: usize,
    /// Complete input accounting.
    pub accounting: Accounting,
    /// Failed/skipped/unlabelled rows without raw diagnostics.
    pub notices: Vec<Notice>,
    /// Always unknown for this legacy schema.
    pub unsupported_accounting: &'static str,
}

/// Preserve legacy eligible metrics while retaining incomplete rows separately.
pub fn aggregate(records: &[Record]) -> Result<Summary, Error> {
    if records.len() > MAX_ROWS {
        return Err(Error::Limit);
    }
    let mut summary = Summary {
        profiles: Vec::new(),
        bot_tpr: 0.0,
        human_fpr: 0.0,
        bot_runs: 0,
        human_runs: 0,
        human_challenges: 0,
        human_non_allow: 0,
        accounting: Accounting {
            scheduled: records.len(),
            ..Default::default()
        },
        notices: Vec::new(),
        unsupported_accounting: "not_encoded_in_legacy_v1",
    };
    let mut groups = BTreeMap::<&str, usize>::new();
    for record in records {
        record.validate()?;
        summary.accounting.error_marked += usize::from(!record.error.is_empty());
        let unavailable = if record.skipped {
            summary.accounting.skipped += 1;
            Some("skipped_diagnostic_redacted")
        } else if !record.error.is_empty() {
            summary.accounting.errors += 1;
            Some("error_diagnostic_redacted")
        } else if record.label.is_empty() {
            summary.accounting.missing_label += 1;
            Some("missing_label")
        } else {
            None
        };
        if let Some(code) = unavailable {
            summary.notices.push(Notice {
                profile: record.profile.clone(),
                label: record.label.clone(),
                code,
            });
            continue;
        }
        summary.accounting.eligible += 1;
        let index = *groups.entry(&record.label).or_insert_with(|| {
            let index = summary.profiles.len();
            summary.profiles.push(ProfileSummary {
                label: record.label.clone(),
                is_bot: record.label.starts_with("bot:"),
                runs: 0,
                verdicts: BTreeMap::new(),
                mean_risk: 0.0,
                rules: BTreeMap::new(),
                top_hits: BTreeMap::new(),
                blocked: 0,
            });
            index
        });
        let profile = &mut summary.profiles[index];
        profile.runs += 1;
        *profile.verdicts.entry(record.verdict.clone()).or_default() += 1;
        profile.mean_risk += record.risk_score;
        if !profile.mean_risk.is_finite() {
            return Err(Error::RiskOverflow);
        }
        if !record.hard_rule_fired.is_empty() {
            *profile
                .rules
                .entry(record.hard_rule_fired.clone())
                .or_default() += 1;
        }
        for id in &record.top {
            *profile.top_hits.entry(id.clone()).or_default() += 1;
        }
        if profile.is_bot && matches!(record.verdict.as_str(), "DENY" | "CHALLENGE") {
            profile.blocked += 1;
        }
    }
    let mut bot_blocked = 0;
    let mut human_denied = 0;
    for profile in &mut summary.profiles {
        profile.mean_risk /= profile.runs as f64;
        if profile.is_bot {
            summary.bot_runs += profile.runs;
            bot_blocked += profile.blocked;
        } else {
            summary.human_runs += profile.runs;
            human_denied += profile.verdicts.get("DENY").copied().unwrap_or(0);
            summary.human_challenges += profile.verdicts.get("CHALLENGE").copied().unwrap_or(0);
            summary.human_non_allow +=
                profile.runs - profile.verdicts.get("ALLOW").copied().unwrap_or(0);
        }
    }
    if summary.bot_runs > 0 {
        summary.bot_tpr = bot_blocked as f64 / summary.bot_runs as f64;
    }
    if summary.human_runs > 0 {
        summary.human_fpr = human_denied as f64 / summary.human_runs as f64;
    }
    Ok(summary)
}

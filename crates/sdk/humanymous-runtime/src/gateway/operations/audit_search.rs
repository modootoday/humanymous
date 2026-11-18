//! Server-side audit search with a `before` sequence cursor (SoT-28 WS9).
//!
//! Port of legacy `adminAudit`: records are scanned newest-first; filters are
//! exact verdict, case-insensitive host substring, exact route class, exact
//! rule, minimum risk and subject pseudonym (session or actor). `limit`
//! defaults to 50 and accepts 1..=500; anything else keeps the default, as in
//! the legacy handler. `next_before` is the sequence of the last returned
//! record, or 0 when nothing was returned.
use crate::audit::record::Record;

/// Default page size.
pub const DEFAULT_LIMIT: usize = 50;
/// Largest accepted page size.
pub const MAX_LIMIT: usize = 500;

/// Parsed query. Unknown or malformed numeric values fall back like the
/// legacy handler (limit to the default, minRisk and before to 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// Page size.
    pub limit: usize,
    /// Exact verdict.
    pub verdict: String,
    /// Lowercased host substring.
    pub host: String,
    /// Exact route class.
    pub route: String,
    /// Exact triggered rule.
    pub rule: String,
    /// Session or actor pseudonym.
    pub subject: String,
    /// Minimum risk score; 0 disables the filter.
    pub min_risk: i64,
    /// Only records with a smaller sequence; 0 starts at the newest.
    pub before: u64,
}

impl Default for Query {
    fn default() -> Self {
        Self {
            limit: DEFAULT_LIMIT,
            verdict: String::new(),
            host: String::new(),
            route: String::new(),
            rule: String::new(),
            subject: String::new(),
            min_risk: 0,
            before: 0,
        }
    }
}

impl Query {
    /// Build from decoded query pairs; the last occurrence of a name wins.
    pub fn from_pairs<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        let mut query = Self::default();
        for (name, value) in pairs {
            match name {
                "limit" => {
                    query.limit = match value.parse::<usize>() {
                        Ok(n) if (1..=MAX_LIMIT).contains(&n) => n,
                        _ => DEFAULT_LIMIT,
                    }
                }
                "verdict" => query.verdict = value.to_owned(),
                "host" => query.host = value.to_lowercase(),
                "route" => query.route = value.to_owned(),
                "rule" => query.rule = value.to_owned(),
                "subject" => query.subject = value.to_owned(),
                "minRisk" => query.min_risk = value.parse().unwrap_or(0),
                "before" => query.before = value.parse().unwrap_or(0),
                _ => {}
            }
        }
        query
    }

    fn matches(&self, record: &Record) -> bool {
        (self.before == 0 || record.seq < self.before)
            && (self.verdict.is_empty() || record.verdict == self.verdict)
            && (self.host.is_empty() || record.host.to_lowercase().contains(&self.host))
            && (self.route.is_empty() || record.route_class == self.route)
            && (self.min_risk <= 0 || record.risk_score >= self.min_risk)
            && (self.rule.is_empty()
                || record
                    .triggered_rules
                    .as_ref()
                    .is_some_and(|rules| rules.contains(&self.rule)))
            && (self.subject.is_empty()
                || record.session_pseudonym == self.subject
                || record.actor.id_pseudonym == self.subject)
    }
}

/// One returned record with its opaque console incident handle.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    /// Opaque, non-enumerable handle for the record's sequence.
    pub incident: String,
    /// The record.
    pub record: Record,
}

/// One page of results.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// Matching records, newest first.
    pub records: Vec<Hit>,
    /// Number of returned records.
    pub count: usize,
    /// Cursor for the next (older) page; 0 when nothing was returned.
    pub next_before: u64,
}

/// Search `records` (in append order) newest-first. `incident` maps a
/// sequence number to its opaque handle, e.g. `TokenKey::incident_handle`.
pub fn search(records: &[Record], query: &Query, incident: impl Fn(u64) -> String) -> Page {
    let mut out = Vec::with_capacity(query.limit.min(records.len()));
    let mut next_before = 0;
    for record in records.iter().rev() {
        if out.len() >= query.limit {
            break;
        }
        if !query.matches(record) {
            continue;
        }
        next_before = record.seq;
        out.push(Hit {
            incident: incident(record.seq),
            record: record.clone(),
        });
    }
    Page {
        count: out.len(),
        records: out,
        next_before,
    }
}

#[cfg(test)]
mod tests;

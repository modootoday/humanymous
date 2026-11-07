use super::LabError;
use humanymous_audit::{log::Log, record::Record};
use serde::{Deserialize, Serialize};

/// Minimized projection of an authenticated audit record, never raw requests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    /// Actual durable sequence.
    pub seq: u64,
    /// Actual runtime event category.
    pub event_type: String,
    /// Explicit policy verdict or COLLECTED.
    pub verdict: String,
    /// Actual rounded risk retained by runtime audit.
    pub risk_score: i64,
    /// Actual frozen rules, when present.
    pub rules: Vec<String>,
    /// Authenticated record commitment, without request identifiers or tokens.
    pub record_hash: String,
}
fn project(record: Record) -> AuditEvent {
    AuditEvent {
        seq: record.seq,
        event_type: record.event_type,
        verdict: record.verdict,
        risk_score: record.risk_score,
        rules: record.triggered_rules.unwrap_or_default(),
        record_hash: record.record_hash,
    }
}

/// Snapshot can only be constructed from an authenticated shared Log owner.
pub struct VerifiedSnapshot {
    origin_effects: u64,
    records: Vec<AuditEvent>,
}
/// Actual before/after effects; no synthetic verdict fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDelta {
    /// Independently observed origin dispatches during the case.
    pub origin_effects: u64,
    /// Actual appended audit records during the case.
    pub audit: Vec<AuditEvent>,
}
impl VerifiedSnapshot {
    /// Read only after the runtime child has released the WAL writer guard.
    /// The caller must own the origin counter and open Log with the provisioned
    /// identity; this is not independent-witness or rollback-proof storage.
    pub fn read(log: &Log, origin_effects: u64) -> Result<Self, LabError> {
        let records = log
            .records()
            .map_err(|_| LabError("authenticated audit export"))?;
        if records.len() as u64 != log.sequence()
            || records
                .iter()
                .enumerate()
                .any(|(index, record)| record.seq != index as u64 + 1)
        {
            return Err(LabError("audit sequence"));
        }
        Ok(Self {
            origin_effects,
            records: records.into_iter().map(project).collect(),
        })
    }
    /// Reject truncation, changed authenticated prefix or origin counter rollback.
    pub fn since(&self, before: &Self) -> Result<SnapshotDelta, LabError> {
        if !self.records.starts_with(&before.records) {
            return Err(LabError("audit prefix changed"));
        }
        Ok(SnapshotDelta {
            origin_effects: self
                .origin_effects
                .checked_sub(before.origin_effects)
                .ok_or(LabError("origin counter rollback"))?,
            audit: self.records[before.records.len()..].to_vec(),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_omits_raw_identifiers_and_prefix_rollback_is_fatal() {
        let record = Record {
            seq: 1,
            verdict: "CHALLENGE".into(),
            session_pseudonym: "private".into(),
            request_id: "capability".into(),
            record_hash: "a".repeat(64),
            ..Default::default()
        };
        let event = project(record);
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(!serialized.contains("private") && !serialized.contains("capability"));
        let before = VerifiedSnapshot {
            origin_effects: 1,
            records: vec![event.clone()],
        };
        let mut after = VerifiedSnapshot {
            origin_effects: 1,
            records: vec![event],
        };
        assert_eq!(after.since(&before).unwrap().origin_effects, 0);
        after.origin_effects = 0;
        assert!(after.since(&before).is_err());
        after.origin_effects = 1;
        after.records[0].verdict = "ALLOW".into();
        assert!(after.since(&before).is_err());
        after.records.clear();
        assert!(after.since(&before).is_err());
    }
}

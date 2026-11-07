//! Read only authenticated sealed records; raw request JSON cannot mint a head.
use super::{Configuration, OperationalSnapshot, Record, SealedAudit, commitment, invalid};
use crate::authority::decision::DurableAudit;
use std::io;
pub(super) struct Head {
    pub revision: u64,
    digest: String,
    previous: Option<String>,
}
impl Head {
    pub fn matches(&self, snapshot: &OperationalSnapshot) -> io::Result<bool> {
        Ok(snapshot.revision() == self.revision
            && commitment(&snapshot.bytes().map_err(|_| invalid())?) == self.digest)
    }
    pub fn parent_matches(&self, snapshot: &OperationalSnapshot) -> io::Result<bool> {
        Ok(snapshot.revision().checked_add(1) == Some(self.revision)
            && self.previous.as_deref()
                == Some(commitment(&snapshot.bytes().map_err(|_| invalid())?).as_str()))
    }
}
pub(super) fn read(
    records: &[Record],
    config: &Configuration,
    witnessed_through: u64,
) -> io::Result<Option<Head>> {
    let mut head: Option<Head> = None;
    for record in records
        .iter()
        .filter(|record| record.event_type == "runtime.settings.commit")
    {
        if record.seq > witnessed_through
            || record.event_version != "1"
            || record.node_id != config.audit.node_id
            || record.tenant_id != config.scope.tenant
            || record.config_version != config.scope.revision
        {
            return Err(invalid());
        }
        let revision = record
            .request_id
            .strip_prefix("settings:")
            .ok_or_else(invalid)?;
        let parsed: u64 = revision.parse().map_err(|_| invalid())?;
        if parsed.to_string() != revision
            || parsed > humanymous_core::integrity::request::browser::MAX_SAFE_INTEGER
            || record.correlation_id.len() != 64
            || !record
                .correlation_id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(invalid());
        }
        let expected = match &head {
            None => 0,
            Some(head) => head.revision.checked_add(1).ok_or_else(invalid)?,
        };
        if parsed != expected {
            return Err(invalid());
        }
        let previous = head.map(|head| head.digest);
        head = Some(Head {
            revision: parsed,
            digest: record.correlation_id.clone(),
            previous,
        });
    }
    Ok(head)
}
pub(super) fn append(
    audit: &mut SealedAudit,
    config: &Configuration,
    snapshot: &OperationalSnapshot,
) -> io::Result<()> {
    audit.append(Record {
        event_type: "runtime.settings.commit".into(),
        event_version: "1".into(),
        tenant_id: config.scope.tenant.clone(),
        request_id: format!("settings:{}", snapshot.revision()),
        correlation_id: commitment(&snapshot.bytes().map_err(|_| invalid())?),
        config_version: config.scope.revision.clone(),
        enforcement_action: "operational_settings_commit".into(),
        enforcement_mode: "enforce".into(),
        fail_mode: "fail_closed".into(),
        ..Default::default()
    })?;
    audit.flush()?;
    if audit
        .witnessed_through()?
        .is_none_or(|through| through < audit.sequence())
    {
        return Err(io::Error::other(
            "operational commit lacks external witness coverage",
        ));
    }
    Ok(())
}

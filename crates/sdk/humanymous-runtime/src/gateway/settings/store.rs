//! Native staging plus authenticated journal selection; no HTTP authorization.
use super::{OperationalSnapshot, PreparedSettings, commitment};
use crate::audit::{
    keys::custody::{Durability, Owner},
    record::Record,
    storage::TrustedSnapshot,
};
use crate::gateway::{configuration::Configuration, startup::SealedAudit};
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf};
mod journal;
use journal::Head;

const STORE_PROTOCOL: &str = "humanymous.operational-store/0.1";
/// Explicit provisioning; existing missing state never silently becomes new.
#[derive(Clone, Copy)]
pub enum SettingsBootstrap {
    /// Both native payload and operational journal must be absent.
    CreateNew,
    /// Restore only exact authenticated existing operational state.
    Existing,
}
/// Trusted private snapshot and minimum native durability configuration.
pub struct SettingsPolicy {
    /// Preprovisioned private directory, never selected by an HTTP request.
    pub directory: PathBuf,
    /// Explicit creation or authenticated restart.
    pub bootstrap: SettingsBootstrap,
    /// FileSyncedNamespaceUnconfirmed is an explicit weaker component policy.
    pub durability: Durability,
    /// Operator-authorized exact pending-envelope recovery; false by default.
    pub recover_native_pending: bool,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    protocol: String,
    committed: Option<OperationalSnapshot>,
    candidate: Option<OperationalSnapshot>,
}
impl Envelope {
    fn decode(bytes: &[u8], config: &Configuration) -> io::Result<Self> {
        let value: Self = serde_json::from_slice(bytes).map_err(|_| invalid())?;
        if value.protocol != STORE_PROTOCOL || value.bytes()?.as_slice() != bytes {
            return Err(invalid());
        }
        if let Some(snapshot) = &value.committed {
            snapshot.validate(config).map_err(|_| invalid())?;
        }
        if let Some(snapshot) = &value.candidate {
            snapshot.validate(config).map_err(|_| invalid())?;
            if let Some(parent) = &value.committed {
                let prepared = parent
                    .prepare(config, parent.revision(), snapshot.settings())
                    .map_err(|_| invalid())?;
                if prepared.bytes() != snapshot.bytes().map_err(|_| invalid())? {
                    return Err(invalid());
                }
            } else if snapshot.revision() != 0 {
                return Err(invalid());
            }
        }
        if value.committed.is_none() && value.candidate.is_none() {
            return Err(invalid());
        }
        Ok(value)
    }
    fn bytes(&self) -> io::Result<Vec<u8>> {
        let bytes = serde_json::to_vec(self).map_err(|_| invalid())?;
        if bytes.len() > 65_536 {
            return Err(invalid());
        }
        Ok(bytes)
    }
    fn select(&self, head: &Head) -> io::Result<OperationalSnapshot> {
        if let Some(candidate) = &self.candidate
            && head.matches(candidate)?
        {
            match &self.committed {
                Some(parent) if head.parent_matches(parent)? => {}
                None if head.revision == 0 => {}
                _ => return Err(invalid()),
            }
            return Ok(candidate.clone());
        }
        if let Some(committed) = &self.committed
            && head.matches(committed)?
        {
            return Ok(committed.clone());
        }
        Err(invalid())
    }
    fn known_native_parent(&self, head: &Head) -> io::Result<bool> {
        for snapshot in self.committed.iter().chain(self.candidate.iter()) {
            if head.matches(snapshot)? || head.parent_matches(snapshot)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
fn invalid() -> io::Error {
    io::Error::other("operational state or journal binding failed")
}
fn durable(actual: Durability, required: Durability) -> io::Result<()> {
    if required == Durability::FileAndDirectory && actual != Durability::FileAndDirectory {
        return Err(io::Error::other(
            "operational snapshot durability insufficient",
        ));
    }
    Ok(())
}
/// Lifetime-locked private state. An I/O ambiguity poisons further mutations.
pub struct SettingsStore {
    native: TrustedSnapshot,
    current: OperationalSnapshot,
    required: Durability,
    poisoned: bool,
}
impl SettingsStore {
    /// Open only after the matching sealed audit was authenticated by startup.
    /// Native privacy/atomicity and writer authentication are separate premises.
    pub fn open(
        policy: SettingsPolicy,
        owner: Owner,
        config: &Configuration,
        audit: &mut SealedAudit,
    ) -> io::Result<Self> {
        if !audit.matches_config(config) {
            return Err(invalid());
        }
        let through = audit
            .witnessed_through()?
            .ok_or_else(|| io::Error::other("operational settings require external witness"))?;
        crate::authority::transport::private_directory(&policy.directory, &config.owner)?;
        let mut native =
            TrustedSnapshot::open(&policy.directory, owner, "runtime-settings", 65_536)?;
        let head = journal::read(&audit.records()?, config, through)?;
        let inspected = native.inspect()?;
        if matches!(policy.bootstrap, SettingsBootstrap::CreateNew)
            && (head.is_some() || inspected.committed.is_some() || inspected.pending.is_some())
        {
            return Err(io::Error::other("operational settings already provisioned"));
        }
        if let Some(pending) = inspected.pending.as_ref() {
            if !policy.recover_native_pending {
                return Err(io::Error::other(
                    "explicit native settings recovery required",
                ));
            }
            let head = head.as_ref().ok_or_else(invalid)?;
            Envelope::decode(pending, config)?.select(head)?;
            if let Some(old) = inspected.committed.as_ref()
                && !Envelope::decode(old, config)?.known_native_parent(head)?
            {
                return Err(invalid());
            }
            durable(
                native.promote_pending(inspected.committed.as_deref(), pending)?,
                policy.durability,
            )?;
        }
        let stored = native.read()?;
        let current = match (stored, head) {
            (None, None) if matches!(policy.bootstrap, SettingsBootstrap::CreateNew) => {
                let initial = OperationalSnapshot::initial(config).map_err(|_| invalid())?;
                let staged = Envelope {
                    protocol: STORE_PROTOCOL.into(),
                    committed: None,
                    candidate: Some(initial.clone()),
                };
                durable(native.replace(&staged.bytes()?)?, policy.durability)?;
                journal::append(audit, config, &initial)?;
                initial
            }
            (Some(bytes), Some(head)) => Envelope::decode(&bytes, config)?.select(&head)?,
            _ => return Err(invalid()),
        };
        let finalized = Envelope {
            protocol: STORE_PROTOCOL.into(),
            committed: Some(current.clone()),
            candidate: None,
        };
        durable(native.replace(&finalized.bytes()?)?, policy.durability)?;
        Ok(Self {
            native,
            current,
            required: policy.durability,
            poisoned: false,
        })
    }
    /// Current authenticated selection. This does not authorize an HTTP write.
    pub fn snapshot(&self) -> &OperationalSnapshot {
        &self.current
    }
    /// Restore authenticated resource values before constructing a live epoch.
    /// This helper does not invalidate sessions in an already running application.
    pub fn restore_configuration(
        &self,
        config: &mut Configuration,
    ) -> Result<(), crate::gateway::Rejection> {
        CommittedSettings {
            snapshot: self.current.clone(),
        }
        .apply(config)
    }
    /// Stage, audit and finalize one exact candidate. The caller must serialize
    /// application, recheck real cancellation/deadlines and poison its serving
    /// epoch on an ambiguous error; no automatic retry is safe after staging.
    pub fn commit(
        &mut self,
        config: &Configuration,
        prepared: PreparedSettings,
        audit: &mut SealedAudit,
        check: impl Fn() -> io::Result<()>,
    ) -> io::Result<CommittedSettings> {
        if self.poisoned {
            return Err(io::Error::other("operational state poisoned"));
        }
        if !audit.matches_config(config) {
            return Err(invalid());
        }
        check()?;
        self.current.validate(config).map_err(|_| invalid())?;
        if prepared.parent_revision != self.current.revision()
            || prepared.parent_commitment
                != commitment(&self.current.bytes().map_err(|_| invalid())?)
        {
            return Err(invalid());
        }
        prepared.snapshot.validate(config).map_err(|_| invalid())?;
        self.poisoned = true;
        let staged = Envelope {
            protocol: STORE_PROTOCOL.into(),
            committed: Some(self.current.clone()),
            candidate: Some(prepared.snapshot.clone()),
        };
        durable(self.native.replace(&staged.bytes()?)?, self.required)?;
        check()?;
        journal::append(audit, config, &prepared.snapshot)?;
        check()?;
        let finalized = Envelope {
            protocol: STORE_PROTOCOL.into(),
            committed: Some(prepared.snapshot.clone()),
            candidate: None,
        };
        durable(self.native.replace(&finalized.bytes()?)?, self.required)?;
        check()?;
        self.current = prepared.snapshot;
        self.poisoned = false;
        Ok(CommittedSettings {
            snapshot: self.current.clone(),
        })
    }
}
/// Issued only after returned staging, audit and final native durability success.
/// It is not serializable or cloneable and cannot be supplied in an HTTP body.
pub struct CommittedSettings {
    snapshot: OperationalSnapshot,
}
impl CommittedSettings {
    /// Inspect the committed settings before one serialized application.
    pub fn snapshot(&self) -> &OperationalSnapshot {
        &self.snapshot
    }
    pub(in crate::gateway) fn apply(
        self,
        config: &mut Configuration,
    ) -> Result<(), crate::gateway::Rejection> {
        self.snapshot.validate(config)?;
        let settings = self.snapshot.settings;
        config.session_capacity = settings.session_capacity;
        config.session_ttl_ms = settings.session_ttl_ms;
        config.timeout_ms = settings.timeout_ms;
        config.rate_per_second = settings.rate_per_second;
        config.session_rate_per_second = settings.session_rate_per_second;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(config: &Configuration, snapshot: &OperationalSnapshot, seq: u64) -> Record {
        Record {
            seq,
            node_id: config.audit.node_id.clone(),
            event_type: "runtime.settings.commit".into(),
            event_version: "1".into(),
            tenant_id: config.scope.tenant.clone(),
            config_version: config.scope.revision.clone(),
            request_id: format!("settings:{}", snapshot.revision()),
            correlation_id: commitment(&snapshot.bytes().unwrap()),
            ..Default::default()
        }
    }
    #[test]
    fn crash_selection_applies_only_the_journal_committed_candidate() {
        let config = crate::gateway::tests::config();
        let initial = OperationalSnapshot::initial(&config).unwrap();
        let mut desired = initial.settings();
        desired.session_capacity = 32;
        let prepared = initial.prepare(&config, 0, desired).unwrap();
        let staged = Envelope {
            protocol: STORE_PROTOCOL.into(),
            committed: Some(initial.clone()),
            candidate: Some(prepared.snapshot.clone()),
        };
        let records = vec![record(&config, &initial, 1)];
        let head = journal::read(&records, &config, 1).unwrap().unwrap();
        assert_eq!(staged.select(&head).unwrap().revision(), 0);
        let mut records = records;
        records.push(record(&config, &prepared.snapshot, 2));
        let head = journal::read(&records, &config, 2).unwrap().unwrap();
        assert_eq!(
            staged.select(&head).unwrap().settings().session_capacity,
            32
        );
        let rolled_back = Envelope {
            protocol: STORE_PROTOCOL.into(),
            committed: Some(initial),
            candidate: None,
        };
        assert!(rolled_back.select(&head).is_err());
    }
    #[test]
    fn unwitnessed_tail_forged_scope_and_skipped_revision_are_refused() {
        let config = crate::gateway::tests::config();
        let initial = OperationalSnapshot::initial(&config).unwrap();
        let prepared = initial.prepare(&config, 0, initial.settings()).unwrap();
        let records = vec![
            record(&config, &initial, 1),
            record(&config, &prepared.snapshot, 2),
        ];
        assert!(journal::read(&records, &config, 1).is_err());
        let mut wrong = record(&config, &initial, 1);
        wrong.tenant_id = "other-tenant".into();
        assert!(journal::read(&[wrong], &config, 1).is_err());
        let mut wrong = record(&config, &initial, 1);
        wrong.request_id = "settings:01".into();
        assert!(journal::read(&[wrong], &config, 1).is_err());
        let wrong = record(&config, &prepared.snapshot, 1);
        assert!(journal::read(&[wrong], &config, 1).is_err());
    }
}

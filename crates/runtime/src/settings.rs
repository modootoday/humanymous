//! Pure scoped operational CAS candidates, never durable change authorization.
use crate::{
    Rejection,
    configuration::{Configuration, OperationalSettings},
    management::ManagementScope,
};
use humanymous_integrity::request::browser::MAX_SAFE_INTEGER;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const PROTOCOL: &str = "humanymous.operational/0.1";
const DOMAIN: &[u8] = b"humanymous/runtime/operational-state/0.1\0";
mod store;
pub use store::{CommittedSettings, SettingsBootstrap, SettingsPolicy, SettingsStore};

/// Strict nonsecret persisted payload. Loading it alone does not prove a commit.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalSnapshot {
    protocol: String,
    scope: ManagementScope,
    node: String,
    revision: u64,
    settings: OperationalSettings,
}
impl OperationalSnapshot {
    /// Explicit initial candidate from validated trusted configuration.
    pub fn initial(config: &Configuration) -> Result<Self, Rejection> {
        config.validate()?;
        Ok(Self {
            protocol: PROTOCOL.into(),
            scope: ManagementScope::from_scope(&config.scope),
            node: config.audit.node_id.clone(),
            revision: 0,
            settings: config.operational(),
        })
    }
    /// Decode a bounded candidate and bind it to the current provisioned scope.
    pub fn from_json(bytes: &[u8], config: &Configuration) -> Result<Self, Rejection> {
        if bytes.len() > 16_384 {
            return Err(Rejection::new(413, "settings_snapshot_budget"));
        }
        let snapshot: Self =
            serde_json::from_slice(bytes).map_err(|_| Rejection::new(400, "settings_snapshot"))?;
        snapshot.validate(config)?;
        if snapshot.bytes()?.as_slice() != bytes {
            return Err(Rejection::new(400, "settings_snapshot_noncanonical"));
        }
        Ok(snapshot)
    }
    fn validate(&self, config: &Configuration) -> Result<(), Rejection> {
        if self.protocol != PROTOCOL
            || self.scope != ManagementScope::from_scope(&config.scope)
            || self.node != config.audit.node_id
            || self.revision > MAX_SAFE_INTEGER
        {
            return Err(Rejection::new(409, "settings_snapshot_binding"));
        }
        self.settings.validate()
    }
    pub(crate) fn validate_binding(&self, config: &Configuration) -> Result<(), Rejection> {
        self.validate(config)
    }
    /// Immutable semantic revision, independent from the frozen scoring policy.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Resource settings only, with no credential or scoring fields.
    pub fn settings(&self) -> OperationalSettings {
        self.settings
    }
    /// Prepare exact candidate bytes after scope and parent-revision validation.
    /// The caller still owes durable staging, audit commit and actual application.
    pub fn prepare(
        &self,
        config: &Configuration,
        parent_revision: u64,
        settings: OperationalSettings,
    ) -> Result<PreparedSettings, Rejection> {
        self.validate(config)?;
        if parent_revision != self.revision {
            return Err(Rejection::new(409, "settings_parent_revision"));
        }
        settings.validate()?;
        let revision = self
            .revision
            .checked_add(1)
            .filter(|value| *value <= MAX_SAFE_INTEGER)
            .ok_or(Rejection::new(409, "settings_revision_exhausted"))?;
        let next = Self {
            protocol: PROTOCOL.into(),
            scope: self.scope.clone(),
            node: self.node.clone(),
            revision,
            settings,
        };
        let bytes = next.bytes()?;
        Ok(PreparedSettings {
            parent_revision,
            parent_commitment: commitment(&self.bytes()?),
            commitment: commitment(&bytes),
            bytes,
            snapshot: next,
        })
    }
    /// Fixed struct JSON representation, not an authenticated storage receipt.
    pub fn bytes(&self) -> Result<Vec<u8>, Rejection> {
        let bytes =
            serde_json::to_vec(self).map_err(|_| Rejection::new(503, "settings_serialization"))?;
        if bytes.len() > 16_384 {
            return Err(Rejection::new(413, "settings_snapshot_budget"));
        }
        Ok(bytes)
    }
}
fn commitment(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(DOMAIN);
    digest.update(bytes);
    hex::encode(digest.finalize())
}
/// Frozen candidate bytes and parent binding. This type grants no side effect.
pub struct PreparedSettings {
    parent_revision: u64,
    parent_commitment: String,
    commitment: String,
    bytes: Vec<u8>,
    snapshot: OperationalSnapshot,
}
impl PreparedSettings {
    /// Revision the actual single writer must still compare before committing.
    pub fn parent_revision(&self) -> u64 {
        self.parent_revision
    }
    /// Exact expected parent payload commitment.
    pub fn parent_commitment(&self) -> &str {
        &self.parent_commitment
    }
    /// Exact candidate payload commitment for the authenticated journal event.
    pub fn commitment(&self) -> &str {
        &self.commitment
    }
    /// Frozen staged payload, never reconstructed from an untrusted response.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Inspect the candidate without mutating its committed bytes.
    pub fn snapshot(&self) -> &OperationalSnapshot {
        &self.snapshot
    }
}

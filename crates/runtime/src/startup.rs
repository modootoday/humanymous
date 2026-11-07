//! Authenticate existing custody and retain its lifetime writer exclusion.
use crate::configuration::Configuration;
use humanymous_audit::{
    checkpoint::CheckpointWitness,
    keys::custody::Owner,
    log::{PinnedWitness, SealedLog, SealedLogOptions, UnlockCredential, WitnessDurability},
    storage::TailRecovery,
};
use humanymous_authority::configuration::Peer;

/// Trusted broker input, never deserialized from application HTTP or startup JSON.
/// Pinning a key does not establish process or organizational independence.
pub struct ExternalWitness {
    pinned: PinnedWitness,
}
impl ExternalWitness {
    /// Pin the broker's authenticated client before opening writer state.
    /// The broker owns peer authentication, I/O deadlines and child lifetime.
    pub fn new(
        client: Box<dyn CheckpointWitness>,
        expected_public: [u8; 32],
        durability: WitnessDurability,
    ) -> std::io::Result<Self> {
        PinnedWitness::new(client, expected_public, durability).map(|pinned| Self { pinned })
    }

    pub(crate) fn durability_name(&self) -> &'static str {
        match self.pinned.durability() {
            WitnessDurability::FileAndDirectory => "file_and_directory",
            WitnessDurability::FileSynced => "file_synced_namespace_unconfirmed",
        }
    }
}

/// Audit and its custody guard share one lifetime; neither is extractable.
pub struct SealedAudit {
    log: SealedLog,
}
impl humanymous_authority::decision::DurableAudit for SealedAudit {
    fn append(&mut self, record: humanymous_audit::record::Record) -> std::io::Result<()> {
        self.log.append(record).map(|_| ())
    }
}
impl SealedAudit {
    /// Flush before reporting a completed service shutdown; custody stays held.
    pub fn flush(&mut self) -> std::io::Result<()> {
        self.log.flush()
    }
    /// Explicit startup recovery facts, never permission to retry prior effects.
    pub fn recovery_coverage(&self) -> Option<&humanymous_audit::storage::WitnessCoverage> {
        self.log.recovery_coverage()
    }
}

/// Open audit with an existing sealed identity; startup never provisions keys.
pub fn open_audit(config: &mut Configuration) -> Result<SealedAudit, &'static str> {
    open_audit_inner(config, None, None)
}

/// Required-witness startup; errors never fall back to the writer-only path.
pub fn open_audit_with_witness(
    config: &mut Configuration,
    witness: ExternalWitness,
) -> Result<SealedAudit, &'static str> {
    open_audit_inner(config, Some(witness), None)
}

/// Explicit trusted witnessed recovery; never selected automatically on failure.
pub fn recover_audit_with_witness(
    config: &mut Configuration,
    witness: ExternalWitness,
    tail: TailRecovery,
) -> Result<SealedAudit, &'static str> {
    open_audit_inner(config, Some(witness), Some(tail))
}

fn open_audit_inner(
    config: &mut Configuration,
    witness: Option<ExternalWitness>,
    recovery: Option<TailRecovery>,
) -> Result<SealedAudit, &'static str> {
    if config.audit.require_external_witness && witness.is_none() {
        return Err("required external witness unavailable");
    }
    if recovery.is_some()
        && (witness.is_none() || config.audit.trusted_head.is_none() || config.audit.allow_new)
    {
        return Err("explicit recovery binding required");
    }
    humanymous_authority::transport::private_directory(&config.audit.directory, &config.owner)
        .map_err(|_| "audit directory is not private")?;
    humanymous_authority::transport::private_directory(&config.audit.keys.directory, &config.owner)
        .map_err(|_| "custody directory is not private")?;
    let owner = match &config.owner {
        #[cfg(unix)]
        Peer::Unix { .. } => Owner::CurrentUser,
        #[cfg(windows)]
        Peer::Windows { sid, .. } => Owner::WindowsSid(sid.clone()),
        #[allow(unreachable_patterns)]
        _ => return Err("custody platform mismatch"),
    };
    let options = SealedLogOptions {
        key_directory: config.audit.keys.directory.clone(),
        wal_directory: config.audit.directory.clone(),
        owner,
        node_id: config.audit.node_id.clone(),
        checkpoint_every: 1,
        allow_new_history: config.audit.allow_new,
        trusted_head: config.audit.trusted_head.clone(),
        require_external_witness: config.audit.require_external_witness,
    };
    let witness = witness.map(|witness| witness.pinned);
    let unlock = UnlockCredential::Utf8(&mut config.audit.keys.passphrase);
    let log = match (witness, recovery) {
        (Some(witness), Some(tail)) => SealedLog::recover_pinned(&options, unlock, witness, tail),
        (witness, None) => SealedLog::open(&options, unlock, witness),
        (None, Some(_)) => return Err("recovery requires external witness"),
    }
    .map_err(|_| "audit authenticated recovery")?;
    Ok(SealedAudit { log })
}

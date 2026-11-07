//! Authenticate existing custody and retain its lifetime writer exclusion.
use crate::configuration::Configuration;
use humanymous_audit::{
    checkpoint::CheckpointWitness,
    keys::{
        KeyError,
        custody::{Custody, Owner},
    },
    log::{Config, Log, WitnessDurability},
    storage::{Wal, WalOptions},
};
use humanymous_authority::configuration::Peer;

/// Trusted broker input, never deserialized from application HTTP or startup JSON.
/// Pinning a key does not establish process or organizational independence.
pub struct ExternalWitness {
    client: Box<dyn CheckpointWitness>,
    durability: WitnessDurability,
}
impl ExternalWitness {
    /// Pin the broker's authenticated client before opening writer state.
    /// The broker owns peer authentication, I/O deadlines and child lifetime.
    pub fn new(
        client: Box<dyn CheckpointWitness>,
        expected_public: [u8; 32],
        durability: WitnessDurability,
    ) -> std::io::Result<Self> {
        if client.public_key() != expected_public {
            return Err(std::io::Error::other("witness public identity mismatch"));
        }
        Ok(Self { client, durability })
    }

    pub(crate) fn durability_name(&self) -> &'static str {
        match self.durability {
            WitnessDurability::FileAndDirectory => "file_and_directory",
            WitnessDurability::FileSynced => "file_synced_namespace_unconfirmed",
        }
    }
}

/// Audit and its custody guard share one lifetime; neither is extractable.
pub struct SealedAudit {
    log: Log,
    _custody: Custody,
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
}

/// Open audit with an existing sealed identity; startup never provisions keys.
pub fn open_audit(config: &mut Configuration) -> Result<SealedAudit, &'static str> {
    open_audit_inner(config, None)
}

/// Required-witness startup; errors never fall back to the writer-only path.
pub fn open_audit_with_witness(
    config: &mut Configuration,
    witness: ExternalWitness,
) -> Result<SealedAudit, &'static str> {
    open_audit_inner(config, Some(witness))
}

fn open_audit_inner(
    config: &mut Configuration,
    witness: Option<ExternalWitness>,
) -> Result<SealedAudit, &'static str> {
    if config.audit.require_external_witness && witness.is_none() {
        return Err("required external witness unavailable");
    }
    let passphrase = std::mem::take(&mut config.audit.keys.passphrase);
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
    let mut custody =
        Custody::acquire(&config.audit.keys.directory, owner).map_err(|_| "custody unavailable")?;
    let (material, _) = custody
        .load_or_create(passphrase.as_bytes(), || Err(KeyError::InvalidMaterial))
        .map_err(|_| "existing sealed custody authentication")?;
    if !(32..=256).contains(&material.hmac_key().len()) {
        return Err("audit key width");
    }
    let wal = Wal::open(&config.audit.directory, WalOptions::default()).map_err(|_| "audit WAL")?;
    let log_config = Config {
        node_id: config.audit.node_id.clone(),
        hmac_key: material.hmac_key().to_vec(),
        signing_seed: *material.signing_seed(),
        checkpoint_every: 1,
        witness: None,
    };
    let log = match witness {
        Some(witness) => Log::open_with_external_witness(
            wal,
            log_config,
            config.audit.trusted_head.as_ref(),
            config.audit.allow_new,
            witness.client,
            witness.durability,
        ),
        None => Log::open(
            wal,
            log_config,
            config.audit.trusted_head.as_ref(),
            config.audit.allow_new,
        ),
    }
    .map_err(|_| "audit authenticated recovery")?;
    Ok(SealedAudit {
        log,
        _custody: custody,
    })
}

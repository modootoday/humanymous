//! Authenticate existing custody and retain its lifetime writer exclusion.
use crate::configuration::Configuration;
use humanymous_audit::{
    keys::{
        KeyError,
        custody::{Custody, Owner},
    },
    log::{Config, Log},
    storage::{Wal, WalOptions},
};
use humanymous_authority::configuration::Peer;

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
    let log = Log::open(
        Wal::open(&config.audit.directory, WalOptions::default()).map_err(|_| "audit WAL")?,
        Config {
            node_id: config.audit.node_id.clone(),
            hmac_key: material.hmac_key().to_vec(),
            signing_seed: *material.signing_seed(),
            checkpoint_every: 1,
            witness: None,
        },
        config.audit.trusted_head.as_ref(),
        config.audit.allow_new,
    )
    .map_err(|_| "audit authenticated recovery")?;
    Ok(SealedAudit {
        log,
        _custody: custody,
    })
}

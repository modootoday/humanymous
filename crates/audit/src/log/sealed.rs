//! One nonextractable lifetime owner for existing sealed custody and the writer.
use super::{Config, Log, WitnessDurability};
use crate::{
    checkpoint::{Checkpoint, CheckpointWitness},
    keys::{
        KeyError,
        custody::{Custody, Owner, permissions},
    },
    record::Record,
    storage::{
        AppliedWitnessRecovery, RecoveryAuthentication, RecoveryPolicy, TailRecovery, Wal,
        WalOptions, WitnessCoverage,
    },
};
use std::{fmt, io, path::PathBuf};
use zeroize::Zeroizing;

/// Borrowed private credential supplied by the trusted composition root. Bytes
/// reuse the existing native-security OS reader without a second reader/encoding.
/// No Debug, deserializer or public extraction method exposes its contents.
pub enum UnlockCredential<'a> {
    /// Existing private UTF-8 startup credential; consumed and zeroized at open.
    Utf8(&'a mut Zeroizing<String>),
    /// Existing native secret bytes; moved without encoding or a buffer copy.
    Bytes(&'a mut Zeroizing<Vec<u8>>),
}
impl UnlockCredential<'_> {
    fn take(self) -> Zeroizing<Vec<u8>> {
        match self {
            Self::Utf8(value) => {
                let original = std::mem::take(value);
                Zeroizing::new(original.as_bytes().to_vec())
            }
            Self::Bytes(value) => std::mem::take(value),
        }
    }
}

/// Trusted composition-root options. This is not an HTTP/startup wire DTO.
pub struct SealedLogOptions {
    /// Existing private sealed-key directory; keys are never provisioned here.
    pub key_directory: PathBuf,
    /// Existing private WAL parent directory.
    pub wal_directory: PathBuf,
    /// Provisioned OS owner, checked through the shared native privacy boundary.
    pub owner: Owner,
    /// Exact node owning records/checkpoints.
    pub node_id: String,
    /// Nonzero record cadence; service consumers normally require one.
    pub checkpoint_every: u64,
    /// Explicit permission to initialize history, never to generate keys.
    pub allow_new_history: bool,
    /// Independently retained checkpoint; mandatory for explicit recovery.
    pub trusted_head: Option<Checkpoint>,
    /// Missing injection is rejected before consuming unlock bytes or opening state.
    pub require_external_witness: bool,
}
/// Broker-pinned witness client and required native persistence strength.
/// Transport authentication, deadlines and child lifetime remain broker-owned.
pub struct PinnedWitness {
    client: Box<dyn CheckpointWitness>,
    durability: WitnessDurability,
}
impl PinnedWitness {
    /// Bind the verification identity before opening custody or writer state.
    pub fn new(
        client: Box<dyn CheckpointWitness>,
        expected_public: [u8; 32],
        durability: WitnessDurability,
    ) -> io::Result<Self> {
        if client.public_key() != expected_public {
            return Err(io::Error::other("witness public identity mismatch"));
        }
        Ok(Self { client, durability })
    }
    /// Explicit requirement, without exposing seed or transport internals.
    pub fn durability(&self) -> WitnessDurability {
        self.durability
    }
}
/// Bounded construction failure stage; raw paths, credentials and key material
/// are deliberately not retained in the error or its source chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealedLogStage {
    /// Required co-signer was absent before state/credential consumption.
    WitnessRequired,
    /// Invalid trusted options or explicit recovery without an independent pin.
    Options,
    /// Key directory privacy could not be established.
    KeyDirectory,
    /// WAL directory privacy could not be established.
    WalDirectory,
    /// Existing key ownership could not be acquired.
    Custody,
    /// Existing sealed key authentication failed; no replacement is generated.
    Unlock,
    /// Key width is unsuitable for the service boundary.
    KeyMaterial,
    /// WAL framing/ownership failed.
    Wal,
    /// Explicit witnessed recovery or coverage policy failed.
    Recovery,
    /// Authenticated writer/witness startup failed.
    Log,
}
/// Safe context for startup errors, preserving OS error category only.
#[derive(Debug)]
pub struct SealedLogError {
    stage: SealedLogStage,
    kind: io::ErrorKind,
}
impl SealedLogError {
    /// Stable failure context without sensitive source payloads.
    pub fn stage(&self) -> SealedLogStage {
        self.stage
    }
    /// Original bounded I/O category where available.
    pub fn kind(&self) -> io::ErrorKind {
        self.kind
    }
}
impl fmt::Display for SealedLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sealed audit unavailable at {:?}", self.stage)
    }
}
impl std::error::Error for SealedLogError {}
fn error(stage: SealedLogStage, kind: io::ErrorKind) -> SealedLogError {
    SealedLogError { stage, kind }
}
fn context(stage: SealedLogStage) -> impl FnOnce(io::Error) -> SealedLogError {
    move |e| error(stage, e.kind())
}
fn preflight(
    options: &SealedLogOptions,
    has_witness: bool,
    recovery: bool,
) -> Result<(), SealedLogError> {
    if options.require_external_witness && !has_witness {
        return Err(error(
            SealedLogStage::WitnessRequired,
            io::ErrorKind::NotConnected,
        ));
    }
    if options.node_id.is_empty()
        || options.node_id.len() > 512
        || options.node_id.trim() != options.node_id
        || options.node_id.chars().any(char::is_control)
        || options.checkpoint_every == 0
        || !options.key_directory.is_absolute()
        || !options.wal_directory.is_absolute()
        || (recovery && (!has_witness || options.trusted_head.is_none()))
    {
        return Err(error(SealedLogStage::Options, io::ErrorKind::InvalidInput));
    }
    Ok(())
}
/// Own custody and WAL/witness writer for exactly the same service lifetime.
/// Neither key material nor an extractable mutable Log/Custody is exposed.
pub struct SealedLog {
    log: Log,
    _custody: Custody,
    recovery: Option<AppliedWitnessRecovery>,
}
impl SealedLog {
    /// Open existing custody. Required witness refusal happens before any state
    /// access or taking the caller's zeroized credential. No key creation fallback.
    /// Existing unwitnessed complete tail requires explicit recover_pinned instead.
    pub fn open(
        options: &SealedLogOptions,
        unlock: UnlockCredential<'_>,
        witness: Option<PinnedWitness>,
    ) -> Result<Self, SealedLogError> {
        Self::open_inner(options, unlock, witness, None)
    }
    /// Explicit trusted operator recovery. Authenticate the independent latest
    /// head, optionally append its missing local checkpoint, and preserve complete
    /// ambiguous records only under the supplied explicit policy. No effect retry.
    pub fn recover_pinned(
        options: &SealedLogOptions,
        unlock: UnlockCredential<'_>,
        witness: PinnedWitness,
        tail: TailRecovery,
    ) -> Result<Self, SealedLogError> {
        Self::open_inner(options, unlock, Some(witness), Some(tail))
    }
    fn open_inner(
        options: &SealedLogOptions,
        unlock: UnlockCredential<'_>,
        witness: Option<PinnedWitness>,
        tail: Option<TailRecovery>,
    ) -> Result<Self, SealedLogError> {
        preflight(options, witness.is_some(), tail.is_some())?;
        permissions::directory(&options.key_directory, &options.owner)
            .map_err(context(SealedLogStage::KeyDirectory))?;
        permissions::directory(&options.wal_directory, &options.owner)
            .map_err(context(SealedLogStage::WalDirectory))?;
        let mut custody = Custody::acquire(&options.key_directory, options.owner.clone())
            .map_err(context(SealedLogStage::Custody))?;
        let passphrase = unlock.take();
        let (material, _) = custody
            .load_or_create(&passphrase, || Err(KeyError::InvalidMaterial))
            .map_err(context(SealedLogStage::Unlock))?;
        drop(passphrase);
        if !(32..=256).contains(&material.hmac_key().len()) {
            return Err(error(
                SealedLogStage::KeyMaterial,
                io::ErrorKind::InvalidData,
            ));
        }
        let config = Config {
            node_id: options.node_id.clone(),
            hmac_key: material.hmac_key().to_vec(),
            signing_seed: *material.signing_seed(),
            checkpoint_every: options.checkpoint_every,
            witness: None,
        };
        drop(material);
        let mut wal = Wal::open(&options.wal_directory, WalOptions::default())
            .map_err(context(SealedLogStage::Wal))?;
        let mut recovery = None;
        if let Some(tail) = tail {
            let external = options
                .trusted_head
                .as_ref()
                .ok_or_else(|| error(SealedLogStage::Options, io::ErrorKind::InvalidInput))?;
            let client = witness.as_ref().ok_or_else(|| {
                error(SealedLogStage::WitnessRequired, io::ErrorKind::NotConnected)
            })?;
            client
                .client
                .validate_local_head(external)
                .map_err(context(SealedLogStage::Recovery))?;
            let public = client.client.public_key();
            let writer = ed25519_dalek::SigningKey::from_bytes(&config.signing_seed)
                .verifying_key()
                .to_bytes();
            let applied = wal
                .recover_witness_checkpoint(
                    RecoveryPolicy {
                        node_id: &config.node_id,
                        hmac_key: &config.hmac_key,
                        writer_public: &writer,
                        witness_public: Some(&public),
                        trusted_head: Some(external),
                        allow_new: false,
                    },
                    tail,
                )
                .map_err(context(SealedLogStage::Recovery))?;
            recovery = Some(applied);
        }
        let external_injected = witness.is_some();
        let log = match witness {
            Some(w) => Log::open_with_external_witness(
                wal,
                config,
                options.trusted_head.as_ref(),
                options.allow_new_history,
                w.client,
                w.durability,
            ),
            None => Log::open(
                wal,
                config,
                options.trusted_head.as_ref(),
                options.allow_new_history,
            ),
        }
        .map_err(context(SealedLogStage::Log))?;
        if external_injected && recovery.is_none() && log.sequence > 0 {
            let external = log
                .checkpoints
                .last()
                .cloned()
                .ok_or_else(|| error(SealedLogStage::Recovery, io::ErrorKind::InvalidData))?;
            let ambiguous_tail_len = log
                .sequence
                .checked_sub(external.tree_size)
                .ok_or_else(|| error(SealedLogStage::Recovery, io::ErrorKind::InvalidData))?;
            if ambiguous_tail_len > 0 {
                return Err(error(SealedLogStage::Recovery, io::ErrorKind::InvalidData));
            }
            // Log::open already verified all HMAC/writer/witness commitments and
            // matched this exact local head to the independently pinned client.
            recovery = Some(AppliedWitnessRecovery {
                coverage: WitnessCoverage {
                    durable_sequence: log.sequence,
                    witnessed_through: external.tree_size,
                    authenticated_external_head: external.clone(),
                    local_latest_checkpoint: Some(external),
                    ambiguous_tail_len,
                    fully_witnessed: true,
                    authentication: RecoveryAuthentication {
                        hmac_checked: true,
                        writer_checked: true,
                        witness_checked: true,
                    },
                },
                checkpoint_appended: false,
                persistence: None,
                recovered_torn_bytes: log.wal.recovered_bytes(),
            });
        }
        Ok(Self {
            log,
            _custody: custody,
            recovery,
        })
    }
    /// Persist before returning a sealed record; custody remains excluded.
    pub fn append(&mut self, record: Record) -> io::Result<Record> {
        self.log.append(record)
    }
    /// Final checkpoint and WAL flush; failure is never shutdown success.
    pub fn flush(&mut self) -> io::Result<()> {
        self.log.flush()
    }
    /// Complete authenticated records; this is not bounded streaming recovery.
    pub fn records(&self) -> io::Result<Vec<Record>> {
        self.log.records()
    }
    /// Persisted writer/witness commitments.
    pub fn checkpoints(&self) -> io::Result<Vec<Checkpoint>> {
        self.log.checkpoints()
    }
    /// Current authenticated sequence, not an origin-side-effect count.
    pub fn sequence(&self) -> u64 {
        self.log.sequence()
    }
    /// Startup coverage, absent for new empty or writer-only history.
    pub fn recovery_coverage(&self) -> Option<&WitnessCoverage> {
        self.recovery.as_ref().map(|result| &result.coverage)
    }
    /// Distinguish an actually persisted checkpoint repair from verification only,
    /// including native persistence strength and earlier torn-frame byte count.
    pub fn recovery_receipt(&self) -> Option<&AppliedWitnessRecovery> {
        self.recovery.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options() -> SealedLogOptions {
        #[cfg(windows)]
        let base = PathBuf::from("C:/synthetic");
        #[cfg(unix)]
        let base = PathBuf::from("/synthetic");
        SealedLogOptions {
            key_directory: base.join("keys"),
            wal_directory: base.join("wal"),
            owner: {
                #[cfg(windows)]
                {
                    Owner::WindowsSid("S-1-5-21-1".into())
                }
                #[cfg(unix)]
                {
                    Owner::CurrentUser
                }
            },
            node_id: "node".into(),
            checkpoint_every: 1,
            allow_new_history: false,
            trusted_head: None,
            require_external_witness: true,
        }
    }
    #[test]
    fn missing_witness_precedes_other_options_without_consuming_state() {
        let mut options = options();
        options.node_id.clear();
        assert_eq!(
            preflight(&options, false, false).unwrap_err().stage(),
            SealedLogStage::WitnessRequired
        );
    }
    #[test]
    fn invalid_options_and_unpinned_recovery_fail_before_io() {
        let mut options = options();
        assert!(preflight(&options, true, false).is_ok());
        assert!(preflight(&options, true, true).is_err());
        options.checkpoint_every = 0;
        assert!(preflight(&options, true, false).is_err());
        options.checkpoint_every = 1;
        options.node_id = " node".into();
        assert!(preflight(&options, true, false).is_err());
        options.node_id = "node".into();
        options.key_directory = PathBuf::from("relative");
        assert!(preflight(&options, true, false).is_err());
    }
}

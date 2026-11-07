//! Bounded private inherited-pipe witness protocol. A trusted supervisor owns
//! both pipe ends and provisions this service; it is not a network endpoint.
use super::{DurableWitness, WitnessBootstrap};
use crate::{
    checkpoint::Checkpoint,
    keys::{
        KeyError,
        custody::{Custody, Durability, Owner},
    },
    merkle::Hash,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, BufRead, Read, Write},
    path::PathBuf,
};
use zeroize::Zeroizing;
mod client;
pub use client::WitnessServiceClient;

/// Largest newline-terminated UTF-8 JSON frame, including its final newline.
pub const MAX_FRAME_BYTES: usize = 128 * 1024;

/// Explicit provisioning choice. Existing service state requires a pin held
/// outside the witness state directory; missing state never selects new mode.
#[derive(Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum WitnessServiceBootstrap {
    /// Trusted supervisor explicitly provisions an initially empty witness.
    CreateNew,
    /// Restart against an independently authenticated minimum commitment.
    Pinned {
        /// Checkpoint held outside the witness state directory.
        minimum: Checkpoint,
        /// Consistency proof from minimum to the current signed stored head.
        consistency_proof: Vec<Hash>,
    },
}

/// One private first-frame configuration. No Debug implementation exposes the
/// unlock credential. Never deliver it in process arguments, environment or logs.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WitnessServiceConfig {
    /// Existing separately provisioned sealed witness identity directory.
    pub key_directory: PathBuf,
    /// Existing private latest-head directory, distinct from key_directory.
    pub state_directory: PathBuf,
    /// Exact Windows owner SID; must be absent on Unix.
    pub owner_sid: Option<String>,
    /// Private sealed-key passphrase, zeroized after this configuration drops.
    pub passphrase: Zeroizing<Vec<u8>>,
    /// Authorized writer public key, distinct from the witness identity.
    pub writer_public: [u8; 32],
    /// Exact provisioned node identity.
    pub node_id: String,
    /// Explicit first provisioning or independently pinned restart.
    pub bootstrap: WitnessServiceBootstrap,
}

/// One bounded supervisor command. Requests contain no witness seed or password.
#[derive(Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum WitnessServiceRequest {
    /// Read the latest independently persisted head.
    State,
    /// Authenticate and persist an append-only checkpoint before replying.
    CounterSign {
        /// Writer-authenticated commitment.
        checkpoint: Checkpoint,
        /// Append-only proof from the service's current commitment.
        consistency_proof: Vec<Hash>,
    },
    /// Close service-owned key/state locks and finish cleanly.
    Shutdown,
}

/// Persistence strength in the typed wire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WitnessPersistence {
    /// File and directory flush completed on a local Unix filesystem.
    FileAndDirectory,
    /// Windows file flushed; namespace power-loss durability unconfirmed.
    FileSyncedNamespaceUnconfirmed,
}
impl From<Durability> for WitnessPersistence {
    fn from(value: Durability) -> Self {
        match value {
            Durability::FileAndDirectory => Self::FileAndDirectory,
            Durability::FileSyncedNamespaceUnconfirmed => Self::FileSyncedNamespaceUnconfirmed,
        }
    }
}

/// Non-sensitive refusal classes; no input, path, passphrase or seed is echoed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WitnessServiceRefusal {
    /// Unknown, oversized, malformed or unterminated command.
    InvalidCommand,
    /// Checkpoint/signature/consistency policy refused the supplied commitment.
    RejectedCheckpoint,
    /// Required durable state is unavailable; service exits after this response.
    Unavailable,
}

/// Public protocol replies. Key material and unlock credentials never appear.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum WitnessServiceResponse {
    /// Key custody and independent latest-head state are open and authenticated.
    Ready {
        /// Verification-only witness identity.
        witness_public: [u8; 32],
        /// Current persisted commitment; absent before explicit first signing.
        checkpoint: Option<Checkpoint>,
        /// Native capability, not a claim that an absent checkpoint was flushed.
        persistence_capability: WitnessPersistence,
    },
    /// Latest persisted commitment.
    State {
        /// Current persisted commitment, never an in-memory-only acceptance.
        checkpoint: Option<Checkpoint>,
    },
    /// Counter-signature is returned only after the reported persistence step.
    Signed {
        /// Complete writer and witness authenticated checkpoint.
        checkpoint: Checkpoint,
        /// Actual native outcome for this replacement.
        persistence: WitnessPersistence,
    },
    /// Failure without sensitive details.
    Refused {
        /// Bounded refusal class.
        reason: WitnessServiceRefusal,
    },
    /// Service is shutting down and releasing its guards.
    Stopped,
}

fn refused() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "witness service unavailable")
}

fn owner(config: &WitnessServiceConfig) -> io::Result<Owner> {
    #[cfg(unix)]
    {
        if config.owner_sid.is_some() {
            return Err(refused());
        }
        Ok(Owner::CurrentUser)
    }
    #[cfg(windows)]
    {
        config
            .owner_sid
            .as_ref()
            .filter(|sid| !sid.is_empty())
            .map(|sid| Owner::WindowsSid(sid.clone()))
            .ok_or_else(refused)
    }
}

/// Serve one private inherited-pipe connection. The supervisor must provide
/// authenticated local pipe provenance and launch/key ACLs; no remote identity
/// is inferred from a header or from possession of public checkpoints.
pub fn serve(mut input: impl BufRead, mut output: impl Write) -> io::Result<()> {
    let config_bytes = read_frame(&mut input)?.ok_or_else(refused)?;
    let config_bytes = Zeroizing::new(config_bytes);
    let config: WitnessServiceConfig =
        serde_json::from_slice(&config_bytes).map_err(|_| refused())?;
    if std::fs::canonicalize(&config.key_directory)?
        == std::fs::canonicalize(&config.state_directory)?
    {
        return Err(refused());
    }
    let mut custody = Custody::acquire(&config.key_directory, owner(&config)?)?;
    // A witness service never generates a replacement identity or silently
    // imports a writer's co-located legacy witness/vault material.
    let (material, _) =
        custody.load_or_create(&config.passphrase, || Err(KeyError::InvalidMaterial))?;
    if material.witness_seed().is_some() || material.vault().is_some() {
        return Err(refused());
    }
    let bootstrap = match &config.bootstrap {
        WitnessServiceBootstrap::CreateNew => WitnessBootstrap::CreateNew,
        WitnessServiceBootstrap::Pinned {
            minimum,
            consistency_proof,
        } => WitnessBootstrap::Pinned {
            minimum,
            consistency_proof,
        },
    };
    let mut witness = DurableWitness::open(
        &config.state_directory,
        owner(&config)?,
        material.signing_seed(),
        config.writer_public,
        config.node_id.clone(),
        bootstrap,
    )?;
    #[cfg(unix)]
    let capability = WitnessPersistence::FileAndDirectory;
    #[cfg(windows)]
    let capability = WitnessPersistence::FileSyncedNamespaceUnconfirmed;
    write_frame(
        &mut output,
        &WitnessServiceResponse::Ready {
            witness_public: witness.public_key(),
            checkpoint: witness.last_checkpoint()?.cloned(),
            persistence_capability: capability,
        },
    )?;
    // Drop the unlock credential and transient plaintext configuration before
    // serving ordinary commands. Custody remains lifetime-locked until exit.
    drop(material);
    drop(config);
    drop(config_bytes);
    loop {
        let bytes = match read_frame(&mut input) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(()),
            Err(_) => {
                write_frame(
                    &mut output,
                    &WitnessServiceResponse::Refused {
                        reason: WitnessServiceRefusal::InvalidCommand,
                    },
                )?;
                return Err(refused());
            }
        };
        let request = match serde_json::from_slice::<WitnessServiceRequest>(&bytes) {
            Ok(request) => request,
            Err(_) => {
                write_frame(
                    &mut output,
                    &WitnessServiceResponse::Refused {
                        reason: WitnessServiceRefusal::InvalidCommand,
                    },
                )?;
                return Err(refused());
            }
        };
        match request {
            WitnessServiceRequest::State => write_frame(
                &mut output,
                &WitnessServiceResponse::State {
                    checkpoint: witness.last_checkpoint()?.cloned(),
                },
            )?,
            WitnessServiceRequest::Shutdown => {
                write_frame(&mut output, &WitnessServiceResponse::Stopped)?;
                return Ok(());
            }
            WitnessServiceRequest::CounterSign {
                checkpoint,
                consistency_proof,
            } => match witness.counter_sign(&checkpoint, &consistency_proof) {
                Ok(receipt) => write_frame(
                    &mut output,
                    &WitnessServiceResponse::Signed {
                        checkpoint: receipt.checkpoint,
                        persistence: receipt.durability.into(),
                    },
                )?,
                Err(_) => {
                    let unavailable = witness.last_checkpoint().is_err();
                    write_frame(
                        &mut output,
                        &WitnessServiceResponse::Refused {
                            reason: if unavailable {
                                WitnessServiceRefusal::Unavailable
                            } else {
                                WitnessServiceRefusal::RejectedCheckpoint
                            },
                        },
                    )?;
                    if unavailable {
                        return Err(refused());
                    }
                }
            },
        }
    }
}

/// Read exactly one bounded newline JSON frame. Clean EOF is distinct from an
/// unterminated frame; size checks happen while reading rather than afterwards.
pub fn read_frame(input: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    (&mut *input)
        .take(MAX_FRAME_BYTES as u64 + 1)
        .read_until(b'\n', &mut bytes)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes.len() > MAX_FRAME_BYTES || bytes.last() != Some(&b'\n') {
        return Err(refused());
    }
    Ok(Some(bytes))
}

/// Write and flush one bounded typed response/request/configuration frame. The
/// caller must use a private pipe for secret configuration. Transient serialized
/// bytes are zeroized, including when this function serializes configuration.
pub fn write_frame(output: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    let mut bytes = Zeroizing::new(serde_json::to_vec(value).map_err(|_| refused())?);
    bytes.push(b'\n');
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(refused());
    }
    output.write_all(&bytes)?;
    output.flush()
}

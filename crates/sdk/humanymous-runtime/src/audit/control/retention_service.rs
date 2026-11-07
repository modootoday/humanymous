//! Verification-only retention over supervisor-owned authenticated private pipes.
//! The supervisor owns deadlines, peer/process identity, isolation and reaping.
use super::{
    ControlAnchorRetainer, LedgerAnchor, NativeAnchorRetainer, RetainedControlAnchor, Scope,
};
use crate::audit::{
    checkpoint::service::{WitnessPersistence, read_frame, write_frame},
    keys::custody::{Durability, Owner},
};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, BufRead, Write},
    path::PathBuf,
};

/// First private configuration frame. Contains public identity, no signing seed.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionServiceConfig {
    /// Independently provisioned private state directory.
    pub directory: PathBuf,
    /// Exact Windows owner SID; absent on Unix.
    pub owner_sid: Option<String>,
    /// Scope uniquely provisioned with this authority public key.
    pub scope: Scope,
    /// Original control authority verification identity.
    pub authority_public: [u8; 32],
    /// Exact independently authenticated latest anchor, including on restart.
    pub pin: LedgerAnchor,
    /// Explicit initial revision-zero provisioning; never inferred from absence.
    pub provision: bool,
}
/// Bounded commands; no command resets state or executes a protected action.
#[derive(Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum RetentionServiceRequest {
    /// Read authenticated persisted state.
    State,
    /// Persist the exact signed successor before acknowledging.
    Retain {
        /// Exact independently retained current signed anchor.
        previous: LedgerAnchor,
        /// Original authority-signed revision+1 anchor.
        next: LedgerAnchor,
    },
    /// Release service-owned locks.
    Shutdown,
}
/// Public acknowledgements, never approval or effect-completion claims.
#[derive(Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum RetentionServiceResponse {
    /// Opened exact pinned independent state.
    Ready {
        /// Exact provisioned installation and node.
        scope: Scope,
        /// Verification-only original authority identity.
        authority_public: [u8; 32],
        /// Authenticated persisted anchor, equal to the supplied external pin.
        anchor: LedgerAnchor,
    },
    /// Current persisted signed anchor.
    State {
        /// Exact current persisted authority-signed anchor.
        anchor: LedgerAnchor,
    },
    /// Exact signed anchor and actual native persistence result.
    Retained {
        /// Exact successor persisted before this acknowledgement.
        anchor: LedgerAnchor,
        /// Actual native replacement strength, including namespace uncertainty.
        persistence: WitnessPersistence,
    },
    /// Bounded failure; connection closes and no action grant follows.
    Refused,
    /// Locks are being released; no future commands accepted.
    Stopped,
}
fn invalid() -> io::Error {
    io::Error::other("control retention service unavailable")
}
fn receive<T: serde::de::DeserializeOwned>(input: &mut impl BufRead) -> io::Result<T> {
    let bytes = read_frame(input)?.ok_or_else(invalid)?;
    serde_json::from_slice(&bytes).map_err(|_| invalid())
}
fn owner(config: &RetentionServiceConfig) -> io::Result<Owner> {
    #[cfg(unix)]
    {
        if config.owner_sid.is_some() {
            return Err(invalid());
        }
        Ok(Owner::CurrentUser)
    }
    #[cfg(windows)]
    {
        Ok(Owner::WindowsSid(
            config.owner_sid.clone().ok_or_else(invalid)?,
        ))
    }
}
/// Run one inherited-pipe service session. Any refusal terminates the session;
/// malformed, failed or unknown outcomes require authenticated pinned restart.
pub fn serve(mut input: impl BufRead, mut output: impl Write) -> io::Result<()> {
    let config: RetentionServiceConfig = receive(&mut input)?;
    let mut retainer = NativeAnchorRetainer::open(
        &config.directory,
        owner(&config)?,
        config.scope.clone(),
        config.authority_public,
        &config.pin,
        config.provision,
    )?;
    write_frame(
        &mut output,
        &RetentionServiceResponse::Ready {
            scope: config.scope,
            authority_public: config.authority_public,
            anchor: retainer.current()?,
        },
    )?;
    loop {
        let request: RetentionServiceRequest = match receive(&mut input) {
            Ok(request) => request,
            Err(error) => {
                write_frame(&mut output, &RetentionServiceResponse::Refused)?;
                return Err(error);
            }
        };
        let result = match request {
            RetentionServiceRequest::State => retainer
                .current()
                .map(|anchor| RetentionServiceResponse::State { anchor }),
            RetentionServiceRequest::Retain { previous, next } => retainer
                .retain(&previous, &next)
                .map(|receipt| RetentionServiceResponse::Retained {
                    anchor: receipt.anchor().clone(),
                    persistence: receipt.durability().into(),
                }),
            RetentionServiceRequest::Shutdown => {
                write_frame(&mut output, &RetentionServiceResponse::Stopped)?;
                return Ok(());
            }
        };
        match result {
            Ok(response) => write_frame(&mut output, &response)?,
            Err(error) => {
                write_frame(&mut output, &RetentionServiceResponse::Refused)?;
                return Err(error);
            }
        }
    }
}

/// Verified protocol client. The broker authenticates pipe provenance; public
/// anchor signatures alone cannot authenticate the live retention process.
pub struct RetentionServiceClient<R: BufRead, W: Write> {
    input: R,
    output: W,
    scope: Scope,
    public: [u8; 32],
    anchor: LedgerAnchor,
    poisoned: bool,
}
impl<R: BufRead, W: Write> RetentionServiceClient<R, W> {
    /// Send public configuration and validate exact readiness. Restarts always
    /// use a separately retained latest pin, not a caller-selected stale anchor.
    pub fn connect(input: R, mut output: W, config: &RetentionServiceConfig) -> io::Result<Self> {
        write_frame(&mut output, config)?;
        Self::from_ready(
            input,
            output,
            config.scope.clone(),
            config.authority_public,
            &config.pin,
        )
    }
    /// Adopt brokered pipes after the trusted supervisor configured the service.
    pub fn from_ready(
        mut input: R,
        output: W,
        scope: Scope,
        public: [u8; 32],
        pin: &LedgerAnchor,
    ) -> io::Result<Self> {
        super::policy::valid_scope(&scope).map_err(|_| invalid())?;
        let RetentionServiceResponse::Ready {
            scope: actual_scope,
            authority_public,
            anchor,
        } = receive(&mut input)?
        else {
            return Err(invalid());
        };
        if actual_scope != scope
            || authority_public != public
            || anchor != *pin
            || !anchor.verify(&public)
        {
            return Err(invalid());
        }
        Ok(Self {
            input,
            output,
            scope,
            public,
            anchor,
            poisoned: false,
        })
    }
    fn exchange(
        &mut self,
        request: &RetentionServiceRequest,
    ) -> io::Result<RetentionServiceResponse> {
        if self.poisoned {
            return Err(invalid());
        }
        // Successful I/O alone is insufficient: the caller must authenticate
        // the response and explicitly clear poison after exact verification.
        self.poisoned = true;
        write_frame(&mut self.output, request)?;
        receive(&mut self.input)
    }
    /// Close once; an unknown outcome never permits further use of this client.
    pub fn shutdown(&mut self) -> io::Result<()> {
        match self.exchange(&RetentionServiceRequest::Shutdown)? {
            RetentionServiceResponse::Stopped => Ok(()),
            _ => Err(invalid()),
        }
    }
}
impl<R: BufRead, W: Write> ControlAnchorRetainer for RetentionServiceClient<R, W> {
    fn scope(&self) -> &Scope {
        &self.scope
    }
    fn authority_public(&self) -> [u8; 32] {
        self.public
    }
    fn current(&mut self) -> io::Result<LedgerAnchor> {
        let RetentionServiceResponse::State { anchor } =
            self.exchange(&RetentionServiceRequest::State)?
        else {
            return Err(invalid());
        };
        if anchor != self.anchor || !anchor.verify(&self.public) {
            return Err(invalid());
        }
        self.poisoned = false;
        Ok(anchor)
    }
    fn retain(
        &mut self,
        previous: &LedgerAnchor,
        next: &LedgerAnchor,
    ) -> io::Result<RetainedControlAnchor> {
        if previous != &self.anchor {
            return Err(invalid());
        }
        super::retention::transition(&self.public, previous, next)?;
        let RetentionServiceResponse::Retained {
            anchor,
            persistence,
        } = self.exchange(&RetentionServiceRequest::Retain {
            previous: previous.clone(),
            next: next.clone(),
        })?
        else {
            return Err(invalid());
        };
        let durability = match persistence {
            WitnessPersistence::FileAndDirectory => Durability::FileAndDirectory,
            WitnessPersistence::FileSyncedNamespaceUnconfirmed => {
                Durability::FileSyncedNamespaceUnconfirmed
            }
        };
        let receipt = RetainedControlAnchor::from_acknowledgement(
            self.scope.clone(),
            self.public,
            next,
            anchor,
            durability,
        )?;
        self.anchor = next.clone();
        self.poisoned = false;
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::io::Cursor;

    fn fixture() -> (Scope, [u8; 32], LedgerAnchor, LedgerAnchor) {
        let key = SigningKey::from_bytes(&[71; 32]);
        let make = |revision| {
            let digest = hex::encode([revision as u8; 32]);
            let message = format!("humanymous/control-ledger/v1\0{revision}\0{digest}");
            LedgerAnchor {
                revision,
                digest,
                signature: hex::encode(key.sign(message.as_bytes()).to_bytes()),
            }
        };
        (
            Scope {
                installation: "pure-service-test".into(),
                node: "node".into(),
            },
            key.verifying_key().to_bytes(),
            make(0),
            make(1),
        )
    }
    fn frames(responses: &[RetentionServiceResponse]) -> Cursor<Vec<u8>> {
        let mut bytes = Vec::new();
        for response in responses {
            write_frame(&mut bytes, response).unwrap();
        }
        Cursor::new(bytes)
    }
    #[test]
    fn client_accepts_only_exact_ready_state_and_persisted_successor() {
        let (scope, public, previous, next) = fixture();
        let input = frames(&[
            RetentionServiceResponse::Ready {
                scope: scope.clone(),
                authority_public: public,
                anchor: previous.clone(),
            },
            RetentionServiceResponse::State {
                anchor: previous.clone(),
            },
            RetentionServiceResponse::Retained {
                anchor: next.clone(),
                persistence: WitnessPersistence::FileSyncedNamespaceUnconfirmed,
            },
            RetentionServiceResponse::State {
                anchor: next.clone(),
            },
        ]);
        let mut client =
            RetentionServiceClient::from_ready(input, Vec::new(), scope, public, &previous)
                .unwrap();
        assert_eq!(client.current().unwrap(), previous);
        let receipt = client.retain(&previous, &next).unwrap();
        assert_eq!(receipt.anchor(), &next);
        assert_eq!(
            receipt.durability(),
            Durability::FileSyncedNamespaceUnconfirmed
        );
        assert_eq!(client.current().unwrap(), next);
    }
    #[test]
    fn stale_acknowledgement_permanently_poisons_connection() {
        let (scope, public, previous, next) = fixture();
        let input = frames(&[
            RetentionServiceResponse::Ready {
                scope: scope.clone(),
                authority_public: public,
                anchor: previous.clone(),
            },
            RetentionServiceResponse::Retained {
                anchor: previous.clone(),
                persistence: WitnessPersistence::FileAndDirectory,
            },
            RetentionServiceResponse::State {
                anchor: previous.clone(),
            },
        ]);
        let mut client =
            RetentionServiceClient::from_ready(input, Vec::new(), scope, public, &previous)
                .unwrap();
        assert!(client.retain(&previous, &next).is_err());
        let sent = client.output.len();
        assert!(client.current().is_err());
        assert!(client.retain(&previous, &next).is_err());
        assert_eq!(client.output.len(), sent);
    }
    #[test]
    fn readiness_rejects_different_scope_and_unexpected_protocol_status() {
        let (scope, public, previous, _) = fixture();
        let mut other = scope.clone();
        other.node = "other".into();
        let ready = RetentionServiceResponse::Ready {
            scope: other,
            authority_public: public,
            anchor: previous.clone(),
        };
        assert!(
            RetentionServiceClient::from_ready(
                frames(&[ready]),
                Vec::new(),
                scope.clone(),
                public,
                &previous
            )
            .is_err()
        );
        assert!(
            RetentionServiceClient::from_ready(
                frames(&[RetentionServiceResponse::State {
                    anchor: previous.clone()
                }]),
                Vec::new(),
                scope,
                public,
                &previous
            )
            .is_err()
        );
    }
}

//! Verification-only client for a supervisor-owned witness pipe connection.
use super::{
    WitnessPersistence, WitnessServiceBootstrap, WitnessServiceConfig, WitnessServiceRequest,
    WitnessServiceResponse, read_frame, write_frame,
};
use crate::{
    checkpoint::{Checkpoint, CheckpointWitness, WitnessBootstrap, WitnessReceipt},
    keys::custody::Durability,
    merkle::{Hash, verify_consistency},
};
use std::io::{self, BufRead, Write};

/// Shared typed client with a verified latest-head cache. A trusted broker owns
/// process/peer authentication, pipe provenance, deadlines and child reaping.
/// Possession of public checkpoints does not authenticate a remote transport.
pub struct WitnessServiceClient<R: BufRead + Send, W: Write + Send> {
    input: R,
    output: W,
    witness: [u8; 32],
    writer: [u8; 32],
    node: String,
    head: Option<Checkpoint>,
    poisoned: bool,
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "witness client unavailable or response invalid",
    )
}

impl<R: BufRead + Send, W: Write + Send> WitnessServiceClient<R, W> {
    /// Supervisor convenience handshake. Configuration contains a private unlock
    /// credential and must reach only the provisioned service over private pipes.
    /// Runtime consumers can instead receive brokered handles via from_ready.
    pub fn connect(
        input: R,
        mut output: W,
        config: &WitnessServiceConfig,
        expected_witness_public: [u8; 32],
    ) -> io::Result<Self> {
        write_frame(&mut output, config)?;
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
        Self::from_ready(
            input,
            output,
            expected_witness_public,
            config.writer_public,
            config.node_id.clone(),
            bootstrap,
        )
    }

    /// Consume one readiness reply after a trusted broker sent configuration.
    /// Existing state must authenticate and consistency-extend the external pin;
    /// explicit CreateNew requires an actually empty readiness commitment.
    pub fn from_ready(
        mut input: R,
        output: W,
        expected_witness_public: [u8; 32],
        expected_writer_public: [u8; 32],
        node: String,
        bootstrap: WitnessBootstrap<'_>,
    ) -> io::Result<Self> {
        if node.is_empty() || node.len() > 4096 || expected_witness_public == expected_writer_public
        {
            return Err(invalid());
        }
        let WitnessServiceResponse::Ready {
            witness_public,
            checkpoint,
            ..
        } = response(&mut input)?
        else {
            return Err(invalid());
        };
        if witness_public != expected_witness_public {
            return Err(invalid());
        }
        let client = Self {
            input,
            output,
            witness: expected_witness_public,
            writer: expected_writer_public,
            node,
            head: checkpoint,
            poisoned: false,
        };
        match bootstrap {
            WitnessBootstrap::CreateNew => {
                if client.head.is_some() {
                    return Err(invalid());
                }
            }
            WitnessBootstrap::Pinned {
                minimum,
                consistency_proof,
            } => {
                client.authenticate(minimum)?;
                let head = client.head.as_ref().ok_or_else(invalid)?;
                client.authenticate(head)?;
                continuity(Some(minimum), head, consistency_proof)?;
            }
        }
        Ok(client)
    }

    fn healthy(&self) -> io::Result<()> {
        if self.poisoned {
            Err(invalid())
        } else {
            Ok(())
        }
    }

    fn authenticate(&self, checkpoint: &Checkpoint) -> io::Result<()> {
        self.writer_checkpoint(checkpoint)?;
        if !checkpoint.verify_witness(&self.witness) {
            return Err(invalid());
        }
        Ok(())
    }

    fn writer_checkpoint(&self, checkpoint: &Checkpoint) -> io::Result<()> {
        if checkpoint.node_id != self.node
            || !checkpoint.verify_writer(&self.writer)
            || checkpoint.tree_size == 0
        {
            return Err(invalid());
        }
        hash(&checkpoint.root)?;
        hash(&checkpoint.merkle_root)?;
        Ok(())
    }
}

impl<R: BufRead + Send, W: Write + Send> CheckpointWitness for WitnessServiceClient<R, W> {
    fn public_key(&self) -> [u8; 32] {
        self.witness
    }
    fn last_size(&self) -> io::Result<u64> {
        self.healthy()?;
        Ok(self.head.as_ref().map_or(0, |head| head.tree_size))
    }
    fn validate_local_head(&self, checkpoint: &Checkpoint) -> io::Result<()> {
        self.healthy()?;
        if self.head.as_ref() == Some(checkpoint) {
            Ok(())
        } else {
            Err(invalid())
        }
    }
    fn counter_sign(
        &mut self,
        checkpoint: &Checkpoint,
        proof: &[Hash],
    ) -> io::Result<WitnessReceipt> {
        self.healthy()?;
        // A refusal, malformed response or I/O error leaves the client poisoned.
        // It must be reconnected against an independently pinned latest head.
        self.poisoned = true;
        self.writer_checkpoint(checkpoint)?;
        continuity(self.head.as_ref(), checkpoint, proof)?;
        write_frame(
            &mut self.output,
            &WitnessServiceRequest::CounterSign {
                checkpoint: checkpoint.clone(),
                consistency_proof: proof.to_vec(),
            },
        )?;
        let WitnessServiceResponse::Signed {
            checkpoint: signed,
            persistence,
        } = response(&mut self.input)?
        else {
            return Err(invalid());
        };
        let mut exact = checkpoint.clone();
        exact.witness_sig = signed.witness_sig.clone();
        if signed != exact {
            return Err(invalid());
        }
        self.authenticate(&signed)?;
        continuity(self.head.as_ref(), &signed, proof)?;
        let durability = match persistence {
            WitnessPersistence::FileAndDirectory => Durability::FileAndDirectory,
            WitnessPersistence::FileSyncedNamespaceUnconfirmed => {
                Durability::FileSyncedNamespaceUnconfirmed
            }
        };
        self.head = Some(signed.clone());
        self.poisoned = false;
        Ok(WitnessReceipt {
            checkpoint: signed,
            durability,
        })
    }
}

fn response(input: &mut impl BufRead) -> io::Result<WitnessServiceResponse> {
    let bytes = read_frame(input)?.ok_or_else(invalid)?;
    serde_json::from_slice(&bytes).map_err(|_| invalid())
}

fn hash(value: &str) -> io::Result<Hash> {
    let decoded: Hash = hex::decode(value)
        .map_err(|_| invalid())?
        .try_into()
        .map_err(|_| invalid())?;
    if hex::encode(decoded) != value {
        return Err(invalid());
    }
    Ok(decoded)
}

fn continuity(previous: Option<&Checkpoint>, next: &Checkpoint, proof: &[Hash]) -> io::Result<()> {
    if proof.len() > 128 {
        return Err(invalid());
    }
    let Some(previous) = previous else {
        return if proof.is_empty() {
            Ok(())
        } else {
            Err(invalid())
        };
    };
    if next.tree_size < previous.tree_size {
        return Err(invalid());
    }
    if next.tree_size == previous.tree_size {
        if previous.root != next.root
            || previous.merkle_root != next.merkle_root
            || !proof.is_empty()
        {
            return Err(invalid());
        }
        return Ok(());
    }
    let old = usize::try_from(previous.tree_size).map_err(|_| invalid())?;
    let new = usize::try_from(next.tree_size).map_err(|_| invalid())?;
    if !verify_consistency(
        old,
        new,
        proof,
        &hash(&previous.merkle_root)?,
        &hash(&next.merkle_root)?,
    ) {
        return Err(invalid());
    }
    Ok(())
}

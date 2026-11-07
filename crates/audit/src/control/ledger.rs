use super::{Action, Administrators, Approval, Proposal, target_commitment};
use crate::{
    keys::{
        custody::{Durability, Owner},
        vault::ErasurePermit,
    },
    storage::PrivateState,
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io, path::Path};

/// Externally retained latest control-ledger anchor. Never infer freshness from
/// the local ledger alone; rollback protection depends on its independent copy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerAnchor {
    /// Strict monotonic transition number.
    pub revision: u64,
    /// SHA256 commitment to the canonical ledger body.
    pub digest: String,
    /// Authority Ed25519 signature over domain-separated revision and digest.
    pub signature: String,
}
/// Explicit provisioning or exact independent latest-anchor restore.
pub enum LedgerBootstrap<'a> {
    /// Provision only when both committed and pending state are absent.
    New,
    /// Require this independently retained latest signed anchor.
    Pinned(&'a LedgerAnchor),
}
/// Durable authorization consumption receipt. It does not assert external action
/// completion; retain its anchor independently before acknowledging that action.
#[derive(Clone, Debug)]
pub struct ControlReceipt {
    /// New externally retainable signed anchor.
    pub anchor: LedgerAnchor,
    /// Approved operation identity, not a reusable bearer credential.
    pub operation_id: String,
    /// Concrete platform persistence strength.
    pub durability: Durability,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Spent {
    proposal: Proposal,
    digest: String,
    administrators: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Body {
    policy: String,
    revision: u64,
    last_time: u64,
    spent: BTreeMap<String, Spent>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    body: Body,
    anchor: LedgerAnchor,
}
/// Private single-writer administrator issuer. Seeds stay outside serialized
/// state; no transport/header/boolean can construct its subject-bound permits.
pub struct ControlAuthority {
    store: PrivateState,
    key: SigningKey,
    policy: Administrators,
    body: Body,
    anchor: LedgerAnchor,
    capacity: usize,
    observed_time: u64,
    clock_poisoned: bool,
}
fn invalid() -> io::Error {
    io::Error::other("control ledger authentication, replay or scope failure")
}
fn anchor_bytes(revision: u64, digest: &str) -> Vec<u8> {
    format!("humanymous/control-ledger/v1\0{revision}\0{digest}").into_bytes()
}
fn make_anchor(key: &SigningKey, body: &Body) -> io::Result<LedgerAnchor> {
    let bytes = serde_json::to_vec(body).map_err(|_| invalid())?;
    let digest = hex::encode(Sha256::digest(bytes));
    Ok(LedgerAnchor {
        revision: body.revision,
        signature: hex::encode(key.sign(&anchor_bytes(body.revision, &digest)).to_bytes()),
        digest,
    })
}
fn authenticate(
    bytes: &[u8],
    key: &SigningKey,
    policy: &Administrators,
    capacity: usize,
) -> io::Result<Snapshot> {
    let snapshot: Snapshot = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    let expected = make_anchor(key, &snapshot.body)?;
    if snapshot.anchor != expected
        || snapshot.body.policy != policy.fingerprint()
        || snapshot.body.spent.len() > capacity
        || snapshot.body.revision != snapshot.body.spent.len() as u64
    {
        return Err(invalid());
    }
    for (id, spent) in &snapshot.body.spent {
        if id != &spent.proposal.operation_id
            || spent.digest != spent.proposal.digest().map_err(|_| invalid())?
            || spent.administrators.len() < 2
        {
            return Err(invalid());
        }
    }
    Ok(snapshot)
}
impl LedgerAnchor {
    /// Verify using only the independently provisioned authority public key.
    pub fn verify(&self, public: &[u8; 32]) -> bool {
        let Ok(key) = VerifyingKey::from_bytes(public) else {
            return false;
        };
        let Ok(bytes) = hex::decode(&self.signature) else {
            return false;
        };
        let Ok(signature) = Signature::from_slice(&bytes) else {
            return false;
        };
        self.digest.len() == 64
            && self
                .digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && key
                .verify_strict(&anchor_bytes(self.revision, &self.digest), &signature)
                .is_ok()
    }
}
impl ControlAuthority {
    /// Open a preprovisioned private directory and authenticate all spent state.
    /// Fresh creation is explicit; pinned state must match exactly on restart.
    pub fn open(
        directory: &Path,
        owner: Owner,
        seed: &[u8; 32],
        policy: Administrators,
        capacity: usize,
        bootstrap: LedgerBootstrap<'_>,
    ) -> io::Result<Self> {
        if capacity == 0 || capacity > 10000 {
            return Err(invalid());
        }
        let mut store = PrivateState::open(directory, owner, "control")?;
        let key = SigningKey::from_bytes(seed);
        let (body, anchor) = match (store.read()?, bootstrap) {
            (None, LedgerBootstrap::New) => {
                let body = Body {
                    policy: policy.fingerprint(),
                    revision: 0,
                    last_time: 0,
                    spent: BTreeMap::new(),
                };
                let anchor = make_anchor(&key, &body)?;
                store.commit(
                    &serde_json::to_vec(&Snapshot {
                        body: body.clone(),
                        anchor: anchor.clone(),
                    })
                    .map_err(|_| invalid())?,
                )?;
                (body, anchor)
            }
            (Some(bytes), LedgerBootstrap::Pinned(pin)) => {
                let snapshot = authenticate(&bytes, &key, &policy, capacity)?;
                if &snapshot.anchor != pin || !pin.verify(&key.verifying_key().to_bytes()) {
                    return Err(invalid());
                }
                (snapshot.body, snapshot.anchor)
            }
            _ => return Err(invalid()),
        };
        let observed_time = body.last_time;
        Ok(Self {
            store,
            key,
            policy,
            body,
            anchor,
            capacity,
            observed_time,
            clock_poisoned: false,
        })
    }
    /// Current signed external freshness anchor. Copy it to independent storage.
    pub fn anchor(&self) -> &LedgerAnchor {
        &self.anchor
    }
    /// Public ledger verification identity; never exposes the seed.
    pub fn public_key(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }
    /// Verify two signatures and persist nonce consumption before any effect.
    /// No id is evicted. Unknown output after an I/O failure requires recovery.
    pub fn consume(
        &mut self,
        proposal: &Proposal,
        approvals: &[Approval],
        now: u64,
    ) -> io::Result<ControlReceipt> {
        if self.clock_poisoned || now < self.observed_time {
            self.clock_poisoned = true;
            return Err(invalid());
        }
        self.observed_time = now;
        if now < self.body.last_time
            || self.body.spent.contains_key(&proposal.operation_id)
            || self.body.spent.len() >= self.capacity
        {
            return Err(invalid());
        }
        let authorization = self
            .policy
            .authorize(proposal, approvals, now)
            .map_err(|_| invalid())?;
        let mut body = self.body.clone();
        body.revision = body.revision.checked_add(1).ok_or_else(invalid)?;
        body.last_time = now;
        body.spent.insert(
            proposal.operation_id.clone(),
            Spent {
                proposal: proposal.clone(),
                digest: authorization.digest().to_owned(),
                administrators: authorization.administrators().to_vec(),
            },
        );
        let anchor = make_anchor(&self.key, &body)?;
        let durability = self.store.commit(
            &serde_json::to_vec(&Snapshot {
                body: body.clone(),
                anchor: anchor.clone(),
            })
            .map_err(|_| invalid())?,
        )?;
        self.body = body;
        self.anchor = anchor.clone();
        Ok(ControlReceipt {
            anchor,
            operation_id: proposal.operation_id.clone(),
            durability,
        })
    }
    /// Issue an exact-subject erasure capability only after durable two-admin
    /// approval consumption. A different subject cannot reuse this permit.
    pub fn issue_erasure(
        &mut self,
        proposal: &Proposal,
        approvals: &[Approval],
        now: u64,
        subject: &str,
    ) -> io::Result<(ErasurePermit, ControlReceipt)> {
        if subject.is_empty()
            || subject.len() > 4096
            || proposal.action != Action::EraseSubject
            || proposal.target != target_commitment(subject.as_bytes())
        {
            return Err(invalid());
        }
        let receipt = self.consume(proposal, approvals, now)?;
        Ok((
            ErasurePermit::issued(subject, &proposal.operation_id, &proposal.scope),
            receipt,
        ))
    }
    /// Authenticate exact membership in the currently pinned spent ledger.
    /// Used solely to reconcile that same operation after an interrupted effect.
    pub fn verify_consumed(&self, proposal: &Proposal) -> io::Result<()> {
        self.store.read()?;
        let spent = self
            .body
            .spent
            .get(&proposal.operation_id)
            .ok_or_else(invalid)?;
        if &spent.proposal != proposal
            || spent.digest != proposal.digest().map_err(|_| invalid())?
        {
            return Err(invalid());
        }
        Ok(())
    }
    /// Generate a fresh proposal ID at the trusted service boundary. Signers
    /// receive these exact bytes; no request-supplied clock is trusted here.
    pub fn propose(
        &self,
        action: Action,
        target: String,
        issued_at: u64,
        expires_at: u64,
    ) -> io::Result<Proposal> {
        let mut nonce = [0u8; 32];
        getrandom::fill(&mut nonce).map_err(|_| invalid())?;
        let proposal = Proposal {
            scope: self.policy.scope().clone(),
            action,
            target,
            operation_id: hex::encode(nonce),
            issued_at,
            expires_at,
        };
        proposal.signing_bytes().map_err(|_| invalid())?;
        Ok(proposal)
    }
    /// Reissue only the already consumed exact subject capability for crash
    /// recovery. Fresh signatures must still satisfy the current approval window.
    pub fn resume_erasure(
        &mut self,
        proposal: &Proposal,
        approvals: &[Approval],
        now: u64,
        subject: &str,
    ) -> io::Result<ErasurePermit> {
        if self.clock_poisoned || now < self.observed_time {
            self.clock_poisoned = true;
            return Err(invalid());
        }
        self.observed_time = now;
        self.policy
            .authorize(proposal, approvals, now)
            .map_err(|_| invalid())?;
        self.verify_consumed(proposal)?;
        if proposal.action != Action::EraseSubject
            || proposal.target != target_commitment(subject.as_bytes())
        {
            return Err(invalid());
        }
        Ok(ErasurePermit::issued(
            subject,
            &proposal.operation_id,
            &proposal.scope,
        ))
    }
    /// Explicitly recover a complete staged ledger transition against the exact
    /// last external anchor. No unsigned reset or spent-id truncation is allowed.
    pub fn recover_pending(
        directory: &Path,
        owner: Owner,
        seed: &[u8; 32],
        policy: Administrators,
        capacity: usize,
        pin: &LedgerAnchor,
    ) -> io::Result<(Self, ControlReceipt)> {
        if capacity == 0 || capacity > 10000 {
            return Err(invalid());
        }
        let mut store = PrivateState::open(directory, owner, "control")?;
        let key = SigningKey::from_bytes(seed);
        let old = authenticate(
            &store.committed()?.ok_or_else(invalid)?,
            &key,
            &policy,
            capacity,
        )?;
        let pending = authenticate(
            &store.pending()?.ok_or_else(invalid)?,
            &key,
            &policy,
            capacity,
        )?;
        if &old.anchor != pin
            || pending.body.revision != old.body.revision.checked_add(1).ok_or_else(invalid)?
            || pending.body.last_time < old.body.last_time
            || old
                .body
                .spent
                .iter()
                .any(|(id, value)| pending.body.spent.get(id) != Some(value))
        {
            return Err(invalid());
        }
        let operation_id = pending
            .body
            .spent
            .keys()
            .find(|id| !old.body.spent.contains_key(*id))
            .ok_or_else(invalid)?
            .clone();
        let durability = store.promote_pending()?;
        let receipt = ControlReceipt {
            anchor: pending.anchor.clone(),
            operation_id,
            durability,
        };
        let observed_time = pending.body.last_time;
        Ok((
            Self {
                store,
                key,
                policy,
                body: pending.body,
                anchor: pending.anchor,
                capacity,
                observed_time,
                clock_poisoned: false,
            },
            receipt,
        ))
    }
}

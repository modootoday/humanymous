//! Private local sealed backups with signed, externally pinned inventory.
//! Retirement removes one named local file; it does not prove physical secure
//! deletion, removal of filesystem snapshots, or deletion of remote copies.
mod retained;
use super::{
    KeyMaterial, LIMIT,
    custody::{Durability, Owner, permissions},
    open,
};
use crate::audit::{
    control::{
        Action, Approval, ControlAuthority, ControlReceipt, Proposal, Scope, target_commitment,
    },
    storage::PrivateState,
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

const DOMAIN: &[u8] = b"humanymous/sealed-backup-inventory/v1\0";

/// Signed exact latest inventory commitment, retained independently of backups.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupAnchor {
    /// Monotonic inventory transition count.
    pub revision: u64,
    /// SHA256 of the canonical inventory body.
    pub digest: String,
    /// Domain-separated Ed25519 signature over revision and digest.
    pub signature: String,
}
impl BackupAnchor {
    /// Authenticate the inventory anchor with a separately provisioned key.
    pub fn verify(&self, public: &[u8; 32]) -> bool {
        let Ok(key) = VerifyingKey::from_bytes(public) else {
            return false;
        };
        let Ok(signature) = hex::decode(&self.signature).and_then(|b| {
            <[u8; 64]>::try_from(b).map_err(|_| hex::FromHexError::InvalidStringLength)
        }) else {
            return false;
        };
        canonical_digest(&self.digest)
            && self.signature == hex::encode(signature)
            && key
                .verify_strict(
                    &anchor_bytes(self.revision, &self.digest),
                    &Signature::from_bytes(&signature),
                )
                .is_ok()
    }
}
/// Creation is explicit; an existing inventory requires its exact external pin.
pub enum BackupBootstrap<'a> {
    /// Refuse any committed/pending prior inventory.
    New,
    /// Require this independently retained latest signed inventory anchor.
    Pinned(&'a BackupAnchor),
}
/// Local backup lifecycle. A consumed retirement can be reconciled after crash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum BackupStatus {
    /// A hash-verified sealed envelope remains in this directory.
    Active,
    /// Administrator approvals were consumed before deletion; the local file
    /// may remain or may already be unlinked after a crash.
    Retiring {
        /// Exact consumed proposal, including scope and target commitment.
        proposal: Proposal,
    },
    /// The named local file was absent after the platform unlink/flush boundary.
    /// Inspect receipt durability; Windows directory flush is not asserted.
    Retired {
        /// Consumed two-administrator operation identity.
        operation_id: String,
    },
}
/// Non-secret content-addressed backup metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupEntry {
    /// SHA256 of the exact encrypted envelope bytes; also its immutable basename.
    pub digest: String,
    /// Exact encrypted envelope length.
    pub bytes: u64,
    /// Local file retirement state, not global backup/erasure completion.
    pub status: BackupStatus,
}
/// Completed inventory transition with honest platform persistence accounting.
#[derive(Debug, Clone)]
pub struct BackupReceipt {
    /// Independently retain before acknowledging the transition.
    pub anchor: BackupAnchor,
    /// Native platform flush strength; Windows namespace durability is unproven.
    pub durability: Durability,
    /// Exact backup digest involved in the transition.
    pub digest: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Body {
    scope: Scope,
    revision: u64,
    entries: BTreeMap<String, BackupEntry>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    body: Body,
    anchor: BackupAnchor,
}

/// One local owner-controlled archive and its signed single-writer inventory.
/// Signing seed and external anchor custody are deployment responsibilities.
pub struct BackupInventory {
    store: PrivateState,
    directory: PathBuf,
    owner: Owner,
    key: SigningKey,
    body: Body,
    anchor: BackupAnchor,
    capacity: usize,
    poisoned: bool,
}
fn invalid() -> io::Error {
    io::Error::other("sealed backup inventory authentication or state failure")
}
fn canonical_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn anchor_bytes(revision: u64, digest: &str) -> Vec<u8> {
    let mut bytes = DOMAIN.to_vec();
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(digest.as_bytes());
    bytes
}
fn make_anchor(key: &SigningKey, body: &Body) -> io::Result<BackupAnchor> {
    let digest = hex::encode(Sha256::digest(
        serde_json::to_vec(body).map_err(|_| invalid())?,
    ));
    let signature = hex::encode(key.sign(&anchor_bytes(body.revision, &digest)).to_bytes());
    Ok(BackupAnchor {
        revision: body.revision,
        digest,
        signature,
    })
}
fn authenticate(
    bytes: &[u8],
    key: &SigningKey,
    scope: &Scope,
    capacity: usize,
) -> io::Result<Snapshot> {
    let snapshot: Snapshot = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if &snapshot.body.scope != scope
        || snapshot.body.entries.len() > capacity
        || make_anchor(key, &snapshot.body)? != snapshot.anchor
    {
        return Err(invalid());
    }
    for (digest, entry) in &snapshot.body.entries {
        if !canonical_digest(digest)
            || &entry.digest != digest
            || entry.bytes == 0
            || entry.bytes > LIMIT as u64
        {
            return Err(invalid());
        }
        match &entry.status {
            BackupStatus::Retiring { proposal }
                if proposal.scope != *scope
                    || proposal.action != Action::RetireBackup
                    || proposal.target != target_commitment(digest.as_bytes())
                    || proposal.signing_bytes().is_err() =>
            {
                return Err(invalid());
            }
            BackupStatus::Retired { operation_id } if !canonical_digest(operation_id) => {
                return Err(invalid());
            }
            _ => {}
        }
    }
    Ok(snapshot)
}
fn copy_owner(owner: &Owner) -> Owner {
    match owner {
        #[cfg(unix)]
        Owner::CurrentUser => Owner::CurrentUser,
        #[cfg(windows)]
        Owner::WindowsSid(sid) => Owner::WindowsSid(sid.clone()),
    }
}
impl BackupInventory {
    /// Open an existing private directory. Existing state must match the exact
    /// externally pinned anchor; malformed/missing state never becomes new.
    pub fn open(
        directory: &Path,
        owner: Owner,
        seed: &[u8; 32],
        scope: Scope,
        capacity: usize,
        bootstrap: BackupBootstrap<'_>,
    ) -> io::Result<Self> {
        if capacity == 0
            || capacity > 10_000
            || scope.installation.is_empty()
            || scope.node.is_empty()
        {
            return Err(invalid());
        }
        let mut store = PrivateState::open(directory, copy_owner(&owner), "backups")?;
        let directory = fs::canonicalize(directory)?;
        let key = SigningKey::from_bytes(seed);
        let (body, anchor) = match (store.read()?, bootstrap) {
            (None, BackupBootstrap::New) => {
                let body = Body {
                    scope,
                    revision: 0,
                    entries: BTreeMap::new(),
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
            (Some(bytes), BackupBootstrap::Pinned(pin)) => {
                let snapshot = authenticate(&bytes, &key, &scope, capacity)?;
                if &snapshot.anchor != pin || !pin.verify(&key.verifying_key().to_bytes()) {
                    return Err(invalid());
                }
                (snapshot.body, snapshot.anchor)
            }
            _ => return Err(invalid()),
        };
        let inventory = Self {
            store,
            directory,
            owner,
            key,
            body,
            anchor,
            capacity,
            poisoned: false,
        };
        inventory.validate_files()?;
        Ok(inventory)
    }
    /// Exact latest signed anchor; retain outside the inventory failure domain.
    pub fn anchor(&self) -> &BackupAnchor {
        &self.anchor
    }
    /// Inventory signing identity, without its private seed.
    pub fn public_key(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }
    fn healthy(&self) -> io::Result<()> {
        if self.poisoned {
            return Err(invalid());
        }
        permissions::directory(&self.directory, &self.owner)
    }
    fn path(&self, digest: &str) -> io::Result<PathBuf> {
        if !canonical_digest(digest) {
            return Err(invalid());
        }
        Ok(self.directory.join(format!("{digest}.sealed")))
    }
    fn sealed(&self, digest: &str, expected: u64) -> io::Result<Option<Vec<u8>>> {
        let file = match permissions::existing(&self.path(digest)?, false) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        permissions::file(&file, &self.owner)?;
        if file.metadata()?.len() != expected || expected > LIMIT as u64 {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        file.take(LIMIT as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != expected || hex::encode(Sha256::digest(&bytes)) != digest {
            return Err(invalid());
        }
        Ok(Some(bytes))
    }
    fn validate_files(&self) -> io::Result<()> {
        self.healthy()?;
        for entry in self.body.entries.values() {
            let present = self.sealed(&entry.digest, entry.bytes)?.is_some();
            match entry.status {
                BackupStatus::Active if !present => return Err(invalid()),
                BackupStatus::Retired { .. } if present => return Err(invalid()),
                _ => {}
            }
        }
        Ok(())
    }
    /// Verified inventory entries; retired tombstones are retained indefinitely
    /// within the fixed capacity rather than evicted to admit new backups.
    pub fn list(&self) -> io::Result<Vec<BackupEntry>> {
        self.validate_files()?;
        Ok(self.body.entries.values().cloned().collect())
    }
    /// Content-addressed local files absent from the signed inventory. These
    /// remain explicit residuals after interrupted creation, never silently
    /// adopted or counted as retired.
    pub fn orphans(&self) -> io::Result<Vec<String>> {
        self.healthy()?;
        let mut found = Vec::new();
        for item in fs::read_dir(&self.directory)? {
            let item = item?;
            let name = item.file_name();
            let Some(name) = name.to_str() else { continue };
            if let Some(digest) = name.strip_suffix(".sealed")
                && canonical_digest(digest)
                && !self.body.entries.contains_key(digest)
            {
                let metadata = fs::symlink_metadata(item.path())?;
                self.sealed(digest, metadata.len())?.ok_or_else(invalid)?;
                found.push(digest.to_owned());
            }
        }
        found.sort();
        Ok(found)
    }
    fn save(&mut self, mut body: Body, digest: &str) -> io::Result<BackupReceipt> {
        body.revision = self.body.revision.checked_add(1).ok_or_else(invalid)?;
        let anchor = make_anchor(&self.key, &body)?;
        self.poisoned = true;
        let durability = self.store.commit(
            &serde_json::to_vec(&Snapshot {
                body: body.clone(),
                anchor: anchor.clone(),
            })
            .map_err(|_| invalid())?,
        )?;
        self.body = body;
        self.anchor = anchor.clone();
        self.poisoned = false;
        Ok(BackupReceipt {
            anchor,
            durability,
            digest: digest.into(),
        })
    }
    /// Authenticate the existing sealed format before immutable archival. No
    /// plaintext keys are persisted and wrong passwords create no backup file.
    pub fn create(&mut self, sealed: &[u8], passphrase: &[u8]) -> io::Result<BackupReceipt> {
        self.healthy()?;
        open(sealed, passphrase).map_err(|_| invalid())?;
        let digest = hex::encode(Sha256::digest(sealed));
        if self.body.entries.contains_key(&digest) || self.body.entries.len() >= self.capacity {
            return Err(invalid());
        }
        self.poisoned = true;
        let mut file = permissions::create(&self.path(&digest)?)?;
        permissions::file(&file, &self.owner)?;
        file.write_all(sealed)?;
        file.sync_all()?;
        drop(file);
        self.sync_directory()?;
        let mut body = self.body.clone();
        body.entries.insert(
            digest.clone(),
            BackupEntry {
                digest: digest.clone(),
                bytes: sealed.len() as u64,
                status: BackupStatus::Active,
            },
        );
        self.save(body, &digest)
    }
    /// Explicitly adopt a fully authenticated, hash-addressed orphan left after
    /// interrupted creation. Existing active/retired entries cannot be replaced.
    pub fn adopt_orphan(&mut self, digest: &str, passphrase: &[u8]) -> io::Result<BackupReceipt> {
        self.healthy()?;
        if self.body.entries.contains_key(digest) || self.body.entries.len() >= self.capacity {
            return Err(invalid());
        }
        let length = fs::symlink_metadata(self.path(digest)?)?.len();
        let sealed = self.sealed(digest, length)?.ok_or_else(invalid)?;
        open(&sealed, passphrase).map_err(|_| invalid())?;
        let file = permissions::existing(&self.path(digest)?, true)?;
        permissions::file(&file, &self.owner)?;
        file.sync_all()?;
        drop(file);
        self.sync_directory()?;
        let mut body = self.body.clone();
        body.entries.insert(
            digest.into(),
            BackupEntry {
                digest: digest.into(),
                bytes: length,
                status: BackupStatus::Active,
            },
        );
        self.save(body, digest)
    }
    /// Authenticate a current, hash-verified active backup into zeroized key
    /// material. Retiring/retired backups are never a restore source.
    pub fn read(&self, digest: &str, passphrase: &[u8]) -> io::Result<KeyMaterial> {
        self.healthy()?;
        let entry = self.body.entries.get(digest).ok_or_else(invalid)?;
        if entry.status != BackupStatus::Active {
            return Err(invalid());
        }
        open(
            &self.sealed(digest, entry.bytes)?.ok_or_else(invalid)?,
            passphrase,
        )
        .map_err(|_| invalid())
    }
    fn verify_retirement(
        &self,
        authority: &ControlAuthority,
        proposal: &Proposal,
    ) -> io::Result<String> {
        if proposal.scope != self.body.scope || proposal.action != Action::RetireBackup {
            return Err(invalid());
        }
        let digest = self
            .body
            .entries
            .keys()
            .find(|digest| target_commitment(digest.as_bytes()) == proposal.target)
            .ok_or_else(invalid)?
            .clone();
        authority.verify_consumed(proposal)?;
        Ok(digest)
    }
    /// Consume two signed administrator approvals before any deletion. A
    /// consumed proposal is not retried through this entrypoint after failure;
    /// use explicit reconciliation with authenticated issuer state instead.
    pub fn retire(
        &mut self,
        authority: &mut ControlAuthority,
        proposal: &Proposal,
        approvals: &[Approval],
        now: u64,
    ) -> io::Result<(BackupReceipt, ControlReceipt)> {
        self.healthy()?;
        if proposal.scope != self.body.scope || proposal.action != Action::RetireBackup {
            return Err(invalid());
        }
        let digest = self
            .body
            .entries
            .keys()
            .find(|digest| target_commitment(digest.as_bytes()) == proposal.target)
            .ok_or_else(invalid)?
            .clone();
        let entry = self.body.entries.get(&digest).ok_or_else(invalid)?;
        if entry.status != BackupStatus::Active {
            return Err(invalid());
        }
        self.sealed(&digest, entry.bytes)?.ok_or_else(invalid)?;
        let receipt = authority.consume(proposal, approvals, now)?;
        Ok((self.reconcile_retirement(authority, proposal)?, receipt))
    }
    /// Resume only an exact operation already consumed in the authenticated
    /// current authority. A user-constructed receipt or boolean cannot authorize
    /// this path. Missing local files are allowed only after that authorization.
    pub fn reconcile_retirement(
        &mut self,
        authority: &ControlAuthority,
        proposal: &Proposal,
    ) -> io::Result<BackupReceipt> {
        self.healthy()?;
        let digest = self.verify_retirement(authority, proposal)?;
        let entry = self.body.entries.get(&digest).ok_or_else(invalid)?.clone();
        match &entry.status {
            BackupStatus::Retired { .. } => return Err(invalid()),
            BackupStatus::Retiring { proposal: prior } if prior != proposal => {
                return Err(invalid());
            }
            BackupStatus::Active => {
                self.sealed(&digest, entry.bytes)?.ok_or_else(invalid)?;
                let mut body = self.body.clone();
                body.entries.get_mut(&digest).ok_or_else(invalid)?.status =
                    BackupStatus::Retiring {
                        proposal: proposal.clone(),
                    };
                self.save(body, &digest)?;
            }
            _ => {}
        }
        self.poisoned = true;
        if self.sealed(&digest, entry.bytes)?.is_some() {
            fs::remove_file(self.path(&digest)?)?;
        }
        self.sync_directory()?;
        let mut body = self.body.clone();
        body.entries.get_mut(&digest).ok_or_else(invalid)?.status = BackupStatus::Retired {
            operation_id: proposal.operation_id.clone(),
        };
        self.save(body, &digest)
    }
    fn sync_directory(&self) -> io::Result<()> {
        #[cfg(unix)]
        fs::File::open(&self.directory)?.sync_all()?;
        Ok(())
    }

    /// Explicitly resume a consumed retirement when a committed inventory step
    /// outran external-pin acknowledgement. Only this exact operation's one or
    /// two lifecycle transitions may extend the supplied pin; reconstruct and
    /// authenticate that precise prior body rather than accepting any newer
    /// signed inventory. No pending file is bypassed by this operation.
    #[allow(clippy::too_many_arguments)]
    pub fn resume_retirement(
        directory: &Path,
        owner: Owner,
        seed: &[u8; 32],
        scope: Scope,
        capacity: usize,
        pin: &BackupAnchor,
        authority: &ControlAuthority,
        proposal: &Proposal,
    ) -> io::Result<(Self, BackupReceipt)> {
        if capacity == 0
            || capacity > 10_000
            || proposal.scope != scope
            || proposal.action != Action::RetireBackup
        {
            return Err(invalid());
        }
        authority.verify_consumed(proposal)?;
        let store = PrivateState::open(directory, copy_owner(&owner), "backups")?;
        let key = SigningKey::from_bytes(seed);
        let snapshot = authenticate(&store.read()?.ok_or_else(invalid)?, &key, &scope, capacity)?;
        let digest = snapshot
            .body
            .entries
            .keys()
            .find(|digest| target_commitment(digest.as_bytes()) == proposal.target)
            .ok_or_else(invalid)?
            .clone();
        let entry = snapshot.body.entries.get(&digest).ok_or_else(invalid)?;
        let mut prior = snapshot.body.clone();
        let exact = snapshot.anchor == *pin;
        match &entry.status {
            BackupStatus::Retiring { proposal: previous } if previous == proposal => {
                prior.entries.get_mut(&digest).ok_or_else(invalid)?.status = BackupStatus::Active;
                prior.revision = prior.revision.checked_sub(1).ok_or_else(invalid)?;
                if !exact && make_anchor(&key, &prior)? != *pin {
                    return Err(invalid());
                }
            }
            BackupStatus::Retired { operation_id } if operation_id == &proposal.operation_id => {
                prior.entries.get_mut(&digest).ok_or_else(invalid)?.status =
                    BackupStatus::Retiring {
                        proposal: proposal.clone(),
                    };
                prior.revision = prior.revision.checked_sub(1).ok_or_else(invalid)?;
                let one_step = make_anchor(&key, &prior)? == *pin;
                prior.entries.get_mut(&digest).ok_or_else(invalid)?.status = BackupStatus::Active;
                prior.revision = prior.revision.checked_sub(1).ok_or_else(invalid)?;
                if !exact && !one_step && make_anchor(&key, &prior)? != *pin {
                    return Err(invalid());
                }
            }
            BackupStatus::Active if exact => {}
            _ => return Err(invalid()),
        }
        if !pin.verify(&key.verifying_key().to_bytes()) {
            return Err(invalid());
        }
        let mut inventory = Self {
            store,
            directory: fs::canonicalize(directory)?,
            owner,
            key,
            body: snapshot.body,
            anchor: snapshot.anchor,
            capacity,
            poisoned: false,
        };
        inventory.validate_files()?;
        let receipt = if matches!(
            inventory
                .body
                .entries
                .get(&digest)
                .ok_or_else(invalid)?
                .status,
            BackupStatus::Retired { .. }
        ) {
            inventory.sync_directory()?;
            BackupReceipt {
                anchor: inventory.anchor.clone(),
                durability: platform_durability(),
                digest,
            }
        } else {
            inventory.reconcile_retirement(authority, proposal)?
        };
        Ok((inventory, receipt))
    }
    /// Promote exactly one signed interrupted transition from the external pin.
    /// Any retiring/retired candidate also requires proof of consumption from
    /// the current pinned issuer, so inventory-key possession is not admin quorum.
    #[allow(clippy::too_many_arguments)]
    pub fn recover_pending(
        directory: &Path,
        owner: Owner,
        seed: &[u8; 32],
        scope: Scope,
        capacity: usize,
        pin: &BackupAnchor,
        authority: &ControlAuthority,
    ) -> io::Result<(Self, BackupReceipt)> {
        if capacity == 0 || capacity > 10_000 {
            return Err(invalid());
        }
        let store = PrivateState::open(directory, copy_owner(&owner), "backups")?;
        let key = SigningKey::from_bytes(seed);
        let old = authenticate(
            &store.committed()?.ok_or_else(invalid)?,
            &key,
            &scope,
            capacity,
        )?;
        let next = authenticate(
            &store.pending()?.ok_or_else(invalid)?,
            &key,
            &scope,
            capacity,
        )?;
        if &old.anchor != pin
            || next.body.revision != old.body.revision.checked_add(1).ok_or_else(invalid)?
        {
            return Err(invalid());
        }
        let changes: Vec<_> = next
            .body
            .entries
            .iter()
            .filter(|(id, entry)| old.body.entries.get(*id) != Some(*entry))
            .collect();
        if changes.len() != 1
            || old
                .body
                .entries
                .keys()
                .any(|id| !next.body.entries.contains_key(id))
        {
            return Err(invalid());
        }
        let (digest, entry) = changes[0];
        let recovered_digest = digest.clone();
        match (old.body.entries.get(digest), &entry.status) {
            (None, BackupStatus::Active) => {}
            (Some(prior), BackupStatus::Retiring { proposal })
                if prior.status == BackupStatus::Active =>
            {
                authority.verify_consumed(proposal)?;
            }
            (Some(prior), BackupStatus::Retired { operation_id }) => {
                let BackupStatus::Retiring { proposal } = &prior.status else {
                    return Err(invalid());
                };
                if operation_id != &proposal.operation_id {
                    return Err(invalid());
                }
                authority.verify_consumed(proposal)?;
            }
            _ => return Err(invalid()),
        }
        if let Some(prior) = old.body.entries.get(digest)
            && (prior.bytes != entry.bytes || prior.digest != entry.digest)
        {
            return Err(invalid());
        }
        let directory = fs::canonicalize(directory)?;
        let mut inventory = Self {
            store,
            directory,
            owner,
            key,
            body: next.body,
            anchor: next.anchor,
            capacity,
            poisoned: false,
        };
        inventory.validate_files()?;
        let durability = inventory.store.promote_pending()?;
        let receipt = BackupReceipt {
            anchor: inventory.anchor.clone(),
            durability,
            digest: recovered_digest,
        };
        Ok((inventory, receipt))
    }
}

fn platform_durability() -> Durability {
    #[cfg(unix)]
    {
        Durability::FileAndDirectory
    }
    #[cfg(windows)]
    {
        Durability::FileSyncedNamespaceUnconfirmed
    }
}

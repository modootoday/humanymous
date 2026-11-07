//! Exclusive local sealed custody. Provision the service directory privately
//! before opening; trusted ancestors and same-owner processes remain premises.
pub(crate) mod permissions;
mod rotation;
pub use rotation::PreparedRotation;
#[cfg(all(test, unix))]
mod tests;
use super::{KeyMaterial, LIMIT, open, seal, vault::Vault};
use crate::storage::ExclusiveFile;
#[cfg(unix)]
use std::fs::File;
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

/// Configured native owner. Supplied by the trusted service supervisor.
#[derive(Clone)]
pub enum Owner {
    /// Current effective UID on Unix; no alternate user is accepted.
    #[cfg(unix)]
    CurrentUser,
    /// Exact Windows service-owner SID, not an account name or DACL group.
    #[cfg(windows)]
    WindowsSid(String),
}
/// Explicit persistence strength of a successful replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    /// File and parent directory sync completed on a local Unix filesystem.
    FileAndDirectory,
    /// File synced; Windows rename namespace power-loss durability unconfirmed.
    FileSyncedNamespaceUnconfirmed,
}
/// A lifetime single-writer guard. Never unlink its lock file. No secret Debug.
pub struct Custody {
    dir: PathBuf,
    owner: Owner,
    _lock: ExclusiveFile,
    poisoned: bool,
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn crypto(_: super::KeyError) -> io::Error {
    invalid("sealed custody authentication or material failure")
}
impl Custody {
    /// Acquire nonblocking exclusive custody of an existing private directory.
    /// All path components must be non-aliases. Does not repair permissions.
    pub fn acquire(directory: &Path, owner: Owner) -> io::Result<Self> {
        if !directory.is_absolute() {
            return Err(invalid("custody path must be absolute"));
        }
        permissions::directory(directory, &owner)?;
        let dir = fs::canonicalize(directory)?;
        let lock_path = dir.join("keys.lock");
        let lock = match permissions::create(&lock_path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                permissions::existing(&lock_path, true)?
            }
            Err(e) => return Err(e),
        };
        permissions::file(&lock, &owner)?;
        let lock = ExclusiveFile::acquire(lock)?;
        Ok(Self {
            dir,
            owner,
            _lock: lock,
            poisoned: false,
        })
    }
    fn ready(&self) -> io::Result<()> {
        if self.poisoned {
            return Err(invalid("custody requires reopen after failed commit"));
        }
        permissions::directory(&self.dir, &self.owner)
    }
    fn exists(&self, name: &str) -> io::Result<bool> {
        match fs::symlink_metadata(self.dir.join(name)) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }
    fn read(&self, name: &str, passphrase: &[u8]) -> io::Result<KeyMaterial> {
        let file = permissions::existing(&self.dir.join(name), false)?;
        permissions::file(&file, &self.owner)?;
        if file.metadata()?.len() > LIMIT as u64 {
            return Err(invalid("sealed custody size limit"));
        }
        let mut bytes = Vec::new();
        file.take(LIMIT as u64 + 1).read_to_end(&mut bytes)?;
        open(&bytes, passphrase).map_err(crypto)
    }
    /// Authenticate the committed identity, or explicitly provision once when
    /// neither committed nor pending state exists. Failure never calls create.
    /// Hold this guard while using returned material to prevent stale writers.
    pub fn load_or_create(
        &mut self,
        passphrase: &[u8],
        create: impl FnOnce() -> Result<KeyMaterial, super::KeyError>,
    ) -> io::Result<(KeyMaterial, Option<Durability>)> {
        self.ready()?;
        if self.exists("keys.pending")? {
            return Err(invalid(
                "pending custody transaction requires explicit recovery",
            ));
        }
        if self.exists("keys.sealed")? {
            return Ok((self.read("keys.sealed", passphrase)?, None));
        }
        super::check_passphrase(passphrase, false).map_err(crypto)?;
        let material = create().map_err(crypto)?;
        let durability = self.commit(&material, passphrase)?;
        Ok((material, Some(durability)))
    }
    /// Replace only the authenticated current identity. Caller-supplied vault
    /// changes are explicit; stale backups must not be submitted as current state.
    fn replace(
        &mut self,
        material: &KeyMaterial,
        old_passphrase: &[u8],
        new_passphrase: &[u8],
    ) -> io::Result<Durability> {
        self.ready()?;
        if self.exists("keys.pending")? {
            return Err(invalid("pending custody transaction requires recovery"));
        }
        let current = self.read("keys.sealed", old_passphrase)?;
        same_identity(&current, material)?;
        self.commit(material, new_passphrase)
    }
    /// Rotate only the encryption passphrase, preserving the complete snapshot.
    pub fn rotate(
        &mut self,
        old_passphrase: &[u8],
        new_passphrase: &[u8],
    ) -> io::Result<Durability> {
        self.ready()?;
        let current = self.read("keys.sealed", old_passphrase)?;
        self.replace(&current, old_passphrase, new_passphrase)
    }
    /// Seal a vault snapshot with the existing writer identity; this does not
    /// issue erasure authority or acknowledge deletion of historical backups.
    /// Runtime must serialize vault mutation and persistence before acknowledging
    /// new pseudonyms; a snapshot captures one point in time under the vault lock.
    pub fn persist_vault(&mut self, vault: &Vault, passphrase: &[u8]) -> io::Result<Durability> {
        self.persist_snapshot(vault, passphrase, None)
    }
    /// Persist permitted erasure using the durable issuer's exact subject/scope
    /// capability. Ordinary snapshot persistence cannot bypass this check.
    pub fn persist_erasure(
        &mut self,
        vault: &Vault,
        passphrase: &[u8],
        permit: &super::vault::ErasurePermit,
    ) -> io::Result<Durability> {
        if !vault.authorizes(permit) {
            return Err(crypto(super::KeyError::InvalidMaterial));
        }
        self.persist_snapshot(vault, passphrase, Some(permit))
    }
    fn persist_snapshot(
        &mut self,
        vault: &Vault,
        passphrase: &[u8],
        erasure: Option<&super::vault::ErasurePermit>,
    ) -> io::Result<Durability> {
        self.ready()?;
        let current = self.read("keys.sealed", passphrase)?;
        let snapshot = vault.snapshot().map_err(crypto)?;
        super::vault::validate_update(current.vault(), &snapshot, erasure).map_err(crypto)?;
        let material = KeyMaterial::new(
            *current.signing_seed(),
            current.hmac_key().to_vec(),
            Some(snapshot.to_vec()),
            current.witness_seed().copied(),
        )
        .map_err(crypto)?;
        self.replace(&material, passphrase, passphrase)
    }
    /// Explicitly promote an interrupted encrypted transaction. Existing state
    /// must also authenticate and retain its identity. Never guess by timestamp.
    /// Accepting a pending snapshot is an operator decision, not automatic boot.
    pub fn recover_pending(
        &mut self,
        committed_passphrase: &[u8],
        pending_passphrase: &[u8],
    ) -> io::Result<Durability> {
        self.recover_inner(committed_passphrase, pending_passphrase, None)
    }
    /// Complete an interrupted erasure transaction only for its exact issued
    /// subject capability. A different tombstone cannot be smuggled in recovery.
    pub fn recover_erasure_pending(
        &mut self,
        committed_passphrase: &[u8],
        pending_passphrase: &[u8],
        permit: &super::vault::ErasurePermit,
    ) -> io::Result<Durability> {
        self.recover_inner(committed_passphrase, pending_passphrase, Some(permit))
    }
    fn recover_inner(
        &mut self,
        committed_passphrase: &[u8],
        pending_passphrase: &[u8],
        permit: Option<&super::vault::ErasurePermit>,
    ) -> io::Result<Durability> {
        self.ready()?;
        let pending = self.read("keys.pending", pending_passphrase)?;
        if self.exists("keys.sealed")? {
            let current = self.read("keys.sealed", committed_passphrase)?;
            same_identity(&current, &pending)?;
            // Generic recovery cannot manufacture erasure authorization. New
            // tombstones require the exact issuer permit in the typed variant.
            super::vault::validate_update(
                current.vault(),
                pending.vault().unwrap_or_default(),
                permit,
            )
            .map_err(crypto)?;
        }
        self.poisoned = true;
        let result = self.promote();
        if result.is_ok() {
            self.poisoned = false;
        }
        result
    }
    /// Exact currently committed encrypted generation identity for a signed
    /// rotation proposal. Never logs or returns the plaintext key material.
    pub fn committed_digest(&self) -> io::Result<String> {
        use sha2::{Digest, Sha256};
        self.ready()?;
        let file = permissions::existing(&self.dir.join("keys.sealed"), false)?;
        permissions::file(&file, &self.owner)?;
        let mut bytes = Vec::new();
        file.take(LIMIT as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > LIMIT {
            return Err(crypto(super::KeyError::SizeLimit));
        }
        Ok(hex::encode(Sha256::digest(bytes)))
    }
    /// Operational rotation consumes two signed approvals bound to the exact
    /// current ciphertext digest before re-encrypting the unchanged identity.
    pub fn rotate_approved(
        &mut self,
        authority: &mut crate::control::ControlAuthority,
        request: crate::control::ApprovalRequest<'_>,
        old_passphrase: &[u8],
        new_passphrase: &[u8],
    ) -> io::Result<(Durability, crate::control::ControlReceipt)> {
        use crate::control::{Action, target_commitment};
        let material = self.read("keys.sealed", old_passphrase)?;
        super::check_passphrase(new_passphrase, false).map_err(crypto)?;
        if request.proposal.action != Action::RotatePassphrase
            || request.proposal.target != target_commitment(self.committed_digest()?.as_bytes())
        {
            return Err(io::Error::other("rotation generation binding mismatch"));
        }
        let receipt = authority.consume(request.proposal, request.approvals, request.now)?;
        let durability = self.replace(&material, old_passphrase, new_passphrase)?;
        Ok((durability, receipt))
    }
    /// Consume two signed approvals, remove only their exact scoped subject,
    /// and persist the encrypted tombstone before returning completion.
    /// Backup retirement is deliberately separate and never inferred here.
    pub fn erase_approved(
        &mut self,
        vault: &Vault,
        passphrase: &[u8],
        authority: &mut crate::control::ControlAuthority,
        request: crate::control::ApprovalRequest<'_>,
        subject: &str,
    ) -> io::Result<(Durability, crate::control::ControlReceipt)> {
        self.ready()?;
        self.read("keys.sealed", passphrase)?;
        let (permit, receipt) =
            authority.issue_erasure(request.proposal, request.approvals, request.now, subject)?;
        vault.shred(subject, &permit).map_err(crypto)?;
        let durability = self.persist_erasure(vault, passphrase, &permit)?;
        Ok((durability, receipt))
    }
    fn commit(&mut self, material: &KeyMaterial, passphrase: &[u8]) -> io::Result<Durability> {
        let sealed = seal(material, passphrase).map_err(crypto)?;
        self.commit_envelope(&sealed)
    }
    fn commit_envelope(&mut self, sealed: &[u8]) -> io::Result<Durability> {
        if sealed.is_empty() || sealed.len() > LIMIT {
            return Err(crypto(super::KeyError::SizeLimit));
        }
        self.poisoned = true;
        let result = (|| {
            let mut pending = permissions::create(&self.dir.join("keys.pending"))?;
            permissions::file(&pending, &self.owner)?;
            pending.write_all(sealed)?;
            pending.sync_all()?;
            drop(pending);
            self.promote()
        })();
        if result.is_ok() {
            self.poisoned = false;
        }
        result
    }
    fn promote(&self) -> io::Result<Durability> {
        // Recovery also syncs pending before rename; authentication alone does
        // not prove an interrupted writer flushed its complete envelope.
        let pending = permissions::existing(&self.dir.join("keys.pending"), true)?;
        permissions::file(&pending, &self.owner)?;
        pending.sync_all()?;
        drop(pending);
        fs::rename(self.dir.join("keys.pending"), self.dir.join("keys.sealed"))?;
        #[cfg(unix)]
        {
            File::open(&self.dir)?.sync_all()?;
            Ok(Durability::FileAndDirectory)
        }
        #[cfg(windows)]
        {
            Ok(Durability::FileSyncedNamespaceUnconfirmed)
        }
    }
}
fn same_identity(a: &KeyMaterial, b: &KeyMaterial) -> io::Result<()> {
    use subtle::ConstantTimeEq;
    if !bool::from(a.signing_seed().ct_eq(b.signing_seed()))
        || !bool::from(a.hmac_key().ct_eq(b.hmac_key()))
        || a.witness_seed() != b.witness_seed()
    {
        return Err(invalid("custody identity replacement refused"));
    }
    Ok(())
}

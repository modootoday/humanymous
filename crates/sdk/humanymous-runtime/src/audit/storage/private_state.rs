//! Shared bounded private atomic state for independent control-plane owners.
use super::ExclusiveFile;
use crate::audit::keys::custody::{Durability, Owner, permissions};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
const LIMIT: u64 = 4 << 20;
pub(in crate::audit) struct PrivateState {
    dir: PathBuf,
    owner: Owner,
    namespace: String,
    _lock: ExclusiveFile,
    poisoned: bool,
    limit: u64,
}
impl PrivateState {
    pub(in crate::audit) fn open(dir: &Path, owner: Owner, namespace: &str) -> io::Result<Self> {
        Self::open_bounded(dir, owner, namespace, LIMIT)
    }
    pub(in crate::audit) fn open_bounded(
        dir: &Path,
        owner: Owner,
        namespace: &str,
        limit: u64,
    ) -> io::Result<Self> {
        if limit == 0 || limit > LIMIT {
            return Err(io::Error::other("invalid private state byte limit"));
        }
        if !dir.is_absolute()
            || namespace.is_empty()
            || namespace.len() > 64
            || !namespace
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'-')
        {
            return Err(io::Error::other("invalid private state namespace"));
        }
        permissions::directory(dir, &owner)?;
        let dir = fs::canonicalize(dir)?;
        let lock_path = dir.join(format!("{namespace}.lock"));
        let file = match permissions::create(&lock_path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                permissions::existing(&lock_path, true)?
            }
            Err(error) => return Err(error),
        };
        permissions::file(&file, &owner)?;
        Ok(Self {
            dir,
            owner,
            namespace: namespace.into(),
            _lock: ExclusiveFile::acquire(file)?,
            poisoned: false,
            limit,
        })
    }
    fn path(&self, suffix: &str) -> PathBuf {
        self.dir.join(format!("{}.{}", self.namespace, suffix))
    }
    fn healthy(&self) -> io::Result<()> {
        if self.poisoned {
            return Err(io::Error::other(
                "private state requires authenticated reopen",
            ));
        }
        permissions::directory(&self.dir, &self.owner)
    }
    fn read_file(&self, suffix: &str) -> io::Result<Option<Vec<u8>>> {
        self.healthy()?;
        let file = match permissions::existing(&self.path(suffix), false) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        permissions::file(&file, &self.owner)?;
        if file.metadata()?.len() > self.limit {
            return Err(io::Error::other("private state size limit"));
        }
        let mut bytes = Vec::new();
        file.take(self.limit + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > self.limit {
            return Err(io::Error::other("private state size limit"));
        }
        Ok(Some(bytes))
    }
    pub(in crate::audit) fn pending(&self) -> io::Result<Option<Vec<u8>>> {
        self.read_file("pending")
    }
    pub(in crate::audit) fn committed(&self) -> io::Result<Option<Vec<u8>>> {
        self.read_file("json")
    }
    pub(in crate::audit) fn read(&self) -> io::Result<Option<Vec<u8>>> {
        if self.pending()?.is_some() {
            return Err(io::Error::other("private state pending recovery required"));
        }
        self.committed()
    }
    pub(in crate::audit) fn commit(&mut self, bytes: &[u8]) -> io::Result<Durability> {
        self.healthy()?;
        if bytes.is_empty() || bytes.len() as u64 > self.limit {
            return Err(io::Error::other("private state size limit"));
        }
        if self.pending()?.is_some() {
            return Err(io::Error::other("private state pending recovery required"));
        }
        self.poisoned = true;
        let result = (|| {
            let mut file = permissions::create(&self.path("pending"))?;
            permissions::file(&file, &self.owner)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            self.promote()
        })();
        if result.is_ok() {
            self.poisoned = false;
        }
        result
    }
    // Caller authenticates committed and pending bytes before invoking this.
    pub(in crate::audit) fn promote_pending(&mut self) -> io::Result<Durability> {
        self.healthy()?;
        self.poisoned = true;
        let result = self.promote();
        if result.is_ok() {
            self.poisoned = false;
        }
        result
    }
    fn promote(&self) -> io::Result<Durability> {
        let file = permissions::existing(&self.path("pending"), true)?;
        permissions::file(&file, &self.owner)?;
        file.sync_all()?;
        drop(file);
        fs::rename(self.path("pending"), self.path("json"))?;
        #[cfg(unix)]
        {
            fs::File::open(&self.dir)?.sync_all()?;
            Ok(Durability::FileAndDirectory)
        }
        #[cfg(windows)]
        {
            Ok(Durability::FileSyncedNamespaceUnconfirmed)
        }
    }
}

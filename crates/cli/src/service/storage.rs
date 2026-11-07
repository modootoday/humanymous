//! Managed immutable generations, private state and fail-closed transaction records.
use super::backend::Identity;
use super::{Result, backend};
use crate::{
    Plan,
    installation::permissions::{plain, private_file, private_parent},
    release::{Descriptor, SignedRelease, error},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Generation {
    pub(super) release: SignedRelease,
    pub(super) identity: Option<Identity>,
    pub(super) sequence: u64,
    pub(super) digest: String,
}
impl Generation {
    pub(super) fn executable(&self, root: &Path) -> PathBuf {
        self.path(root)
            .join(crate::installation::service_artifact_name())
    }
    pub(super) fn new(release: SignedRelease, d: &Descriptor) -> Self {
        Self {
            release,
            identity: None,
            sequence: d.sequence,
            digest: d.artifact_sha256.clone(),
        }
    }
    pub(super) fn path(&self, root: &Path) -> PathBuf {
        root.join(format!(
            "generation-{}-{}",
            self.sequence,
            &self.digest[..16]
        ))
    }
    pub(super) fn definition_path(&self, root: &Path, d: &Descriptor) -> PathBuf {
        root.join(format!("definition-{}", self.sequence))
            .join(format!(
                "{}.{}",
                d.service_name,
                if d.os == "linux" { "service" } else { "plist" }
            ))
    }
    pub(super) fn descriptor(&self, key: &[u8; 32]) -> Result<Descriptor> {
        let d = self.release.verify(key)?;
        if self.sequence != d.sequence || self.digest != d.artifact_sha256 {
            return Err(error("generation state mismatch"));
        }
        Ok(d)
    }
    pub(super) fn plan(&self, root: &Path) -> Result<Plan> {
        let source = self.executable(root);
        plain(&source)?;
        Plan::prepared_service(
            read_bounded(&source, 256 * 1024 * 1024)?,
            &self.path(root),
            &self.digest,
        )
    }
    pub(super) fn verify(&self, root: &Path, key: &[u8; 32]) -> Result<()> {
        let d = self.descriptor(key)?;
        #[cfg(all(windows, feature = "runtime-service"))]
        if let Some(sid) = self
            .identity
            .as_ref()
            .and_then(|identity| identity.service_sid.as_deref())
        {
            crate::runtime_host::activation::verify(root, &d, sid)?;
        }
        self.plan(root)?.verify_service(
            self.identity
                .as_ref()
                .and_then(|i| i.service_sid.as_deref()),
        )?;
        if d.os != "windows" {
            let path = self.definition_path(root, &d);
            private_file(&path)?;
            if read_bounded(&path, 65_536)? != backend::definition(&d, &self.executable(root))?
                || fs::read_dir(path.parent().ok_or_else(|| error("definition parent"))?)?.count()
                    != 1
            {
                return Err(error("service definition changed"));
            }
        }
        Ok(())
    }
    pub(super) fn stage(&self, root: &Path, d: &Descriptor, bytes: Vec<u8>) -> Result<()> {
        let plan = Plan::prepared_service(bytes, &self.path(root), &self.digest)?;
        if !plan.install()? {
            return Err(error("unowned generation collision"));
        }
        if d.os != "windows" {
            let path = self.definition_path(root, d);
            let mut created = false;
            let result = (|| {
                create_private_dir(path.parent().ok_or_else(|| error("definition parent"))?)?;
                created = true;
                write_new(&path, &backend::definition(d, &self.executable(root))?)
            })();
            if let Err(cause) = result {
                if created {
                    fs::remove_dir(path.parent().ok_or_else(|| error("definition parent"))?)?;
                }
                plan.rollback()
                    .map_err(|e| error(format!("{cause}; staged cleanup failed: {e}")))?;
                return Err(cause);
            }
        }
        Ok(())
    }
    pub(super) fn grant(&self, root: &Path) -> Result<()> {
        #[cfg(windows)]
        {
            let sid = self
                .identity
                .as_ref()
                .and_then(|i| i.service_sid.as_deref())
                .ok_or_else(|| error("native service SID missing"))?;
            let plan = self.plan(root)?;
            if plan.verify_service(Some(sid)).is_err() {
                plan.verify_service(None)?;
                humanymous_runtime::native_security::storage::grant_service_access(
                    &self.executable(root),
                    sid,
                    humanymous_runtime::native_security::storage::ServiceRole::Executable,
                )?;
            }
        }
        #[cfg(not(windows))]
        let _ = root;
        Ok(())
    }
    pub(super) fn remove(&self, root: &Path, key: &[u8; 32]) -> Result<()> {
        self.verify(root, key)?;
        #[cfg(all(windows, feature = "runtime-service"))]
        if let Some(sid) = self
            .identity
            .as_ref()
            .and_then(|identity| identity.service_sid.as_deref())
        {
            crate::runtime_host::activation::revoke(root, &self.descriptor(key)?, sid)?;
        }
        #[cfg(windows)]
        if let Some(sid) = self
            .identity
            .as_ref()
            .and_then(|i| i.service_sid.as_deref())
        {
            humanymous_runtime::native_security::storage::revoke_service_access(
                &self.executable(root),
                sid,
                humanymous_runtime::native_security::storage::ServiceRole::Executable,
            )?;
        }
        let d = self.descriptor(key)?;
        if d.os != "windows" {
            let path = self.definition_path(root, &d);
            fs::remove_file(&path)?;
            fs::remove_dir(path.parent().ok_or_else(|| error("definition parent"))?)?;
        }
        self.plan(root)?.rollback()
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct State {
    pub(super) schema: String,
    pub(super) trust_key: String,
    pub(super) highest: u64,
    pub(super) current: Generation,
    pub(super) previous: Option<Generation>,
    pub(super) retired: Vec<Generation>,
}

pub(super) fn check_root(root: &Path) -> Result<()> {
    plain(root)?;
    private_parent(root)?;
    if root
        .to_str()
        .is_none_or(|s| s.chars().any(char::is_control))
    {
        return Err(error(
            "UTF-8 private root without control characters required",
        ));
    }
    if fs::symlink_metadata(root.join("pending.json")).is_ok() {
        return Err(error(
            "incomplete service transaction; manual recovery required",
        ));
    }
    Ok(())
}
pub(super) fn read_state(root: &Path, key: &[u8; 32]) -> Result<Option<State>> {
    let path = root.join("deployment.json");
    plain(&path)?;
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
        Ok(_) => (),
    }
    private_file(&path)?;
    let state: State = serde_json::from_slice(&read_bounded(&path, 512 * 1024)?)
        .map_err(|_| error("invalid deployment state"))?;
    if state.schema != "humanymous.deployment.v1"
        || state.trust_key != hex::encode(key)
        || state.retired.len() > 14
    {
        return Err(error("deployment state trust/budget mismatch"));
    }
    let name = state.current.descriptor(key)?.service_name;
    let mut sequences = std::collections::BTreeSet::new();
    for g in state
        .retired
        .iter()
        .chain(state.previous.iter())
        .chain(std::iter::once(&state.current))
    {
        let d = g.descriptor(key)?;
        if d.service_name != name || g.sequence > state.highest || !sequences.insert(g.sequence) {
            return Err(error("deployment generation replay/mismatch"));
        }
        g.verify(root, key)?;
    }
    Ok(Some(state))
}
pub(super) fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(error("regular managed file required"));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(error("managed file budget"));
    }
    Ok(bytes)
}
fn create_private_dir(path: &Path) -> Result<()> {
    let builder = fs::DirBuilder::new();
    #[cfg(unix)]
    let builder = {
        use std::os::unix::fs::DirBuilderExt;
        let mut b = builder;
        b.mode(0o700);
        b
    };
    builder.create(path)
}
pub(super) fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    let result = file.write_all(bytes).and_then(|_| file.sync_all());
    drop(file);
    if let Err(cause) = result {
        fs::remove_file(path)
            .map_err(|e| error(format!("{cause}; partial file cleanup failed: {e}")))?;
        return Err(cause);
    }
    Ok(())
}
pub(super) fn save_state(root: &Path, state: &State) -> Result<()> {
    let temporary = root.join("deployment.next");
    write_new(
        &temporary,
        &serde_json::to_vec(state).map_err(|_| error("deployment serialization"))?,
    )?;
    if let Err(cause) = fs::rename(&temporary, root.join("deployment.json")) {
        fs::remove_file(&temporary)?;
        return Err(cause);
    }
    Ok(())
}
pub(super) fn begin(root: &Path, operation: &str) -> Result<()> {
    write_new(
        &root.join("pending.json"),
        format!("{{\"schema\":\"humanymous.pending.v1\",\"operation\":\"{operation}\"}}\n")
            .as_bytes(),
    )
}
pub(super) fn finish(root: &Path) -> Result<()> {
    fs::remove_file(root.join("pending.json"))
}
pub(super) struct Lock(Option<PathBuf>);
impl Lock {
    pub(super) fn acquire(root: &Path) -> Result<Self> {
        check_root(root)?;
        let path = root.join("service.lock");
        write_new(&path, b"exclusive CLI service transaction\n")?;
        Ok(Self(Some(path)))
    }
    pub(super) fn release(mut self) -> Result<()> {
        if let Some(path) = self.0.as_ref() {
            fs::remove_file(path)?;
            self.0 = None;
        }
        Ok(())
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        if let Some(path) = self.0.as_ref() {
            let _ = fs::remove_file(path);
        }
    }
}

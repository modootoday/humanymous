//! Explicit, local artifact provisioning without service activation.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) mod permissions;
#[cfg(windows)]
use permissions::{InstallationObject, private_windows};
use permissions::{plain, private_parent};

/// A verified artifact and an isolated installation destination.
#[derive(Debug)]
pub struct Plan {
    destination: PathBuf,
    bytes: Vec<u8>,
    receipt: String,
    artifact_name: &'static str,
}

fn err(message: impl Into<String>) -> std::io::Error {
    std::io::Error::other(message.into())
}

impl Plan {
    /// Read-only prerequisite validation. The acknowledgement is not runtime verification.
    pub fn inspect(
        artifact: &Path,
        destination: &Path,
        expected: &str,
        local_verified: bool,
    ) -> std::io::Result<Self> {
        if !matches!(std::env::consts::OS, "windows" | "linux" | "macos") {
            return Err(err("unsupported operating system"));
        }
        if !local_verified {
            return Err(err(
                "local management binding and IPC ACL verification acknowledgement required",
            ));
        }
        plain(artifact)?;
        plain(destination)?;
        let parent = destination
            .parent()
            .ok_or_else(|| err("destination parent required"))?;
        private_parent(parent)?;
        if !fs::symlink_metadata(artifact)?.is_file() {
            return Err(err("regular artifact required"));
        }
        let bytes = fs::read(artifact)?;
        if bytes.is_empty() {
            return Err(err("empty artifact"));
        }
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if expected.len() != 64 || digest != expected {
            return Err(err("artifact SHA-256 mismatch"));
        }
        let plan = Self::prepared(bytes, destination, expected)?;
        if destination.exists() {
            plan.verify()?;
        }
        Ok(plan)
    }

    pub(crate) fn prepared(
        bytes: Vec<u8>,
        destination: &Path,
        expected: &str,
    ) -> std::io::Result<Self> {
        Self::prepared_named(bytes, destination, expected, "artifact", true)
    }

    pub(crate) fn prepared_service(
        bytes: Vec<u8>,
        destination: &Path,
        expected: &str,
    ) -> std::io::Result<Self> {
        Self::prepared_named(bytes, destination, expected, service_artifact_name(), false)
    }

    fn prepared_named(
        bytes: Vec<u8>,
        destination: &Path,
        expected: &str,
        artifact_name: &'static str,
        local_verified: bool,
    ) -> std::io::Result<Self> {
        plain(destination)?;
        private_parent(destination.parent().ok_or_else(|| err("parent required"))?)?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if digest != expected || bytes.is_empty() {
            return Err(err("artifact SHA-256 mismatch"));
        }
        Ok(Self {
            artifact_name,
            destination: destination.to_owned(),
            bytes,
            receipt: format!(
                "humanymous-provision-v1\nsha256={digest}\nlocal-management={}\nservice=none\n",
                if local_verified {
                    "operator-verified"
                } else {
                    "not-runtime-qualified"
                }
            ),
        })
    }

    fn verify(&self) -> std::io::Result<()> {
        self.verify_service(None)
    }

    pub(crate) fn verify_service(&self, service_sid: Option<&str>) -> std::io::Result<()> {
        #[cfg(not(windows))]
        if service_sid.is_some() {
            return Err(err("Windows service SID on non-Windows target"));
        }
        plain(&self.destination)?;
        #[cfg(windows)]
        private_windows(&self.destination, InstallationObject::Directory)?;
        #[cfg(unix)]
        private_parent(&self.destination)?;
        let mut names = fs::read_dir(&self.destination)?
            .map(|e| e.map(|e| e.file_name()))
            .collect::<std::io::Result<Vec<_>>>()?;
        names.sort();
        if names
            != [
                std::ffi::OsString::from(self.artifact_name),
                std::ffi::OsString::from("receipt"),
            ]
        {
            return Err(err("unmanaged or partial destination"));
        }
        for (name, bytes) in [
            (self.artifact_name, self.bytes.as_slice()),
            ("receipt", self.receipt.as_bytes()),
        ] {
            let path = self.destination.join(name);
            plain(&path)?;
            #[cfg(windows)]
            if name == self.artifact_name && service_sid.is_some() {
                use std::os::windows::fs::OpenOptionsExt;
                let file = fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(0x00200000)
                    .open(&path)?;
                let owner = humanymous_runtime::native_security::process::current_identity()?;
                humanymous_runtime::native_security::storage::verify_service_access(
                    &file,
                    owner.sid(),
                    service_sid.unwrap_or(""),
                    humanymous_runtime::native_security::storage::ServiceRole::Executable,
                )?;
            } else {
                private_windows(&path, InstallationObject::File)?;
            }
            if !fs::symlink_metadata(&path)?.is_file() || fs::read(path)? != bytes {
                return Err(err("managed file changed"));
            }
        }
        Ok(())
    }

    /// Exact proposed changes; calling this does not write or lock anything.
    pub fn describe(&self) -> String {
        format!(
            "provision {}: artifact ({} bytes), receipt; service=none; activation=none",
            self.destination.display(),
            self.bytes.len()
        )
    }

    /// Create a new installation or verify an identical existing installation.
    pub fn install(&self) -> std::io::Result<bool> {
        self.install_with(|file, bytes| {
            file.write_all(bytes)?;
            file.sync_all()
        })
    }

    fn install_with(
        &self,
        mut write: impl FnMut(&mut fs::File, &[u8]) -> std::io::Result<()>,
    ) -> std::io::Result<bool> {
        private_parent(
            self.destination
                .parent()
                .ok_or_else(|| err("parent required"))?,
        )?;
        plain(&self.destination)?;
        let directory = fs::DirBuilder::new();
        #[cfg(unix)]
        let directory = {
            use std::os::unix::fs::DirBuilderExt;
            let mut directory = directory;
            directory.mode(0o700);
            directory
        };
        match directory.create(&self.destination) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                self.verify()?;
                return Ok(false);
            }
            Err(e) => return Err(e),
        }
        let mut created = Vec::new();
        let result = (|| {
            for (name, bytes) in [
                (self.artifact_name, self.bytes.as_slice()),
                ("receipt", self.receipt.as_bytes()),
            ] {
                let path = self.destination.join(name);
                let mut options = fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(if name == self.artifact_name {
                        0o700
                    } else {
                        0o600
                    });
                }
                let mut file = options.open(&path)?;
                created.push(path);
                write(&mut file, bytes)?;
            }
            self.verify()
        })();
        if let Err(cause) = result {
            for path in created.into_iter().rev() {
                fs::remove_file(path).map_err(|e| err(format!("{cause}; cleanup failed: {e}")))?;
            }
            fs::remove_dir(&self.destination)
                .map_err(|e| err(format!("{cause}; cleanup failed: {e}")))?;
            return Err(cause);
        }
        Ok(true)
    }

    /// Remove only an unmodified installation; requires exclusive administrative access.
    pub fn rollback(&self) -> std::io::Result<()> {
        private_parent(
            self.destination
                .parent()
                .ok_or_else(|| err("parent required"))?,
        )?;
        self.verify()?;
        fs::remove_file(self.destination.join("receipt"))?;
        fs::remove_file(self.destination.join(self.artifact_name))?;
        fs::remove_dir(&self.destination)
    }
}

pub(crate) fn service_artifact_name() -> &'static str {
    if cfg!(windows) {
        "artifact.exe"
    } else {
        "artifact"
    }
}

#[cfg(test)]
pub(crate) mod tests;

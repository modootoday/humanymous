//! Explicit, local artifact provisioning without service activation.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

/// A verified artifact and an isolated installation destination.
#[derive(Debug)]
pub struct Plan {
    destination: PathBuf,
    bytes: Vec<u8>,
    receipt: String,
}

fn err(message: impl Into<String>) -> std::io::Error {
    std::io::Error::other(message.into())
}

fn plain(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let drive = match path.components().next() {
            Some(Component::Prefix(p)) => match p.kind() {
                std::path::Prefix::Disk(letter) => format!("{}:\\", char::from(letter)),
                _ => return Err(err("only local drive-letter paths are supported")),
            },
            _ => return Err(err("absolute local drive-letter path required")),
        };
        let status = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; if([IO.DriveInfo]::new($env:HUMANYMOUS_DRIVE_CHECK).DriveType -ne 'Fixed'){exit 1}"])
            .env("HUMANYMOUS_DRIVE_CHECK", drive).status()?;
        if !status.success() {
            return Err(err("local fixed drive required"));
        }
    }
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(err("absolute path without traversal required"));
    }
    let mut current = PathBuf::new();
    for part in path.components() {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) => {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err(err("reparse point rejected"));
                    }
                }
                if meta.file_type().is_symlink() {
                    return Err(err("symlink rejected"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && current == path => (),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn private_parent(path: &Path) -> std::io::Result<()> {
    if !fs::metadata(path)?.is_dir() {
        return Err(err("parent must exist"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let uid = std::process::Command::new("id").arg("-u").output()?;
        let uid: u32 = String::from_utf8_lossy(&uid.stdout)
            .trim()
            .parse()
            .map_err(|_| err("cannot establish process owner"))?;
        let meta = fs::metadata(path)?;
        if !uid_is_allowed(uid) || meta.uid() != uid || meta.mode() & 0o077 != 0 {
            return Err(err("private non-root owned parent required"));
        }
    }
    #[cfg(windows)]
    private_windows(path, true)?;
    #[cfg(not(any(unix, windows)))]
    return Err(err("unsupported OS permission boundary"));
    Ok(())
}

#[cfg(unix)]
fn uid_is_allowed(uid: u32) -> bool {
    uid != 0
}

#[cfg(windows)]
fn private_windows(path: &Path, parent: bool) -> std::io::Result<()> {
    let script = r#"
$ErrorActionPreference='Stop'
$u=[Security.Principal.WindowsIdentity]::GetCurrent()
$p=[Security.Principal.WindowsPrincipal]::new($u)
if($p.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){exit 3}
if([IO.Directory]::Exists($env:HUMANYMOUS_ACL_CHECK)){$a=([IO.DirectoryInfo]::new($env:HUMANYMOUS_ACL_CHECK)).GetAccessControl()}else{$a=([IO.FileInfo]::new($env:HUMANYMOUS_ACL_CHECK)).GetAccessControl()}
if($env:HUMANYMOUS_ACL_PARENT -eq 'true' -and !$a.AreAccessRulesProtected){exit 4}
$allowed=@($u.User.Value,'S-1-5-18','S-1-5-32-544')
if($a.GetOwner([Security.Principal.SecurityIdentifier]).Value -ne $u.User.Value){exit 5}
$ownerInherits=$false
foreach($r in $a.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])){
  if($r.AccessControlType -eq 'Allow'){
    if($r.IdentityReference.Value -notin $allowed){exit 6}
    if($r.IdentityReference.Value -eq $u.User.Value -and
       ($r.FileSystemRights -band 2032127) -eq 2032127 -and
       ($env:HUMANYMOUS_ACL_PARENT -ne 'true' -or (($r.InheritanceFlags -band 3) -eq 3 -and $r.PropagationFlags -eq 0))){$ownerInherits=$true}
  }
}
if(!$ownerInherits){exit 7}
"#;
    let status = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("HUMANYMOUS_ACL_CHECK", path)
        .env(
            "HUMANYMOUS_ACL_PARENT",
            if parent { "true" } else { "false" },
        )
        .status()?;
    if !status.success() {
        return Err(err(
            "protected user-owned DACL and non-elevated token required",
        ));
    }
    Ok(())
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
        let plan = Self {
            destination: destination.to_owned(),
            bytes,
            receipt: format!(
                "humanymous-provision-v1\nsha256={digest}\nlocal-management=operator-verified\nservice=none\n"
            ),
        };
        if destination.exists() {
            plan.verify()?;
        }
        Ok(plan)
    }

    fn verify(&self) -> std::io::Result<()> {
        plain(&self.destination)?;
        #[cfg(windows)]
        private_windows(&self.destination, false)?;
        #[cfg(unix)]
        private_parent(&self.destination)?;
        let mut names = fs::read_dir(&self.destination)?
            .map(|e| e.map(|e| e.file_name()))
            .collect::<std::io::Result<Vec<_>>>()?;
        names.sort();
        if names
            != [
                std::ffi::OsString::from("artifact"),
                std::ffi::OsString::from("receipt"),
            ]
        {
            return Err(err("unmanaged or partial destination"));
        }
        for (name, bytes) in [
            ("artifact", self.bytes.as_slice()),
            ("receipt", self.receipt.as_bytes()),
        ] {
            let path = self.destination.join(name);
            plain(&path)?;
            #[cfg(windows)]
            private_windows(&path, false)?;
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
                ("artifact", self.bytes.as_slice()),
                ("receipt", self.receipt.as_bytes()),
            ] {
                let path = self.destination.join(name);
                let mut options = fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(if name == "artifact" { 0o700 } else { 0o600 });
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
        fs::remove_file(self.destination.join("artifact"))?;
        fs::remove_dir(&self.destination)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static NEXT_FIXTURE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            // macOS temp paths can traverse /var -> /private/var. Resolve the
            // trusted OS fixture root, keeping product symlink rejection intact.
            #[cfg(unix)]
            let root = fs::canonicalize(std::env::temp_dir()).expect("native temporary root");
            #[cfg(not(unix))]
            let root = std::env::temp_dir();
            let path = root.join(format!(
                "humanymous-test-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("fixture");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("mode");
            }
            #[cfg(windows)]
            {
                let status = std::process::Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", "$a=[Security.AccessControl.DirectorySecurity]::new(); $u=[Security.Principal.WindowsIdentity]::GetCurrent().User; $a.SetOwner($u); $a.SetAccessRuleProtection($true,$false); $a.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($u,'FullControl','ContainerInherit,ObjectInherit','None','Allow')); ([IO.DirectoryInfo]::new($env:HUMANYMOUS_FIXTURE)).SetAccessControl($a)"]).env("HUMANYMOUS_FIXTURE", &path).status().expect("ACL");
                assert!(status.success());
            }
            fs::write(path.join("source"), b"verified fixture artifact").expect("source");
            Self(path)
        }
        fn plan(&self) -> Plan {
            Plan::inspect(
                &self.0.join("source"),
                &self.0.join("installed"),
                &format!("{:x}", Sha256::digest(b"verified fixture artifact")),
                true,
            )
            .expect("plan")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn dry_repeat_and_rollback() {
        let f = Fixture::new();
        let p = f.plan();
        assert!(!f.0.join("installed").exists());
        assert!(p.describe().contains("service=none"));
        assert!(p.install().expect("install"));
        assert!(!f.plan().install().expect("repeat"));
        p.rollback().expect("rollback");
        assert!(!f.0.join("installed").exists());
        assert!(f.0.join("source").exists());
    }
    #[test]
    fn fail_closed_input_and_collision() {
        let f = Fixture::new();
        assert!(
            Plan::inspect(
                &f.0.join("source"),
                &f.0.join("installed"),
                &"0".repeat(64),
                true
            )
            .is_err()
        );
        assert!(Plan::inspect(&f.0.join("source"), &f.0.join("installed"), "", false).is_err());
        assert!(plain(Path::new("relative")).is_err());
        assert!(plain(&f.0.join("../escape")).is_err());
        let p = f.plan();
        fs::write(&p.destination, b"existing").expect("collision");
        assert!(p.install().is_err());
        assert_eq!(fs::read(&p.destination).expect("read"), b"existing");
    }
    #[test]
    fn changed_or_extra_files_block_rollback() {
        let f = Fixture::new();
        let p = f.plan();
        p.install().expect("install");
        fs::write(p.destination.join("extra"), b"user").expect("extra");
        assert!(p.rollback().is_err());
        fs::remove_file(p.destination.join("extra")).expect("remove extra");
        fs::write(p.destination.join("artifact"), b"changed").expect("changed");
        assert!(p.rollback().is_err());
        assert!(f.plan_result().is_err());
    }
    impl Fixture {
        fn plan_result(&self) -> std::io::Result<Plan> {
            Plan::inspect(
                &self.0.join("source"),
                &self.0.join("installed"),
                &format!("{:x}", Sha256::digest(b"verified fixture artifact")),
                true,
            )
        }
    }
    #[test]
    fn partial_write_failure_cleans_owned_files() {
        let f = Fixture::new();
        let p = f.plan();
        let mut calls = 0;
        assert!(
            p.install_with(|file, bytes| {
                calls += 1;
                file.write_all(&bytes[..2])?;
                if calls == 2 {
                    Err(err("injected disk failure"))
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
        assert!(!p.destination.exists());
        assert!(f.0.join("source").exists());
    }
    #[test]
    fn linked_artifact_is_rejected() {
        let f = Fixture::new();
        let link = f.0.join("link");
        #[cfg(unix)]
        std::os::unix::fs::symlink(f.0.join("source"), &link).expect("symlink");
        #[cfg(windows)]
        {
            // Junctions do not need Developer Mode or symbolic-link privilege.
            let status = std::process::Command::new("cmd.exe")
                .args(["/c", "mklink", "/J"])
                .arg(&link)
                .arg(&f.0)
                .output()
                .expect("junction");
            assert!(status.status.success());
        }
        assert!(plain(&link).is_err());
        #[cfg(windows)]
        fs::remove_dir(link).expect("remove junction before fixture cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn unsafe_acl_and_unc_fail_closed() {
        let f = Fixture::new();
        assert!(plain(Path::new(r"\\server\share\artifact")).is_err());
        let p = f.plan();
        p.install().expect("install");
        let status = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; $f=[IO.FileInfo]::new($env:HUMANYMOUS_FIXTURE_FILE); $a=$f.GetAccessControl(); $a.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new([Security.Principal.SecurityIdentifier]::new('S-1-1-0'),'Read','Allow')); $f.SetAccessControl($a)"])
            .env("HUMANYMOUS_FIXTURE_FILE", p.destination.join("artifact")).status().expect("ACL mutation");
        assert!(status.success());
        assert!(p.install().is_err());
        assert!(p.rollback().is_err());
    }
}

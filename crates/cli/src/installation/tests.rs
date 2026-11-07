use super::*;
#[cfg(unix)]
#[test]
fn doctor_refuses_parent_without_owner_write_permission() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o500)).unwrap();
    let refused = fixture.plan_result().is_err();
    fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(refused);
    assert!(!fixture.0.join("installed").exists());
}
static NEXT_FIXTURE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub(crate) struct Fixture(pub(crate) PathBuf);
impl Fixture {
    pub(crate) fn new() -> Self {
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
            set_acl(&path, "");
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
        junction::create(&f.0, &link).expect("junction");
    }
    assert!(plain(&link).is_err());
    #[cfg(windows)]
    junction::delete(&link).expect("remove junction before fixture cleanup");
}

#[cfg(windows)]
#[test]
fn unsafe_acl_and_unc_fail_closed() {
    let f = Fixture::new();
    assert!(plain(Path::new(r"\\server\share\artifact")).is_err());
    let p = f.plan();
    p.install().expect("install");
    set_acl(&p.destination.join("artifact"), "(A;;FR;;;WD)");
    assert!(p.install().is_err());
    assert!(p.rollback().is_err());
}

#[cfg(windows)]
fn set_acl(path: &Path, extra_ace: &str) {
    use windows_permissions::{
        LocalBox, SecurityDescriptor, Sid,
        constants::{SeObjectType, SecurityInformation},
        wrappers,
    };
    let identity =
        humanymous_runtime::native_security::process::current_identity().expect("native identity");
    let owner: LocalBox<Sid> = identity.sid().parse().unwrap();
    let sd: LocalBox<SecurityDescriptor> =
        format!("D:P(A;OICI;FA;;;{}){extra_ace}", identity.sid())
            .parse()
            .unwrap();
    wrappers::SetNamedSecurityInfo(
        path.as_os_str(),
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        Some(&owner),
        None,
        sd.dacl(),
        None,
    )
    .unwrap();
}

//! Source-bound process pilots; full native registration is explicitly opted in.
#![cfg(feature = "service-pilot")]
use ed25519_dalek::{Signer, SigningKey};
use humanymous_cli::{
    release::{Descriptor, SignedRelease, signing_bytes},
    service::{self, ServicePlan},
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Fixture {
    root: PathBuf,
    descriptor: Descriptor,
    release: SignedRelease,
    key: [u8; 32],
    source: PathBuf,
    manifest: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        #[cfg(unix)]
        let temporary = fs::canonicalize(std::env::temp_dir()).unwrap();
        #[cfg(windows)]
        let temporary = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!("humanymous-lab-p{}-{stamp}", std::process::id());
        let root = temporary.join(&name);
        fs::create_dir(&root).unwrap();
        private(&root);
        let source = root.join("source.exe");
        fs::copy(env!("CARGO_BIN_EXE_humanymous-service-fixture"), &source).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let readiness = listener.local_addr().unwrap();
        drop(listener);
        let response = "fixture-ready-v1".to_string();
        let mut arguments = vec![name.clone(), readiness.to_string(), response.clone()];
        if cfg!(windows) {
            arguments.push("--scm".into());
        }
        let descriptor = Descriptor {
            schema: "humanymous.release.v1".into(),
            version: "fixture-v1".into(),
            sequence: 1,
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            artifact_sha256: format!("{:x}", Sha256::digest(fs::read(&source).unwrap())),
            service_name: name,
            arguments,
            readiness,
            ready_response: response,
        };
        let (release, key) = sign(&descriptor);
        let manifest = root.join("release.json");
        fs::write(&manifest, &release.document).unwrap();
        Self {
            root,
            descriptor,
            release,
            key,
            source,
            manifest,
        }
    }
    fn plan(&self) -> ServicePlan {
        ServicePlan::inspect(&self.root, &self.source, self.release.clone(), self.key).unwrap()
    }
    fn command(&self, name: &str) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_humanymous"));
        c.arg(name).args([
            "--destination",
            self.root.to_str().unwrap(),
            "--trust-key",
            &hex::encode(self.key),
        ]);
        if ["service-plan", "service-install", "service-upgrade"].contains(&name) {
            c.args([
                "--manifest",
                self.manifest.to_str().unwrap(),
                "--signature",
                &self.release.signature,
                "--artifact",
                self.source.to_str().unwrap(),
            ]);
        }
        c
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !self.root.join("deployment.json").exists() && !self.root.join("pending.json").exists() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
fn sign(d: &Descriptor) -> (SignedRelease, [u8; 32]) {
    let key = SigningKey::from_bytes(&[81; 32]);
    let document = serde_json::to_string(d).unwrap();
    let signature = hex::encode(key.sign(&signing_bytes(document.as_bytes())).to_bytes());
    (
        SignedRelease {
            document,
            signature,
        },
        key.verifying_key().to_bytes(),
    )
}
#[cfg(unix)]
fn private(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}
#[cfg(windows)]
fn private(path: &Path) {
    use windows_permissions::{
        LocalBox, SecurityDescriptor, Sid,
        constants::{SeObjectType, SecurityInformation},
        wrappers,
    };
    let owner = humanymous_runtime::native_security::process::current_identity().unwrap();
    assert!(owner.installer_allowed(), "ordinary actor is mandatory");
    let sid: LocalBox<Sid> = owner.sid().parse().unwrap();
    let descriptor: LocalBox<SecurityDescriptor> =
        format!("D:P(A;OICI;FA;;;{})", owner.sid()).parse().unwrap();
    wrappers::SetNamedSecurityInfo(
        path.as_os_str(),
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        Some(&sid),
        None,
        descriptor.dacl(),
        None,
    )
    .unwrap();
}
fn snapshot(root: &Path) -> Vec<String> {
    let mut paths = vec![root.to_path_buf()];
    let mut result = Vec::new();
    while let Some(path) = paths.pop() {
        let meta = fs::symlink_metadata(&path).unwrap();
        let mut row = format!(
            "{}:{}:{:?}",
            path.strip_prefix(root).unwrap().display(),
            meta.len(),
            meta.modified().unwrap()
        );
        if meta.is_dir() {
            paths.extend(fs::read_dir(&path).unwrap().map(|e| e.unwrap().path()));
        } else {
            row.push_str(&format!("{:x}", Sha256::digest(fs::read(&path).unwrap())));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_permissions::{
                constants::{SeObjectType, SecurityInformation},
                wrappers,
            };
            let file = fs::OpenOptions::new()
                .read(true)
                .custom_flags(0x02200000)
                .open(&path)
                .unwrap();
            let info = SecurityInformation::Owner | SecurityInformation::Dacl;
            let descriptor =
                wrappers::GetSecurityInfo(&file, SeObjectType::SE_FILE_OBJECT, info).unwrap();
            let text =
                wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(&descriptor, info)
                    .unwrap();
            row.push_str(text.to_str().unwrap());
        }
        result.push(row);
    }
    result.sort();
    result
}
struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn probe(address: SocketAddr, response: &str) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(100)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
    if stream.write_all(b"HUMANYMOUS-READY/1\n").is_err() {
        return false;
    }
    let mut bytes = Vec::new();
    stream.take(513).read_to_end(&mut bytes).is_ok() && bytes == response.as_bytes()
}

#[test]
fn signed_plan_is_read_only_and_fixture_is_actual_process() {
    let fixture = Fixture::new();
    let before = snapshot(&fixture.root);
    let output = fixture.command("service-plan").output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = fixture
        .command("service-install")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(before, snapshot(&fixture.root));
    assert!(
        !fixture
            .command("service-install")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(before, snapshot(&fixture.root));
    let _child = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_humanymous-service-fixture"))
            .args([
                &fixture.descriptor.service_name,
                &fixture.descriptor.readiness.to_string(),
                &fixture.descriptor.ready_response,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let until = Instant::now() + Duration::from_secs(3);
    while !probe(
        fixture.descriptor.readiness,
        &fixture.descriptor.ready_response,
    ) {
        assert!(Instant::now() < until, "actual fixture readiness deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
    println!(
        "{}",
        serde_json::json!({"schema":"humanymous.cli-service-pilot.v1","case":"signed-plan-and-native-process","os":std::env::consts::OS,"arch":std::env::consts::ARCH,"dry_run_unchanged":true,"actual_process_ready":true,"native_service_registered":false,"product_qualified":false,"artifact_sha256":fixture.descriptor.artifact_sha256})
    );
}

#[cfg(windows)]
#[test]
fn scm_ordinary_actor_refusal_cleans_staging() {
    let fixture = Fixture::new();
    let before = fs::read_dir(&fixture.root).unwrap().count();
    let output = fixture
        .command("service-install")
        .arg("--activate")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "ordinary actor unexpectedly has SCM create authority; use disposable opt-in pilot instead"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("SCM"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_dir(&fixture.root).unwrap().count(), before);
    assert!(!fixture.root.join("pending.json").exists());
    println!(
        "{}",
        serde_json::json!({"schema":"humanymous.cli-service-pilot.v1","case":"ordinary-actor-SCM-refused","status":"observed-refused","staging_cleanup_verified":true,"native_service_registered":false,"product_qualified":false})
    );
}

#[test]
#[ignore = "requires explicit disposable native user-service actor and manager"]
fn native_service_install_upgrade_rollback_and_remove() {
    assert_eq!(
        std::env::var("HUMANYMOUS_DISPOSABLE_SERVICE_ACTOR").as_deref(),
        Ok("1"),
        "explicit disposable actor contract required"
    );
    let mut fixture = Fixture::new();
    assert!(fixture.plan().install(true).unwrap());
    assert!(!fixture.plan().install(true).unwrap());
    let report = service::doctor(&fixture.root, fixture.key).unwrap();
    assert!(report.registered && report.running && report.diagnostic_ready);
    assert!(!report.product_qualified);
    fixture.descriptor.sequence = 2;
    fixture.descriptor.version = "fixture-v2".into();
    fixture.descriptor.ready_response = "fixture-ready-v2".into();
    fixture.descriptor.arguments[2] = "fixture-ready-v2".into();
    (fixture.release, fixture.key) = sign(&fixture.descriptor);
    fixture.plan().upgrade(true).unwrap();
    assert_eq!(
        service::doctor(&fixture.root, fixture.key)
            .unwrap()
            .sequence,
        2
    );
    service::rollback(&fixture.root, fixture.key, true).unwrap();
    let report = service::doctor(&fixture.root, fixture.key).unwrap();
    assert_eq!(report.sequence, 1);
    assert_eq!(report.highest_sequence, 2);
    assert!(fixture.plan().upgrade(true).is_err());
    service::remove(&fixture.root, fixture.key, true).unwrap();
    assert!(!fixture.root.join("deployment.json").exists());
    assert_eq!(fs::read_dir(&fixture.root).unwrap().count(), 2);
    println!(
        "{}",
        serde_json::json!({"schema":"humanymous.cli-service-pilot.v1","case":"native-service-lifecycle","os":std::env::consts::OS,"arch":std::env::consts::ARCH,"native_service_registered":true,"activated":true,"readiness_verified":true,"upgrade_verified":true,"rollback_verified":true,"cleanup_verified":true,"product_qualified":false,"artifact_sha256":fixture.descriptor.artifact_sha256})
    );
}

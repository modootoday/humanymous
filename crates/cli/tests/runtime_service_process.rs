//! Actual SCM/OS-store/ServiceHost qualification; disposable hosted actor only.
#![cfg(all(windows, feature = "runtime-service-pilot"))]
use ed25519_dalek::{Signer, SigningKey};
use humanymous_authority::credentials::Credential;
use humanymous_cli::{
    release::{Descriptor, SignedRelease, signing_bytes},
    service::{self, ServicePlan},
};
use humanymous_native_security::process::current_identity;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use windows_service::{
    service::{ServiceAccess, ServiceState},
    service_manager::{ServiceManager, ServiceManagerAccess},
};

fn private(path: &Path) {
    use windows_permissions::{
        LocalBox, SecurityDescriptor, Sid,
        constants::{SeObjectType, SecurityInformation},
        wrappers,
    };
    let identity = current_identity().unwrap();
    assert!(identity.installer_allowed());
    let sid: LocalBox<Sid> = identity.sid().parse().unwrap();
    let descriptor: LocalBox<SecurityDescriptor> = format!("D:P(A;OICI;FA;;;{})", identity.sid())
        .parse()
        .unwrap();
    let mut file = fs::OpenOptions::new()
        .access_mode(0xe0000)
        .custom_flags(0x02000000)
        .open(path)
        .unwrap();
    wrappers::SetSecurityInfo(
        &mut file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        Some(&sid),
        None,
        descriptor.dacl(),
        None,
    )
    .unwrap();
}
use std::os::windows::fs::OpenOptionsExt;
fn nonce() -> String {
    Credential::generate().unwrap().expose_for_provisioning()[..32].to_owned()
}
fn address() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}
fn digest(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
fn signed(descriptor: &Descriptor) -> (SignedRelease, [u8; 32]) {
    let key = SigningKey::from_bytes(&[81; 32]);
    let document = serde_json::to_string(descriptor).unwrap();
    let signature = hex::encode(key.sign(&signing_bytes(document.as_bytes())).to_bytes());
    (
        SignedRelease {
            document,
            signature,
        },
        key.verifying_key().to_bytes(),
    )
}
fn manager() -> ServiceManager {
    ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT).unwrap()
}
struct Fixture {
    root: PathBuf,
    name: String,
    source: PathBuf,
    release: SignedRelease,
    key: [u8; 32],
    receipt: PathBuf,
    metadata: Value,
    runtime: SocketAddr,
    origin: TcpListener,
}
impl Fixture {
    fn new(run: u64, source_sha: &str, missing: bool) -> Self {
        let correlation = nonce();
        let name = format!("humanymous-lab-runtime-{}", &correlation[..16]);
        let root = std::env::temp_dir().join(&name);
        fs::create_dir(&root).unwrap();
        private(&root);
        let state = root.join("state");
        fs::create_dir(&state).unwrap();
        private(&state);
        let source = root.join("source.exe");
        fs::copy(env!("CARGO_BIN_EXE_humanymous"), &source).unwrap();
        let hash = digest(&source);
        let runtime = address();
        let diagnostic = address();
        let origin_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        origin_listener.set_nonblocking(true).unwrap();
        let origin = origin_listener.local_addr().unwrap();
        assert_ne!(runtime, diagnostic);
        assert_ne!(runtime, origin);
        assert_ne!(diagnostic, origin);
        let plan_path = root.join("launch");
        let receipt = state.join("receipt");
        let metadata = json!({"schema":"humanymous.runtime-host.v1","service_name":name,"installer_owner_sid":current_identity().unwrap().sid(),"installation":"runtime-pilot","generation":correlation,"executable_sha256":hash,"configuration":state.join("private").join("configuration"),"configuration_sha256":"0".repeat(64),"unseal_credential_id":"pilot-bootstrap","observer":null,"diagnostic":diagnostic,"ready_response":"runtime-pilot-ready","startup_ms":15000,"stop":{"drain_ms":2000,"supervisor_reserve_ms":3000,"manager_stop_ms":6000},"mode":"native","pilot":{"run_id":run,"source_sha":source_sha,"nonce":correlation,"parent":state,"child":"private","receipt":"receipt","origin":origin,"listen":runtime,"missing_credential":missing}});
        fs::write(&plan_path, serde_json::to_vec(&metadata).unwrap()).unwrap();
        let descriptor = Descriptor {
            schema: "humanymous.release.v1".into(),
            version: "runtime-pilot-v1".into(),
            sequence: 1,
            os: "windows".into(),
            arch: std::env::consts::ARCH.into(),
            artifact_sha256: hash,
            service_name: name.clone(),
            arguments: vec![
                "runtime-host".into(),
                "--plan".into(),
                plan_path.to_str().unwrap().into(),
                "--plan-sha256".into(),
                digest(&plan_path),
            ],
            readiness: diagnostic,
            ready_response: "runtime-pilot-ready".into(),
        };
        let (release, key) = signed(&descriptor);
        release.verify(&key).unwrap();
        Self {
            root,
            name,
            source,
            release,
            key,
            receipt,
            metadata,
            runtime,
            origin: origin_listener,
        }
    }
    fn plan(&self) -> ServicePlan {
        ServicePlan::inspect(&self.root, &self.source, self.release.clone(), self.key).unwrap()
    }
    fn read_receipt(&self, pid: u32) -> Value {
        let path = self.receipt.with_file_name(format!(
            "{}-{pid}",
            self.receipt.file_name().unwrap().to_str().unwrap()
        ));
        let file = fs::File::open(path).expect("missing receipt is unknown/failure, never success");
        assert!(file.metadata().unwrap().len() <= 8192);
        let mut bytes = Vec::new();
        file.take(8193).read_to_end(&mut bytes).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }
    fn bind_receipt(&self, receipt: &Value, pid: u32) {
        assert_eq!(
            receipt["schema"],
            "humanymous.runtime-service-pilot.receipt.v1"
        );
        for field in ["service", "generation", "executable_sha256"] {
            let key = if field == "service" {
                "service_name"
            } else {
                field
            };
            assert_eq!(receipt[field], self.metadata[key]);
        }
        for field in ["run_id", "source_sha", "nonce"] {
            assert_eq!(receipt[field], self.metadata["pilot"][field]);
        }
        assert_eq!(receipt["service_pid"], pid);
        assert_eq!(receipt["prepared"], true);
        assert_eq!(receipt["credential_cleanup"], true);
        assert_eq!(receipt["private_state_cleanup"], true);
        assert_eq!(receipt["error_code"], "none");
        assert_eq!(receipt["product_qualified"], false);
        assert!(
            matches!(fs::symlink_metadata(Path::new(self.metadata["configuration"].as_str().unwrap()).parent().unwrap()),Err(error) if error.kind()==std::io::ErrorKind::NotFound)
        );
    }
    fn next(&self) -> Self {
        let mut metadata = self.metadata.clone();
        let correlation = nonce();
        let runtime = address();
        let diagnostic = address();
        let origin = TcpListener::bind("127.0.0.1:0").unwrap();
        origin.set_nonblocking(true).unwrap();
        let origin_address = origin.local_addr().unwrap();
        assert_ne!(runtime, diagnostic);
        assert_ne!(runtime, origin_address);
        assert_ne!(diagnostic, origin_address);
        metadata["generation"] = json!(correlation);
        metadata["diagnostic"] = json!(diagnostic);
        metadata["ready_response"] = json!("runtime-pilot-ready-v2");
        metadata["configuration"] = json!(
            self.root
                .join("state")
                .join("private-two")
                .join("configuration")
        );
        metadata["pilot"]["nonce"] = json!(correlation);
        metadata["pilot"]["child"] = json!("private-two");
        metadata["pilot"]["receipt"] = json!("receipt-two");
        metadata["pilot"]["listen"] = json!(runtime);
        metadata["pilot"]["origin"] = json!(origin_address);
        let path = self.root.join("launch-two");
        fs::write(&path, serde_json::to_vec(&metadata).unwrap()).unwrap();
        let mut descriptor = self.release.verify(&self.key).unwrap();
        descriptor.sequence = 2;
        descriptor.version = "runtime-pilot-v2".into();
        descriptor.readiness = diagnostic;
        descriptor.ready_response = "runtime-pilot-ready-v2".into();
        descriptor.arguments[2] = path.to_str().unwrap().into();
        descriptor.arguments[4] = digest(&path);
        let (release, key) = signed(&descriptor);
        assert_eq!(key, self.key);
        Self {
            root: self.root.clone(),
            name: self.name.clone(),
            source: self.source.clone(),
            release,
            key,
            receipt: self.root.join("state").join("receipt-two"),
            metadata,
            runtime,
            origin,
        }
    }
    fn running(&self, sequence: u64, highest: u64) -> u32 {
        let report = service::doctor(&self.root, self.key).unwrap();
        assert!(report.registered && report.running && report.diagnostic_ready);
        assert_eq!(report.sequence, sequence);
        assert_eq!(report.highest_sequence, highest);
        let service = manager()
            .open_service(&self.name, ServiceAccess::QUERY_STATUS)
            .unwrap();
        let status = service.query_status().unwrap();
        assert_eq!(status.current_state, ServiceState::Running);
        let pid = status.process_id.expect("actual SCM PID required");
        assert!(pid > 0);
        pid
    }
    fn reject_missing_integrity(&self) {
        let mut runtime = TcpStream::connect(self.runtime).unwrap();
        runtime
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        runtime
            .write_all(b"GET /protected HTTP/1.1\r\nHost: pilot.local\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut response = Vec::new();
        runtime.take(8192).read_to_end(&mut response).unwrap();
        assert!(!response.is_empty());
        assert!(!response.starts_with(b"HTTP/1.1 200"));
        assert!(
            matches!(self.origin.accept(),Err(error)if error.kind()==std::io::ErrorKind::WouldBlock),
            "missing integrity opened the owned origin"
        );
    }
    fn stopped_during<T>(&self, pid: u32, operation: impl FnOnce() -> T) -> T {
        let name = self.name.clone();
        let (ready_sender, ready_receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::scope(|scope| {
            let monitor = scope.spawn(move || {
                let service = manager()
                    .open_service(&name, ServiceAccess::QUERY_STATUS)
                    .unwrap();
                assert_eq!(service.query_status().unwrap().process_id, Some(pid));
                ready_sender.send(()).unwrap();
                let until = Instant::now() + Duration::from_secs(15);
                let mut stop_deadline = None;
                loop {
                    let status = service.query_status().unwrap();
                    if status.current_state == ServiceState::Stopped {
                        assert_eq!(
                            status.exit_code,
                            windows_service::service::ServiceExitCode::Win32(0)
                        );
                        break;
                    }
                    if status.current_state == ServiceState::StopPending && stop_deadline.is_none()
                    {
                        stop_deadline = Some(Instant::now() + Duration::from_secs(6));
                    }
                    assert!(
                        stop_deadline.is_none_or(|deadline| Instant::now() < deadline),
                        "native stop budget exceeded"
                    );
                    assert!(Instant::now() < until, "retired process stop unproven");
                    std::thread::sleep(Duration::from_millis(2));
                }
                // Release the old instance so SCM deletion can complete before re-registration.
                drop(service);
            });
            ready_receiver.recv_timeout(Duration::from_secs(2)).unwrap();
            let result = operation();
            monitor.join().unwrap();
            result
        })
    }
    fn successful_receipt(&self, pid: u32) {
        let receipt = self.read_receipt(pid);
        self.bind_receipt(&receipt, pid);
        assert_eq!(receipt["readiness"], true);
        for field in ["stopped", "drained", "audit_flushed"] {
            assert_eq!(receipt["exit"][field], true);
        }
        assert!(TcpStream::connect(self.runtime).is_err());
    }
    fn absent(&self) {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            match manager().open_service(&self.name, ServiceAccess::QUERY_STATUS) {
                Err(windows_service::Error::Winapi(error))
                    if error.raw_os_error() == Some(1060) =>
                {
                    break;
                }
                Err(error) => panic!("SCM absence unproven: {error}"),
                Ok(_) => assert!(Instant::now() < until, "SCM deletion pending"),
            };
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    fn cleanup(self) {
        self.absent();
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
#[ignore = "requires disposable hosted ordinary actor, delegated SCM create and protected service store"]
fn native_runtime_service_lifecycle_and_missing_credential_fail_closed() {
    assert_eq!(
        std::env::var("HUMANYMOUS_DISPOSABLE_SERVICE_ACTOR").as_deref(),
        Ok("1")
    );
    let run: u64 = std::env::var("HUMANYMOUS_SERVICE_PILOT_RUN_ID")
        .expect("controller run identity required")
        .parse()
        .unwrap();
    let source_sha = std::env::var("HUMANYMOUS_SERVICE_PILOT_SOURCE_SHA")
        .expect("controller source identity required");
    assert!(
        run > 0
            && matches!(source_sha.len(), 40 | 64)
            && source_sha
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    assert!(current_identity().unwrap().installer_allowed());
    let positive = Fixture::new(run, &source_sha, false);
    assert!(positive.plan().install(true).unwrap());
    let first_pid = positive.running(1, 1);
    positive.reject_missing_integrity();
    let second = positive.next();
    positive.stopped_during(first_pid, || second.plan().upgrade(true).unwrap());
    positive.successful_receipt(first_pid);
    let second_pid = second.running(2, 2);
    assert_ne!(first_pid, second_pid);
    second.reject_missing_integrity();
    second.stopped_during(second_pid, || {
        service::rollback(&second.root, second.key, true).unwrap()
    });
    second.successful_receipt(second_pid);
    let rollback_pid = positive.running(1, 2);
    assert_ne!(first_pid, rollback_pid);
    assert_ne!(second_pid, rollback_pid);
    positive.reject_missing_integrity();
    assert!(
        second.plan().upgrade(true).is_err(),
        "rollback must preserve replay floor"
    );
    assert_eq!(
        positive.running(1, 2),
        rollback_pid,
        "replay refusal changed the running process"
    );
    positive.stopped_during(rollback_pid, || {
        manager()
            .open_service(&positive.name, ServiceAccess::STOP)
            .unwrap()
            .stop()
            .unwrap();
    });
    positive.successful_receipt(rollback_pid);
    service::remove(&positive.root, positive.key, true).unwrap();
    second.absent();
    positive.cleanup();

    let negative = Fixture::new(run, &source_sha, true);
    let done = std::sync::atomic::AtomicBool::new(false);
    let (install, (negative_pid, failed_stop)) = std::thread::scope(|scope| {
        let monitor = scope.spawn(|| {
            let until = Instant::now() + Duration::from_secs(20);
            let mut pid = None;
            let mut failed_stop = false;
            while !done.load(std::sync::atomic::Ordering::Acquire) && Instant::now() < until {
                if let Ok(service) =
                    manager().open_service(&negative.name, ServiceAccess::QUERY_STATUS)
                    && let Ok(status) = service.query_status()
                {
                    if let Some(observed) = status.process_id.filter(|pid| *pid > 0) {
                        assert!(
                            pid.is_none_or(|old| old == observed),
                            "unexpected service PID change"
                        );
                        pid = Some(observed);
                    }
                    if status.current_state == ServiceState::Stopped
                        && status.exit_code != windows_service::service::ServiceExitCode::Win32(0)
                    {
                        failed_stop = true;
                    }
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            (pid, failed_stop)
        });
        let install = negative.plan().install(true);
        done.store(true, std::sync::atomic::Ordering::Release);
        (install, monitor.join().unwrap())
    });
    assert!(install.is_err());
    assert!(
        failed_stop,
        "nonzero native stopped state must be independently observed"
    );
    negative.absent();
    let negative_pid = negative_pid.expect("actual negative SCM PID required");
    let receipt = negative.read_receipt(negative_pid);
    negative.bind_receipt(&receipt, negative_pid);
    assert_eq!(receipt["readiness"], false);
    assert_eq!(receipt["missing_credential_fail_closed"], true);
    assert!(receipt["exit"].is_null());
    assert!(TcpStream::connect(negative.runtime).is_err());
    negative.cleanup();
    println!(
        "\n{}",
        json!({"schema":"humanymous.runtime-service-pilot/1","case":"native-runtime-service-lifecycle-and-missing-credential","source_sha":source_sha,"run_id":run,"missing_credential_fail_closed":true,"service_principal_verified":true,"readiness_verified":true,"graceful_stop_verified":true,"host_exit_stopped":true,"credential_cleanup_verified":true,"state_cleanup_verified":true,"receipt_binding_verified":true,"scm_registration_removed":true,"runtime_upgrade_verified":true,"runtime_rollback_verified":true,"replay_floor_verified":true,"product_qualified":false})
    );
}

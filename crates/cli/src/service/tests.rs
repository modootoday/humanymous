use super::*;
use std::{
    cell::RefCell,
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

struct FakeManager {
    active: RefCell<Option<(u64, Identity)>>,
    fail_sequence: std::cell::Cell<u64>,
    fail_readiness: std::cell::Cell<u64>,
    response: Arc<Mutex<String>>,
}
impl Manager for FakeManager {
    fn absent(&self, _: &Descriptor) -> Result<()> {
        if self.active.borrow().is_some() {
            Err(error("collision"))
        } else {
            Ok(())
        }
    }
    fn register(&self, g: &Generation, _: &Path, d: &Descriptor) -> Result<Identity> {
        self.absent(d)?;
        if self.fail_sequence.get() == d.sequence {
            return Err(error("injected native registration failure"));
        }
        let identity = Identity {
            configuration: format!("fixture-{}", g.sequence),
            service_sid: if cfg!(windows) {
                Some("S-1-5-80-1-2-3-4-5".into())
            } else {
                None
            },
        };
        *self.active.borrow_mut() = Some((g.sequence, identity.clone()));
        *self.response.lock().unwrap() = if self.fail_readiness.get() == d.sequence {
            "not-ready".into()
        } else {
            d.ready_response.clone()
        };
        Ok(identity)
    }
    fn verify(&self, g: &Generation, _: &Path, _: &Descriptor) -> Result<()> {
        if self
            .active
            .borrow()
            .as_ref()
            .is_some_and(|(s, _)| *s == g.sequence)
        {
            Ok(())
        } else {
            Err(error("changed manager"))
        }
    }
    fn start(&self, _: &Descriptor) -> Result<()> {
        Ok(())
    }
    fn running(&self, _: &Descriptor) -> Result<bool> {
        Ok(self.active.borrow().is_some())
    }
    fn unregister(&self, _: &Descriptor) -> Result<()> {
        self.active.borrow_mut().take();
        Ok(())
    }
}
struct Probe {
    address: std::net::SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    response: Arc<Mutex<String>>,
}
impl Probe {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let response = Arc::new(Mutex::new(String::new()));
        let s = stop.clone();
        let r = response.clone();
        let thread = std::thread::spawn(move || {
            while !s.load(Ordering::Relaxed) {
                if let Ok((mut socket, _)) = listener.accept() {
                    socket
                        .set_read_timeout(Some(Duration::from_millis(100)))
                        .unwrap();
                    let mut bytes = [0; 64];
                    let _ = socket.read(&mut bytes);
                    let _ = socket.write_all(r.lock().unwrap().as_bytes());
                } else {
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        });
        Self {
            address,
            stop,
            thread: Some(thread),
            response,
        }
    }
    fn manager(&self) -> FakeManager {
        FakeManager {
            active: RefCell::new(None),
            fail_sequence: std::cell::Cell::new(0),
            fail_readiness: std::cell::Cell::new(0),
            response: self.response.clone(),
        }
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn plan(root: &Path, probe: &Probe, sequence: u64) -> ServicePlan {
    let mut d = crate::release::tests::descriptor(sequence);
    d.artifact_sha256 = digest(b"verified fixture artifact");
    d.readiness = probe.address;
    let (release, key) = crate::release::tests::signed(&d);
    ServicePlan::inspect(root, &root.join("source"), release, key).unwrap()
}
#[test]
fn pure_plan_explicit_opt_in_install_repeat_upgrade_and_known_rollback() {
    let fixture = crate::installation::tests::Fixture::new();
    let probe = Probe::new();
    let manager = probe.manager();
    let p = plan(&fixture.0, &probe, 1);
    let names = fs::read_dir(&fixture.0).unwrap().count();
    assert!(
        p.describe()["requires_explicit_activation"]
            .as_bool()
            .unwrap()
    );
    assert!(p.install_with(&manager, false).is_err());
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), names);
    assert!(p.install_with(&manager, true).unwrap());
    assert!(!p.install_with(&manager, true).unwrap());
    let p2 = plan(&fixture.0, &probe, 2);
    p2.upgrade_with(&manager, true).unwrap();
    assert_eq!(
        read_state(&fixture.0, &p.key)
            .unwrap()
            .unwrap()
            .current
            .sequence,
        2
    );
    rollback_with(&fixture.0, p.key, true, &manager).unwrap();
    let state = read_state(&fixture.0, &p.key).unwrap().unwrap();
    assert_eq!(state.current.sequence, 1);
    assert_eq!(state.highest, 2);
    assert!(p2.upgrade_with(&manager, true).is_err());
    assert!(rollback_with(&fixture.0, p.key, true, &manager).is_err());
}
#[test]
fn registration_failure_cleans_new_files_and_restores_previous() {
    let fixture = crate::installation::tests::Fixture::new();
    let probe = Probe::new();
    let manager = probe.manager();
    let p = plan(&fixture.0, &probe, 1);
    manager.fail_sequence.set(1);
    assert!(p.install_with(&manager, true).is_err());
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
    manager.fail_sequence.set(0);
    p.install_with(&manager, true).unwrap();
    let p2 = plan(&fixture.0, &probe, 2);
    manager.fail_sequence.set(2);
    assert!(p2.upgrade_with(&manager, true).is_err());
    assert_eq!(
        read_state(&fixture.0, &p.key)
            .unwrap()
            .unwrap()
            .current
            .sequence,
        1
    );
    assert_eq!(manager.active.borrow().as_ref().unwrap().0, 1);
    assert!(!fixture.0.join("pending.json").exists());
}
#[test]
fn mutation_pending_and_wrong_key_block_before_manager_changes() {
    let fixture = crate::installation::tests::Fixture::new();
    let probe = Probe::new();
    let manager = probe.manager();
    let p = plan(&fixture.0, &probe, 1);
    p.install_with(&manager, true).unwrap();
    assert!(read_state(&fixture.0, &[0; 32]).is_err());
    let state = read_state(&fixture.0, &p.key).unwrap().unwrap();
    fs::write(state.current.executable(&fixture.0), b"tampered").unwrap();
    assert!(read_state(&fixture.0, &p.key).is_err());
    assert!(p.install_with(&manager, true).is_err());
    write_new(&fixture.0.join("pending.json"), b"incomplete").unwrap();
    assert!(check_root(&fixture.0).is_err());
    assert_eq!(manager.active.borrow().as_ref().unwrap().0, 1);
}

#[test]
fn readiness_failure_restores_previous_and_keeps_replay_floor() {
    let fixture = crate::installation::tests::Fixture::new();
    let probe = Probe::new();
    let manager = probe.manager();
    let p = plan(&fixture.0, &probe, 1);
    p.install_with(&manager, true).unwrap();
    let p2 = plan(&fixture.0, &probe, 2);
    manager.fail_readiness.set(2);
    assert!(p2.upgrade_with(&manager, true).is_err());
    let state = read_state(&fixture.0, &p.key).unwrap().unwrap();
    assert_eq!(state.current.sequence, 1);
    assert_eq!(state.highest, 1);
    assert_eq!(manager.active.borrow().as_ref().unwrap().0, 1);
    assert!(!fixture.0.join("pending.json").exists());
    assert!(
        !Generation::new(p2.release.clone(), &p2.descriptor)
            .path(&fixture.0)
            .exists()
    );
}

#[test]
fn create_new_lock_blocks_concurrent_mutation_and_releases_cleanly() {
    let fixture = crate::installation::tests::Fixture::new();
    let probe = Probe::new();
    let manager = probe.manager();
    let p = plan(&fixture.0, &probe, 1);
    let guard = Lock::acquire(&fixture.0).unwrap();
    assert!(p.install_with(&manager, true).is_err());
    assert!(manager.active.borrow().is_none());
    drop(guard);
    assert!(p.install_with(&manager, true).is_ok());
}

#[test]
fn failed_predecessor_registration_restores_current_without_stale_identity() {
    let fixture = crate::installation::tests::Fixture::new();
    let probe = Probe::new();
    let manager = probe.manager();
    let p = plan(&fixture.0, &probe, 1);
    p.install_with(&manager, true).unwrap();
    let p2 = plan(&fixture.0, &probe, 2);
    p2.upgrade_with(&manager, true).unwrap();
    manager.fail_sequence.set(1);
    assert!(rollback_with(&fixture.0, p.key, true, &manager).is_err());
    let state = read_state(&fixture.0, &p.key).unwrap().unwrap();
    assert_eq!(state.current.sequence, 2);
    assert_eq!(state.highest, 2);
    assert_eq!(manager.active.borrow().as_ref().unwrap().0, 2);
    assert!(!fixture.0.join("pending.json").exists());
}

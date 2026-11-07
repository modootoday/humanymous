//! Signed immutable generations and reversible, explicitly activated OS services.
mod backend;
mod storage;
use crate::{
    installation::permissions::plain,
    release::{Descriptor, SignedRelease, digest, error},
};
#[cfg(test)]
use backend::Identity;
use backend::{Manager, Native};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
#[cfg(test)]
use storage::write_new;
use storage::{
    Generation, Lock, State, begin, check_root, finish, read_bounded, read_state, save_state,
};
type Result<T> = std::io::Result<T>;

/// Read-only verified deployment proposal. Callers must explicitly opt in to apply.
pub struct ServicePlan {
    root: PathBuf,
    release: SignedRelease,
    descriptor: Descriptor,
    bytes: Vec<u8>,
    key: [u8; 32],
    state: Option<State>,
}

/// Bounded native service diagnostic. This is never full product qualification.
#[derive(Debug, Serialize)]
pub struct Doctor {
    /// Report schema.
    pub schema: &'static str,
    /// Native OS service registration matched owned state.
    pub registered: bool,
    /// Native manager reports running.
    pub running: bool,
    /// Exact loopback diagnostic challenge succeeded.
    pub diagnostic_ready: bool,
    /// Always false until runtime/credential/peer qualification is integrated.
    pub product_qualified: bool,
    /// Signed active release sequence.
    pub sequence: u64,
    /// Highest accepted sequence, preserved by rollback.
    pub highest_sequence: u64,
}

impl ServicePlan {
    /// Verify a signed release, private root and immutable current state without mutation.
    pub fn inspect(
        root: &Path,
        artifact: &Path,
        release: SignedRelease,
        key: [u8; 32],
    ) -> Result<Self> {
        check_root(root)?;
        let descriptor = release.verify(&key)?;
        plain(artifact)?;
        let bytes = read_bounded(artifact, 256 * 1024 * 1024)?;
        if bytes.is_empty() || digest(&bytes) != descriptor.artifact_sha256 {
            return Err(error("signed artifact digest mismatch"));
        }
        let state = read_state(root, &key)?;
        if let Some(s) = &state
            && s.current.descriptor(&key)?.service_name != descriptor.service_name
        {
            return Err(error("service name cannot change during deployment"));
        }
        Ok(Self {
            root: root.into(),
            release,
            descriptor,
            bytes,
            key,
            state,
        })
    }
    /// Exact managed paths and OS registration intent, without taking a lock.
    pub fn describe(&self) -> serde_json::Value {
        let generation = Generation::new(self.release.clone(), &self.descriptor);
        serde_json::json!({"schema":"humanymous.service-plan.v1","os":std::env::consts::OS,"arch":std::env::consts::ARCH,"service":self.descriptor.service_name,"sequence":self.descriptor.sequence,"current_sequence":self.state.as_ref().map(|s|s.current.sequence),"highest_sequence":self.state.as_ref().map(|s|s.highest),"artifact_sha256":self.descriptor.artifact_sha256,"generation":generation.path(&self.root),"executable":generation.executable(&self.root),"definition":if cfg!(windows){None}else{Some(generation.definition_path(&self.root,&self.descriptor))},"state":self.root.join("deployment.json"),"requires_explicit_activation":true,"autostart":false,"product_qualified":false})
    }
    /// Reject an unrelated existing deployment before describing installation.
    pub fn check_install(&self) -> Result<()> {
        if self
            .state
            .as_ref()
            .is_some_and(|s| s.current.release != self.release)
        {
            return Err(error("existing deployment requires service-upgrade"));
        }
        Ok(())
    }
    /// Reject replay and unsupported upgrade intent without a lock or write.
    pub fn check_upgrade(&self) -> Result<()> {
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| error("no owned service deployment"))?;
        upgrade_descriptor(state, &self.descriptor, &self.key).map(|_| ())
    }
    /// Stage, register and start a new owned service, reverting ordinary failures.
    pub fn install(&self, activate: bool) -> Result<bool> {
        self.install_with(&Native, activate)
    }
    fn install_with(&self, manager: &impl Manager, activate: bool) -> Result<bool> {
        if !activate {
            return Err(error("--activate required for service mutation"));
        }
        let lock = Lock::acquire(&self.root)?;
        let current = read_state(&self.root, &self.key)?;
        if let Some(s) = current {
            if s.current.release == self.release {
                manager.verify(&s.current, &self.root, &self.descriptor)?;
                backend::wait_ready(manager, &self.descriptor)?;
                lock.release()?;
                return Ok(false);
            }
            return Err(error("existing deployment requires service-upgrade"));
        }
        manager.absent(&self.descriptor)?;
        let mut g = Generation::new(self.release.clone(), &self.descriptor);
        begin(&self.root, "install")?;
        stage_or_abort(&g, &self.root, &self.descriptor, self.bytes.clone())?;
        if let Err(cause) = activate_generation(manager, &mut g, &self.root, &self.descriptor) {
            cleanup_failed(manager, &g, &self.root, &self.key, &self.descriptor)?;
            finish(&self.root)?;
            return Err(cause);
        }
        let state = State {
            schema: "humanymous.deployment.v1".into(),
            trust_key: hex::encode(self.key),
            highest: g.sequence,
            current: g.clone(),
            previous: None,
            retired: vec![],
        };
        if let Err(cause) = save_state(&self.root, &state) {
            cleanup_failed(manager, &g, &self.root, &self.key, &self.descriptor)?;
            finish(&self.root)?;
            return Err(cause);
        }
        finish(&self.root)?;
        lock.release()?;
        Ok(true)
    }
    /// Upgrade only to a higher signed sequence, retaining the known predecessor.
    pub fn upgrade(&self, activate: bool) -> Result<()> {
        self.upgrade_with(&Native, activate)
    }
    fn upgrade_with(&self, manager: &impl Manager, activate: bool) -> Result<()> {
        if !activate {
            return Err(error("--activate required for service mutation"));
        }
        let lock = Lock::acquire(&self.root)?;
        let mut state = read_state(&self.root, &self.key)?
            .ok_or_else(|| error("no owned service deployment"))?;
        let old_d = upgrade_descriptor(&state, &self.descriptor, &self.key)?;
        manager.verify(&state.current, &self.root, &old_d)?;
        let mut next = Generation::new(self.release.clone(), &self.descriptor);
        begin(&self.root, "upgrade")?;
        stage_or_abort(&next, &self.root, &self.descriptor, self.bytes.clone())?;
        manager.unregister(&old_d)?;
        if let Err(cause) = activate_generation(manager, &mut next, &self.root, &self.descriptor) {
            cleanup_failed(manager, &next, &self.root, &self.key, &self.descriptor)?;
            activate_generation(manager, &mut state.current, &self.root, &old_d)
                .map_err(|e| error(format!("{cause}; prior service restore failed: {e}")))?;
            save_state(&self.root, &state)?;
            finish(&self.root)?;
            return Err(cause);
        }
        let old_state = state.clone();
        if let Some(previous) = state.previous.take() {
            state.retired.push(previous);
        }
        state.previous = Some(std::mem::replace(&mut state.current, next.clone()));
        state.highest = next.sequence;
        if let Err(cause) = save_state(&self.root, &state) {
            cleanup_failed(manager, &next, &self.root, &self.key, &self.descriptor)?;
            let mut restore = old_state;
            activate_generation(manager, &mut restore.current, &self.root, &old_d)?;
            save_state(&self.root, &restore)?;
            finish(&self.root)?;
            return Err(cause);
        }
        finish(&self.root)?;
        lock.release()
    }
}

fn upgrade_descriptor(state: &State, next: &Descriptor, key: &[u8; 32]) -> Result<Descriptor> {
    if next.sequence <= state.highest || state.retired.len() >= 14 {
        return Err(error("release replay or retained generation budget"));
    }
    let previous = state.current.descriptor(key)?;
    if previous.service_name != next.service_name {
        return Err(error("service name changed after planning"));
    }
    Ok(previous)
}

/// Check immutable files, native registration and diagnostic readiness without writes.
pub fn doctor(root: &Path, key: [u8; 32]) -> Result<Doctor> {
    check_root(root)?;
    let state = read_state(root, &key)?.ok_or_else(|| error("no owned deployment"))?;
    let d = state.current.descriptor(&key)?;
    Native.verify(&state.current, root, &d)?;
    let running = Native.running(&d)?;
    let diagnostic_ready = running && backend::ready(&d).is_ok();
    Ok(Doctor {
        schema: "humanymous.service-doctor.v1",
        registered: true,
        running,
        diagnostic_ready,
        product_qualified: false,
        sequence: state.current.sequence,
        highest_sequence: state.highest,
    })
}

/// Inspect rollback/removal paths without manager calls, probes, locks or writes.
pub fn existing_plan(root: &Path, key: [u8; 32], operation: &str) -> Result<serde_json::Value> {
    check_root(root)?;
    let state = read_state(root, &key)?.ok_or_else(|| error("no owned deployment"))?;
    if operation != "service-remove" && operation != "service-rollback" {
        return Err(error("unsupported existing deployment plan"));
    }
    if operation == "service-rollback" && state.previous.is_none() {
        return Err(error("no signed predecessor available"));
    }
    Ok(
        serde_json::json!({"schema":"humanymous.service-plan.v1","operation":operation,"service":state.current.descriptor(&key)?.service_name,"current_sequence":state.current.sequence,"previous_sequence":state.previous.map(|g|g.sequence),"highest_sequence":state.highest,"requires_explicit_activation":true,"product_qualified":false}),
    )
}

/// Explicitly reactivate only the known signed predecessor, preserving replay floor.
pub fn rollback(root: &Path, key: [u8; 32], activate: bool) -> Result<()> {
    rollback_with(root, key, activate, &Native)
}
fn rollback_with(root: &Path, key: [u8; 32], activate: bool, manager: &impl Manager) -> Result<()> {
    if !activate {
        return Err(error("--activate required for service mutation"));
    }
    check_root(root)?;
    let lock = Lock::acquire(root)?;
    let mut state = read_state(root, &key)?.ok_or_else(|| error("no owned deployment"))?;
    let mut previous = state
        .previous
        .clone()
        .ok_or_else(|| error("no signed predecessor available"))?;
    let old_d = state.current.descriptor(&key)?;
    let prev_d = previous.descriptor(&key)?;
    manager.verify(&state.current, root, &old_d)?;
    begin(root, "rollback")?;
    manager.unregister(&old_d)?;
    if let Err(cause) = activate_generation(manager, &mut previous, root, &prev_d) {
        if previous.identity.is_some() {
            manager.verify(&previous, root, &prev_d)?;
            manager.unregister(&prev_d)?;
        } else {
            manager.absent(&prev_d)?;
        }
        activate_generation(manager, &mut state.current, root, &old_d)?;
        save_state(root, &state)?;
        finish(root)?;
        return Err(cause);
    }
    state
        .retired
        .push(std::mem::replace(&mut state.current, previous));
    state.previous = None;
    save_state(root, &state)?;
    finish(root)?;
    lock.release()
}

/// Remove only verified owned services and generations; preserve the caller's root.
pub fn remove(root: &Path, key: [u8; 32], activate: bool) -> Result<()> {
    if !activate {
        return Err(error("--activate required for service mutation"));
    }
    check_root(root)?;
    let lock = Lock::acquire(root)?;
    let state = read_state(root, &key)?.ok_or_else(|| error("no owned deployment"))?;
    let d = state.current.descriptor(&key)?;
    Native.verify(&state.current, root, &d)?;
    begin(root, "remove")?;
    Native.unregister(&d)?;
    for generation in state
        .retired
        .iter()
        .chain(state.previous.iter())
        .chain(std::iter::once(&state.current))
    {
        generation.remove(root, &key)?;
    }
    fs::remove_file(root.join("deployment.json"))?;
    finish(root)?;
    lock.release()
}

fn activate_generation(
    manager: &impl Manager,
    g: &mut Generation,
    root: &Path,
    d: &Descriptor,
) -> Result<()> {
    // A retained generation's old identity is not proof that a new registration
    // succeeded. Failure cleanup must distinguish absent from newly owned state.
    g.identity = None;
    g.identity = Some(manager.register(g, root, d)?);
    g.grant(root)?;
    #[cfg(all(windows, feature = "runtime-service"))]
    crate::runtime_host::activation::grant(
        root,
        d,
        g.identity
            .as_ref()
            .and_then(|identity| identity.service_sid.as_deref())
            .ok_or_else(|| error("runtime service SID missing"))?,
    )?;
    manager.verify(g, root, d)?;
    manager.start(d)?;
    backend::wait_ready(manager, d)
}
fn cleanup_failed(
    manager: &impl Manager,
    g: &Generation,
    root: &Path,
    key: &[u8; 32],
    d: &Descriptor,
) -> Result<()> {
    if g.identity.is_some() {
        manager.verify(g, root, d)?;
        manager.unregister(d)?;
    } else {
        manager.absent(d)?;
    }
    // Registration may have failed before the Windows ACL grant was applied.
    let mut staged = g.clone();
    if staged.plan(root)?.verify_service(None).is_ok() {
        staged.identity = None;
    }
    staged.remove(root, key)
}
fn stage_or_abort(g: &Generation, root: &Path, d: &Descriptor, bytes: Vec<u8>) -> Result<()> {
    if let Err(cause) = g.stage(root, d, bytes) {
        if !g.path(root).exists()
            && !g
                .definition_path(root, d)
                .parent()
                .is_some_and(Path::exists)
        {
            finish(root)?;
        }
        return Err(cause);
    }
    Ok(())
}

#[cfg(test)]
mod tests;

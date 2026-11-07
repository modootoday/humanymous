//! Explicit signed launch-document grants between native registration and start.
use super::{HostPlan, invalid};
#[cfg(feature = "runtime-service-pilot")]
use crate::installation::permissions::private_parent;
use crate::{
    installation::permissions::{plain, private_file},
    release::{Descriptor, digest},
};
use humanymous_native_security::{
    process::current_identity,
    storage::{self, ServiceRole},
};
use std::{
    fs,
    io::{self, Read},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

fn load(
    root: &Path,
    descriptor: &Descriptor,
    sid: &str,
) -> io::Result<Option<(PathBuf, HostPlan)>> {
    if descriptor
        .arguments
        .first()
        .is_none_or(|value| value != "runtime-host")
    {
        return Ok(None);
    }
    let args = &descriptor.arguments;
    if args.len() != 5
        || args[1] != "--plan"
        || args[3] != "--plan-sha256"
        || !super::plan::hash(&args[4])
    {
        return Err(invalid("strict signed runtime launch arguments required"));
    }
    let path = PathBuf::from(&args[2]);
    if path.parent() != Some(root) {
        return Err(invalid(
            "launch document must be a direct owned deployment child",
        ));
    }
    plain(&path)?;
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x00200000)
        .open(&path)?;
    if private_file(&path).is_err() {
        storage::verify_service_access(
            &file,
            current_identity()?.sid(),
            sid,
            ServiceRole::Executable,
        )?;
    }
    let mut bytes = Vec::new();
    (&mut file).take(32769).read_to_end(&mut bytes)?;
    if bytes.len() > 32768 || digest(&bytes) != args[4] {
        return Err(invalid("signed runtime launch-document digest mismatch"));
    }
    let plan: HostPlan =
        serde_json::from_slice(&bytes).map_err(|_| invalid("runtime launch-document schema"))?;
    plan.validate()?;
    if plan.service_name != descriptor.service_name
        || plan.executable_sha256 != descriptor.artifact_sha256
        || plan.diagnostic != descriptor.readiness
        || plan.ready_response != descriptor.ready_response
        || plan.installer_owner_sid.as_deref() != Some(current_identity()?.sid())
    {
        return Err(invalid(
            "runtime launch bindings do not match signed release and installer",
        ));
    }
    #[cfg(feature = "runtime-service-pilot")]
    if let Some(pilot) = &plan.pilot {
        if pilot.parent.parent() != Some(root) || pilot.parent == path {
            return Err(invalid(
                "fixture delegated parent must be a direct deployment child",
            ));
        }
        plain(&pilot.parent)?;
    }
    Ok(Some((path, plan)))
}

/// Called only after the owned SCM registration resolves its real service SID.
pub(crate) fn grant(root: &Path, descriptor: &Descriptor, sid: &str) -> io::Result<()> {
    if let Some((path, plan)) = load(root, descriptor, sid)? {
        storage::grant_service_access(&path, sid, ServiceRole::Executable)?;
        #[cfg(feature = "runtime-service-pilot")]
        if let Some(pilot) = &plan.pilot {
            if private_parent(&pilot.parent).is_err() {
                verify_parent(&pilot.parent, sid)?;
            }
            storage::grant_service_access(&pilot.parent, sid, ServiceRole::StateDirectory)?;
        }
        #[cfg(not(feature = "runtime-service-pilot"))]
        let _ = plan;
    }
    Ok(())
}
#[cfg(feature = "runtime-service-pilot")]
fn verify_parent(path: &Path, sid: &str) -> io::Result<()> {
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x02200000)
        .open(path)?;
    storage::verify_service_access(
        &file,
        current_identity()?.sid(),
        sid,
        ServiceRole::StateDirectory,
    )
}
/// Revalidate signed bytes and either unchanged private or exact delegated ACLs.
pub(crate) fn verify(root: &Path, descriptor: &Descriptor, sid: &str) -> io::Result<()> {
    let _ = load(root, descriptor, sid)?;
    Ok(())
}
/// Called after native stop/deletion. Never repairs an unexpected descriptor.
pub(crate) fn revoke(root: &Path, descriptor: &Descriptor, sid: &str) -> io::Result<()> {
    if let Some((path, plan)) = load(root, descriptor, sid)? {
        #[cfg(feature = "runtime-service-pilot")]
        if let Some(pilot) = &plan.pilot
            && private_parent(&pilot.parent).is_err()
        {
            verify_parent(&pilot.parent, sid)?;
            storage::revoke_service_access(&pilot.parent, sid, ServiceRole::StateDirectory)?;
        }
        #[cfg(not(feature = "runtime-service-pilot"))]
        let _ = plan;
        if private_file(&path).is_err() {
            storage::revoke_service_access(&path, sid, ServiceRole::Executable)?;
        }
    }
    Ok(())
}

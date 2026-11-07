//! Opt-in CI actor metadata carrier, not product authority or host attestation.
//! Exact cross-process producer-handle/provider association remains Unsupported.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use windows_permissions::{
    LocalBox, SecurityDescriptor, Sid,
    constants::{AceType, SeObjectType, SecurityInformation},
    wrappers,
};

const CONTEXT: &str = "context.json";
const BINDING: &str = "binding.json";
const LIMIT: usize = 64 * 1024;
const BINDING_LIMIT: usize = 16 * 1024;
const READ: u32 = 0x120089;
const RX: u32 = 0x1200a9;
const FULL: u32 = 0x1f01ff;

fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "protected CI actor carrier required",
    )
}
fn nonce(value: &str) -> io::Result<()> {
    if value.len() != 32
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || value.bytes().all(|b| b == b'0')
    {
        return Err(denied());
    }
    Ok(())
}
fn actual_actor(value: &str, expected: &str) -> io::Result<()> {
    nonce(value)?;
    let name = format!("hmrun{}", &value[..12]);
    let (sid, _, _) = wrappers::LookupAccountName(Option::<&std::ffi::OsStr>::None, name)?;
    if sid.to_string() != expected {
        return Err(denied());
    }
    Ok(())
}
fn descriptor(sd: &SecurityDescriptor, actor: &Sid, directory: bool) -> io::Result<()> {
    let admin: LocalBox<Sid> = "S-1-5-32-544".parse()?;
    let system: LocalBox<Sid> = "S-1-5-18".parse()?;
    if sd.owner() != Some(&*admin) && sd.owner() != Some(&*system) {
        return Err(denied());
    }
    if actor == &*admin || actor == &*system {
        return Err(denied());
    }
    super::windows::protected(sd)?;
    let acl = sd.dacl().ok_or_else(denied)?;
    if acl.len() != 3 {
        return Err(denied());
    }
    let mut counts = [0_u8; 3];
    for index in 0..acl.len() {
        let ace = acl.get_ace(index).ok_or_else(denied)?;
        if ace.ace_type() != AceType::ACCESS_ALLOWED_ACE_TYPE {
            return Err(denied());
        }
        let role = if ace.sid() == Some(&*admin) {
            0
        } else if ace.sid() == Some(&*system) {
            1
        } else if ace.sid() == Some(actor) {
            2
        } else {
            return Err(denied());
        };
        let (mask, flags) = if role == 2 {
            (if directory { RX } else { READ }, 0)
        } else {
            (FULL, if directory { 3 } else { 0 })
        };
        if ace.mask().bits() != mask || ace.flags().bits() != flags {
            return Err(denied());
        }
        counts[role] += 1;
    }
    if counts != [1, 1, 1] {
        return Err(denied());
    }
    Ok(())
}
fn inspect(file: &File, actor: &str, directory: bool) -> io::Result<()> {
    let metadata = file.metadata()?;
    if metadata.file_attributes() & 0x400 != 0
        || metadata.is_dir() != directory
        || (!directory && !metadata.is_file())
    {
        return Err(denied());
    }
    let sd = wrappers::GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )?;
    let actor: LocalBox<Sid> = actor.parse()?;
    descriptor(&sd, &actor, directory)
}
fn set_new_acl(file: &mut File, actor: &str, directory: bool) -> io::Result<()> {
    // Never repair/change an owner. An unsupported default token owner fails.
    let sd = wrappers::GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner,
    )?;
    let admin: LocalBox<Sid> = "S-1-5-32-544".parse()?;
    let system: LocalBox<Sid> = "S-1-5-18".parse()?;
    if sd.owner() != Some(&*admin) && sd.owner() != Some(&*system) {
        return Err(denied());
    }
    let sddl = if directory {
        format!("D:P(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)(A;;0x1200a9;;;{actor})")
    } else {
        format!("D:P(A;;FA;;;BA)(A;;FA;;;SY)(A;;0x120089;;;{actor})")
    };
    let sd: LocalBox<SecurityDescriptor> = sddl.parse()?;
    wrappers::SetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        None,
        None,
        sd.dacl(),
        None,
    )?;
    inspect(file, actor, directory)
}
fn opened(path: &Path, directory: bool, access: u32) -> io::Result<File> {
    super::service::require_local_path(path)?;
    OpenOptions::new()
        .access_mode(access)
        .share_mode(1)
        .custom_flags(if directory { 0x02200000 } else { 0x00200000 })
        .open(path)
}

/// Real owned files/pins. This is metadata publication, not trusted launch proof.
/// No Debug/Serialize, inner File getter, implicit cleanup or authority upgrade.
pub struct PublishedActorContext {
    base: PathBuf,
    root: PathBuf,
    nonce: String,
    actor: String,
    created_root: bool,
    created_context: bool,
    created_binding: bool,
    file_pins: Vec<File>,
    root_pin: Option<File>,
}
/// A publication/cleanup failure retains actual owned files and handles.
pub enum ContextPublishError {
    /// No new owned directory was created.
    BeforeCreation(io::Error),
    /// Keep this owner for explicit cleanup; Drop only closes handles.
    Retained {
        /// Actual unpublished/partially published carrier owner.
        owner: Box<PublishedActorContext>,
        /// Private OS error; do not emit raw context/path data.
        error: io::Error,
    },
}
impl PublishedActorContext {
    /// Fixed carrier path, only a locator. It grants no authority to its holder.
    pub fn context_path(&self) -> PathBuf {
        self.root.join(CONTEXT)
    }
    /// Independent binding path, also untrusted until native consumer checks.
    pub fn binding_path(&self) -> PathBuf {
        self.root.join(BINDING)
    }
    /// Require the still-unimplemented exact cross-process handle association.
    /// Neither path reopening, hash equality nor a caller DTO substitutes for it.
    pub fn require_exact_producer_identity(&self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "producer handle association unqualified",
        ))
    }
    fn write_new(&mut self, name: &str, bytes: &[u8]) -> io::Result<()> {
        let path = self.root.join(name);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .access_mode(0xc0060000)
            .share_mode(0)
            .custom_flags(0x00200000)
            .open(&path)?;
        match name {
            CONTEXT => self.created_context = true,
            BINDING => self.created_binding = true,
            _ => return Err(denied()),
        }
        set_new_acl(&mut file, &self.actor, false)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        let pin = opened(&path, false, READ)?;
        inspect(&pin, &self.actor, false)?;
        if pin.metadata()?.len() != bytes.len() as u64 {
            return Err(denied());
        }
        self.file_pins.push(pin);
        Ok(())
    }
    fn cleanup(&mut self) -> io::Result<()> {
        let expected = format!("hmctx-{}", self.nonce);
        if self.root.parent() != Some(self.base.as_path())
            || self.root.file_name().and_then(|v| v.to_str()) != Some(expected.as_str())
        {
            return Err(denied());
        }
        self.file_pins.clear();
        if self.created_context {
            fs::remove_file(self.root.join(CONTEXT))?;
            self.created_context = false;
        }
        if self.created_binding {
            fs::remove_file(self.root.join(BINDING))?;
            self.created_binding = false;
        }
        drop(self.root_pin.take());
        if self.created_root {
            fs::remove_dir(&self.root)?;
            self.created_root = false;
        }
        Ok(())
    }
    /// Call only after the parent's actual Job/reader-settled gates. This deletes
    /// ONLY the two create-new metadata files and empty owned directory, never a
    /// recursive tree. Failure returns the actual owner; Drop is not cleanup proof.
    pub fn finish(mut self) -> Result<(), ContextPublishError> {
        match self.cleanup() {
            Ok(()) => Ok(()),
            Err(error) => Err(ContextPublishError::Retained {
                owner: Box::new(self),
                error,
            }),
        }
    }
}

/// Publish only bounded metadata after actual elevated/Admin token and native
/// reserved-actor SID checks. Caller additionally owns hosted/source/artifact and
/// original-budget preflight; this function does NOT infer those from labels.
/// No user-owner fallback, owner repair, accounts, secrets or SDK operation.
pub fn publish_actor_context(
    run_nonce: &str,
    actor_sid: &str,
    context: &[u8],
    binding: &[u8],
) -> Result<PublishedActorContext, ContextPublishError> {
    let prepare = || -> io::Result<PublishedActorContext> {
        nonce(run_nonce)?;
        if context.is_empty()
            || context.len() > LIMIT
            || binding.is_empty()
            || binding.len() > BINDING_LIMIT
        {
            return Err(denied());
        }
        let identity = crate::native_security::process::current_identity()?;
        if !identity.elevated() || !identity.enabled_administrator() || identity.sid() == actor_sid
        {
            return Err(denied());
        }
        actual_actor(run_nonce, actor_sid)?;
        let base =
            PathBuf::from(std::env::var_os("ProgramData").ok_or_else(denied)?).canonicalize()?;
        let text = base.to_str().ok_or_else(denied)?;
        let base = PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(text));
        super::service::require_local_path(&base)?;
        Ok(PublishedActorContext {
            root: base.join(format!("hmctx-{run_nonce}")),
            base,
            nonce: run_nonce.into(),
            actor: actor_sid.into(),
            created_root: false,
            created_context: false,
            created_binding: false,
            file_pins: Vec::new(),
            root_pin: None,
        })
    };
    let mut owner = prepare().map_err(ContextPublishError::BeforeCreation)?;
    fs::create_dir(&owner.root).map_err(ContextPublishError::BeforeCreation)?;
    owner.created_root = true;
    let result = (|| -> io::Result<()> {
        let mut root = opened(&owner.root, true, 0x60000)?;
        set_new_acl(&mut root, actor_sid, true)?;
        owner.root_pin = Some(root);
        owner.write_new(CONTEXT, context)?;
        owner.write_new(BINDING, binding)?;
        drop(owner.root_pin.take());
        let root = opened(&owner.root, true, READ)?;
        inspect(&root, actor_sid, true)?;
        owner.root_pin = Some(root);
        Ok(())
    })();
    match result {
        Ok(()) => Ok(owner),
        Err(error) => Err(ContextPublishError::Retained {
            owner: Box::new(owner),
            error,
        }),
    }
}

/// Native role/ACL inspection with retained read-only files. This is not a
/// verified producer PID/handle/provider or trusted launch profile.
pub struct InspectedActorContext {
    _files: Vec<File>,
    _root: File,
    context: Vec<u8>,
    binding: Vec<u8>,
    actor: String,
    nonce: String,
}
impl InspectedActorContext {
    /// Claimed codec bytes from the actual pinned regular file; still unverified.
    pub fn context_bytes(&self) -> &[u8] {
        &self.context
    }
    /// Independent binding bytes; hashes/claims do not mint native authority.
    pub fn binding_bytes(&self) -> &[u8] {
        &self.binding
    }
    /// Actual current ordinary TokenUser SID, not a decoded claim.
    pub fn actor_sid(&self) -> &str {
        &self.actor
    }
    /// Reserved directory/account nonce, not authenticated freshness by itself.
    pub fn run_nonce(&self) -> &str {
        &self.nonce
    }
    /// Mandatory unresolved gate before any trusted-profile/HostIO upgrade.
    pub fn require_exact_producer_identity(&self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "producer handle association unqualified",
        ))
    }
}
fn bounded_read(file: &mut File, limit: usize) -> io::Result<Vec<u8>> {
    let expected = file.metadata()?.len();
    if expected == 0 || expected > limit as u64 {
        return Err(denied());
    }
    let mut bytes = Vec::new();
    (&mut *file)
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != expected || file.metadata()?.len() != expected {
        return Err(denied());
    }
    Ok(bytes)
}
/// Inspect a fixed context.json carrier for the ACTUAL reserved ordinary actor.
/// No GHA label/current-user fallback, ACL repair, account or shell operation.
pub fn inspect_actor_context(path: &Path) -> io::Result<InspectedActorContext> {
    super::service::require_local_path(path)?;
    if path.file_name().and_then(|v| v.to_str()) != Some(CONTEXT) {
        return Err(denied());
    }
    let root = path.parent().ok_or_else(denied)?;
    let run_nonce = root
        .file_name()
        .and_then(|v| v.to_str())
        .and_then(|v| v.strip_prefix("hmctx-"))
        .ok_or_else(denied)?;
    nonce(run_nonce)?;
    let identity = crate::native_security::process::current_identity()?;
    if !identity.installer_allowed() {
        return Err(denied());
    }
    actual_actor(run_nonce, identity.sid())?;
    let root_pin = opened(root, true, READ)?;
    inspect(&root_pin, identity.sid(), true)?;
    let mut context = opened(path, false, READ)?;
    inspect(&context, identity.sid(), false)?;
    let mut binding = opened(&root.join(BINDING), false, READ)?;
    inspect(&binding, identity.sid(), false)?;
    let context_bytes = bounded_read(&mut context, LIMIT)?;
    let binding_bytes = bounded_read(&mut binding, BINDING_LIMIT)?;
    Ok(InspectedActorContext {
        _files: vec![context, binding],
        _root: root_pin,
        context: context_bytes,
        binding: binding_bytes,
        actor: identity.sid().into(),
        nonce: run_nonce.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const ACTOR: &str = "S-1-5-21-1-2-3-1001";
    #[test]
    fn nonce_and_exact_role_descriptors_reject_owner_or_write_fallbacks() {
        assert!(nonce(&"a".repeat(32)).is_ok());
        for value in [
            "0".repeat(32),
            "A".repeat(32),
            "a".repeat(31),
            "../escape".into(),
        ] {
            assert!(nonce(&value).is_err());
        }
        let actor: LocalBox<Sid> = ACTOR.parse().unwrap();
        for owner in ["BA", "SY"] {
            for directory in [false, true] {
                let flags = if directory { "OICI" } else { "" };
                let mask = if directory { "0x1200a9" } else { "0x120089" };
                let text = format!(
                    "O:{owner}D:P(A;{flags};FA;;;BA)(A;{flags};FA;;;SY)(A;;{mask};;;{ACTOR})"
                );
                let sd: LocalBox<SecurityDescriptor> = text.parse().unwrap();
                assert!(descriptor(&sd, &actor, directory).is_ok());
                for bad in [
                    text.replace("D:P", "D:"),
                    text.replace(mask, "FA"),
                    text.replace("O:BA", &format!("O:{ACTOR}"))
                        .replace("O:SY", &format!("O:{ACTOR}")),
                    format!("{text}(A;;FR;;;WD)"),
                ] {
                    let sd: LocalBox<SecurityDescriptor> = bad.parse().unwrap();
                    assert!(descriptor(&sd, &actor, directory).is_err());
                }
            }
        }
    }
}

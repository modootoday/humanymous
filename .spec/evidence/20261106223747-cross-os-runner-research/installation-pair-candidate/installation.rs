//! Actual read-only installation queries; original trust producer is absent.
#![forbid(unsafe_code)]

use super::{QueryClock, Refusal, windows_service::ServiceObservation};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Component, Path, PathBuf},
    time::Duration,
};
use windows_permissions::{
    LocalBox, SecurityDescriptor,
    constants::{SeObjectType, SecurityInformation},
    wrappers,
};

/// Locators only. No SID, PID, key, original issuer or success flag is accepted.
pub(super) struct InstallationLocators {
    pub(super) root: PathBuf,
    pub(super) base_plans: [PathBuf; 3],
    /// Fixed publisher/service/observer order; all names are separately validated.
    pub(super) service_names: [String; 3],
}

struct HeldFile {
    file: File,
    descriptor: Option<LocalBox<SecurityDescriptor>>,
    /// Bounded nonsecret BASE bytes, not parsed/trusted authorization.
    bytes: Vec<u8>,
}

/// Retains every acquired owner on success AND failure. Never a native grant.
pub(super) struct InstallationObservation {
    pub(super) clock: QueryClock,
    locators: InstallationLocators,
    ancestors: Vec<HeldFile>,
    bases: Vec<HeldFile>,
    services: Vec<ServiceObservation>,
    self_identity: Option<crate::native_security::process::WindowsIdentity>,
    /// Native observations only; common ownership is not original-I provenance.
    observed_owner: Option<String>,
}

pub(super) struct InstallFailure {
    reason: Refusal,
    retained: InstallationObservation,
}
impl InstallFailure {
    pub(super) fn reason(&self) -> Refusal { self.reason }
    pub(super) fn retained(&self) -> &InstallationObservation { &self.retained }
}

// No constructor, test factory, bool or caller SID can inhabit this proof.
enum OriginalInstallerProof {}
pub(super) struct DelegatedInstallation {
    _observation: InstallationObservation,
    _original: OriginalInstallerProof,
}

fn local_root(path: &Path) -> Result<(), Refusal> {
    let text = path.to_str().ok_or(Refusal::InvalidSelection)?;
    let bytes = text.as_bytes();
    if bytes.len() < 4 || !bytes[0].is_ascii_alphabetic() || &bytes[1..3] != b":\\"
        || bytes[3..].contains(&b':') || text.ends_with('\\') || text.ends_with('.') || text.ends_with(' ')
        || path.components().any(|part| !matches!(part, Component::Prefix(_) | Component::RootDir | Component::Normal(_)))
    {
        return Err(Refusal::InvalidSelection);
    }
    // This classification does not prove NTFS, file ID or retained ancestry.
    crate::native_security::volume::require_fixed(&text[..3]).map_err(|_| Refusal::Query)
}

fn descriptor(file: &File) -> Result<LocalBox<SecurityDescriptor>, Refusal> {
    wrappers::GetSecurityInfo(file, SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl).map_err(|_| Refusal::Query)
}

fn open_directory(path: &Path) -> Result<File, Refusal> {
    OpenOptions::new().access_mode(0x0002_0001).share_mode(3)
        .custom_flags(0x0220_0000).open(path).map_err(|_| Refusal::Query)
}

fn inspect_directory(held: &mut HeldFile) -> Result<(), Refusal> {
    let metadata = held.file.metadata().map_err(|_| Refusal::Query)?;
    if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
        return Err(Refusal::Descriptor);
    }
    held.descriptor = Some(descriptor(&held.file)?);
    Ok(())
}

fn open_base(path: &Path) -> Result<File, Refusal> {
    // Read data/attributes/control; deny new write and delete sharing.
    OpenOptions::new().access_mode(0x0012_0089).share_mode(1)
        .custom_flags(0x0020_0000).open(path).map_err(|_| Refusal::Query)
}

fn inspect_base(held: &mut HeldFile) -> Result<(), Refusal> {
    let metadata = held.file.metadata().map_err(|_| Refusal::Query)?;
    const CAP: u64 = 64 * 1024;
    if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 || metadata.len() > CAP {
        return Err(Refusal::Descriptor);
    }
    held.descriptor = Some(descriptor(&held.file)?);
    (&mut held.file).take(CAP + 1).read_to_end(&mut held.bytes).map_err(|_| Refusal::Query)?;
    if held.bytes.len() as u64 > CAP || held.bytes.len() as u64 != metadata.len() {
        return Err(Refusal::Changed);
    }
    Ok(())
}

impl InstallationObservation {
    /// First Instant precedes validation and ALL native I/O. This observation
    /// does not certify the independent outer supervisor required for blocking calls.
    pub(super) fn observe(locators: InstallationLocators, cap: Duration) -> Result<Self, InstallFailure> {
        // A refused cap has no acquired native resource. Use a zero-life retained
        // owner rather than manufacture a fresh valid budget on that failure.
        let started = std::time::Instant::now();
        let clock = match QueryClock::from_start(started, cap) {
            Ok(clock) => clock,
            Err(reason) => return Err(InstallFailure { reason, retained: Self {
                clock: QueryClock { started, deadline: started }, locators,
                ancestors: Vec::new(), bases: Vec::new(), services: Vec::new(), self_identity: None, observed_owner: None,
            }}),
        };
        let mut owner = Self { clock, locators, ancestors: Vec::new(), bases: Vec::new(), services: Vec::new(), self_identity: None, observed_owner: None };
        match owner.query() {
            Ok(()) => Ok(owner),
            Err(reason) => Err(InstallFailure { reason, retained: owner }),
        }
    }

    fn query(&mut self) -> Result<(), Refusal> {
        self.clock.check()?;
        self.self_identity = Some(crate::native_security::process::current_identity().map_err(|_| Refusal::Query)?);
        if !self.self_identity.as_ref().unwrap().installer_allowed() { return Err(Refusal::WrongCaller); }
        self.clock.check()?;
        for name in &self.locators.service_names {
            crate::native_security::process::require_owned_service_name(name).map_err(|_| Refusal::InvalidSelection)?;
        }
        if self.locators.service_names[0] == self.locators.service_names[1]
            || self.locators.service_names[0] == self.locators.service_names[2]
            || self.locators.service_names[1] == self.locators.service_names[2]
        { return Err(Refusal::InvalidSelection); }
        local_root(&self.locators.root)?;
        self.clock.check()?;
        for base in &self.locators.base_plans {
            if base.parent() != Some(self.locators.root.as_path())
                || base.file_name().and_then(|name| name.to_str()).is_none_or(|name| {
                    name.is_empty() || name.len() > 64 || !name.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte))
                })
            { return Err(Refusal::InvalidSelection); }
        }
        if self.locators.base_plans[0] == self.locators.base_plans[1]
            || self.locators.base_plans[0] == self.locators.base_plans[2]
            || self.locators.base_plans[1] == self.locators.base_plans[2]
        { return Err(Refusal::InvalidSelection); }
        let prefixes: Vec<_> = self.locators.root.ancestors().map(Path::to_path_buf).collect();
        for prefix in prefixes.into_iter().rev() {
            self.clock.check()?;
            let file = open_directory(&prefix)?;
            self.ancestors.push(HeldFile { file, descriptor: None, bytes: Vec::new() });
            inspect_directory(self.ancestors.last_mut().unwrap())?;
            self.clock.check()?;
        }
        let root = self.ancestors.last().ok_or(Refusal::Descriptor)?;
        self.observed_owner = Some(root.descriptor.as_ref().ok_or(Refusal::Descriptor)?.owner().ok_or(Refusal::Descriptor)?.to_string());
        for base in &self.locators.base_plans {
            self.clock.check()?;
            let file = open_base(base)?;
            self.bases.push(HeldFile { file, descriptor: None, bytes: Vec::new() });
            inspect_base(self.bases.last_mut().unwrap())?;
            self.clock.check()?;
            if self.bases.last().unwrap().descriptor.as_ref().ok_or(Refusal::Descriptor)?.owner().map(ToString::to_string) != self.observed_owner {
                return Err(Refusal::Descriptor);
            }
        }
        for name in &self.locators.service_names {
            self.clock.check()?;
            let service = ServiceObservation::open_locator(name, self.clock.deadline).map_err(|_| Refusal::Query)?;
            self.services.push(service);
            let held = self.services.last().unwrap();
            let sd = held.descriptor_data().map_err(|_| Refusal::Query)?;
            if sd.owner().map(ToString::to_string) != self.observed_owner { return Err(Refusal::Descriptor); }
            let _configuration_data = held.configuration_data().map_err(|_| Refusal::Query)?;
            self.clock.check()?;
        }
        Ok(())
    }

    pub(super) fn service_name(&self, index: usize) -> Option<&str> {
        self.locators.service_names.get(index).map(String::as_str)
    }

    /// Repeat SAME-handle observations. This is not namespace/BASE authenticity.
    pub(super) fn recheck_files(&self) -> Result<(), Refusal> {
        self.clock.check()?;
        for held in self.ancestors.iter().chain(&self.bases) {
            let metadata = held.file.metadata().map_err(|_| Refusal::Query)?;
            if metadata.file_attributes() & 0x400 != 0 { return Err(Refusal::Changed); }
            let current = descriptor(&held.file)?;
            if current.owner().map(ToString::to_string) != held.descriptor.as_ref().ok_or(Refusal::Descriptor)?.owner().map(ToString::to_string) {
                return Err(Refusal::Changed);
            }
            if metadata.is_file() && metadata.len() != held.bytes.len() as u64 { return Err(Refusal::Changed); }
            self.clock.check()?;
        }
        Ok(())
    }

    /// Exact missing manufacturer, with ALL observed handles/clock retained.
    /// Common actual owner equality never becomes a trusted original I anchor.
    pub(super) fn admit_original(self) -> Result<DelegatedInstallation, InstallFailure> {
        Err(InstallFailure { reason: Refusal::OriginalInstallerProducerUnavailable, retained: self })
    }

    pub(super) fn namespace_admission_available(&self) -> Result<(), Refusal> {
        // No actual FileID/NTFS/ancestor mutation-exclusion capability is minted.
        Err(Refusal::HeldNamespaceAdmissionUnavailable)
    }
}

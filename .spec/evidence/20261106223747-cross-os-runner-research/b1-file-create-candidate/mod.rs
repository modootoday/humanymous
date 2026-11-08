//! Isolated Windows opened-object queries and descriptor-at-create bridge.
//! This package does not authenticate a producer, process, provider or hosted
//! environment. Native callers retain the same files/ancestors and enforce role,
//! ACL, fixed-local-volume and lifetime policy. Queries may block in the kernel.
#![cfg(windows)]
#![deny(unsafe_code)]

use std::{
    fs::File,
    path::{Path, PathBuf},
};

mod create;
/// Opt-in descriptor-at-create FILE primitive; no namespace/provider authority.
#[cfg(feature = "windows-file-create")]
pub mod create_file;
mod query;

/// Redacted bridge failure; no raw path, SID or context is exposed.
#[derive(Debug, PartialEq, Eq)]
pub enum BridgeError {
    /// Invalid bounded input.
    InvalidInput,
    /// Non-disk, reparse or invalid object/query output.
    InvalidObject,
    /// Required local NTFS profile facts were unavailable or unsupported.
    UnsupportedProvider,
    /// Native call failed; outputs have not been interpreted.
    OsFailure,
}

/// Native observation from the same borrowed open File, never launch authority.
/// No raw handle/File getter, caller constructor, serde or Debug implementation.
///
/// Public imports and forgeable claims are usable without invoking native queries:
/// ```
/// use humanymous_platform::windows_object::{
///     BridgeError, OpenedObjectObservation, UnverifiedIdentityClaim, inspect_opened,
/// };
/// let _query: fn(&std::fs::File) -> Result<OpenedObjectObservation, BridgeError> = inspect_opened;
/// assert_eq!(UnverifiedIdentityClaim::from_wire([42; 24]).to_wire(), [42; 24]);
/// ```
/// External callers cannot construct a native observation from wire values:
/// ```compile_fail,E0451
/// use humanymous_platform::windows_object::OpenedObjectObservation;
/// let _forged = OpenedObjectObservation {
///     identity: [0; 24], volume_path: String::new(), directory: false,
/// };
/// ```
pub struct OpenedObjectObservation {
    identity: [u8; 24],
    volume_path: String,
    directory: bool,
}

/// Forgeable wire expectation; equality alone cannot authorize any operation.
pub struct UnverifiedIdentityClaim([u8; 24]);
impl UnverifiedIdentityClaim {
    /// Decode volume64 little-endian plus opaque file128 bytes, without trust.
    pub fn from_wire(bytes: [u8; 24]) -> Self {
        Self(bytes)
    }
    /// Return unverified metadata for a strict parent association codec.
    pub fn to_wire(&self) -> [u8; 24] {
        self.0
    }
}
impl OpenedObjectObservation {
    /// Compare native observations while callers retain BOTH original owners.
    pub fn same_object(&self, other: &Self) -> bool {
        self.identity == other.identity && self.volume_path == other.volume_path
    }
    /// Export a separately typed, freely forgeable wire expectation.
    pub fn unverified_claim(&self) -> UnverifiedIdentityClaim {
        UnverifiedIdentityClaim(self.identity)
    }
    /// Compare metadata only; this grants no verified-carrier capability.
    pub fn matches_claim(&self, claim: &UnverifiedIdentityClaim) -> bool {
        self.identity == claim.0
    }
    /// Observed directory flag; caller still checks exact object role and ACL.
    pub fn is_directory(&self) -> bool {
        self.directory
    }
    /// Native volume-GUID prefix, a locator for an additional SAFE fixed-drive check.
    /// This string alone is not a reusable provider permit for another File.
    pub fn volume_guid_path(&self) -> &str {
        &self.volume_path
    }
}

/// Query the SAME live File. Supported syntax is NTFS/persistent-ACL/local GUID;
/// caller additionally enforces actual fixed drive, ancestors, role and custody.
pub fn inspect_opened(file: &File) -> Result<OpenedObjectObservation, BridgeError> {
    query::inspect(file)
}

/// Actual created-directory cleanup obligation, not a verified carrier.
#[must_use]
pub struct CreatedDirectory {
    path: PathBuf,
}
impl CreatedDirectory {
    /// Locator for independent inspection and explicit empty-directory cleanup.
    /// No Drop deletion or successful cleanup is implied by obtaining this path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Create a NEW directory with the borrowed descriptor applied AT CREATION.
/// Caller validates exact descriptor policy and retains the created obligation on
/// all later failures. No owner repair, open-existing fallback or implicit cleanup.
/// Synchronous native call requires an independent caller/host watchdog.
pub fn create_directory_with_descriptor(
    path: &Path,
    descriptor: &windows_permissions::SecurityDescriptor,
) -> Result<CreatedDirectory, BridgeError> {
    create::create(path, descriptor)
}

fn os_failure() -> BridgeError {
    // Capture immediately, before any other native operation; do not expose it.
    let _captured = std::io::Error::last_os_error();
    BridgeError::OsFailure
}

#[cfg(test)]
mod tests;

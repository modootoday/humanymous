//! New FILE creation only. No parent, provider, original-clock or Ready authority.
#![deny(unsafe_code)]

use super::{BridgeError, os_failure};
use std::{
    fs::File,
    mem::size_of,
    os::windows::{ffi::OsStrExt, io::{FromRawHandle, OwnedHandle}},
    path::{Path, PathBuf},
};
use windows_permissions::SecurityDescriptor;
use windows_sys::Win32::{
    Foundation::INVALID_HANDLE_VALUE,
    Security::SECURITY_ATTRIBUTES,
    Storage::FileSystem::{CREATE_NEW, CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_OPEN_REPARSE_POINT},
};

struct CreationObligation { path: PathBuf }

/// Owns the created File AND its creation obligation. Dropping closes the handle
/// without deleting the file or proving cleanup. Retain on validation failures.
/// No identity, admitted namespace, O, native proof or Ready is implied.
///
/// External callers cannot construct it from a path or arbitrary File:
/// ```compile_fail,E0451
/// use humanymous_platform::windows_object::create_file::CreatedFile;
/// let _forged = CreatedFile { file: todo!(), obligation: todo!() };
/// ```
/// No lone-File escape drops the creation obligation:
/// ```compile_fail,E0599
/// use humanymous_platform::windows_object::create_file::CreatedFile;
/// fn forbidden(owner: CreatedFile) { let _ = owner.into_file(); }
/// ```
#[must_use = "retain the created file and creation obligation through validation and explicit cleanup"]
pub struct CreatedFile {
    file: File,
    obligation: CreationObligation,
}
impl CreatedFile {
    /// Borrow the actual open File without separating its creation obligation.
    pub fn file(&self) -> &File { &self.file }
    /// Borrow the original locator, not a handle-relative namespace proof.
    pub fn path(&self) -> &Path { &self.obligation.path }
}

struct PreparedCreation<'a> {
    wide: Vec<u16>,
    obligation: CreationObligation,
    attributes: SECURITY_ATTRIBUTES,
    // The valid opaque native descriptor allocation must outlive the API call.
    _descriptor: &'a SecurityDescriptor,
}

fn ascii_upper(unit: u16) -> u16 {
    if (u16::from(b'a')..=u16::from(b'z')).contains(&unit) { unit - 32 } else { unit }
}

fn reserved(component: &[u16]) -> bool {
    let end = component.iter().position(|unit| *unit == u16::from(b'.')).unwrap_or(component.len());
    let mut stem = &component[..end];
    while stem.last() == Some(&u16::from(b' ')) { stem = &stem[..stem.len() - 1]; }
    let upper: Vec<u16> = stem.iter().copied().map(ascii_upper).collect();
    let equal = |name: &[u8]| upper.iter().copied().eq(name.iter().copied().map(u16::from));
    if [b"CON".as_slice(), b"PRN", b"AUX", b"NUL", b"CONIN$", b"CONOUT$", b"CLOCK$"].iter().any(|name| equal(name)) {
        return true;
    }
    if upper.len() != 4 { return false; }
    let prefix = &upper[..3];
    let port = prefix == [67, 79, 77] || prefix == [76, 80, 84];
    port && ((49..=57).contains(&upper[3]) || [0x00b9, 0x00b2, 0x00b3].contains(&upper[3]))
}

fn wide_locator(path: &Path) -> Result<Vec<u16>, BridgeError> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().take(32768).collect();
    if wide.len() < 4 || wide.len() >= 32768
        || !((65..=90).contains(&wide[0]) || (97..=122).contains(&wide[0]))
        || wide[1] != 58 || wide[2] != 92
    { return Err(BridgeError::InvalidInput); }
    // Parse original UTF16, not normalized Path::components (which hides dots).
    for component in wide[3..].split(|unit| *unit == 92) {
        if component.is_empty() || component.len() > 255
            || component == [46] || component == [46, 46]
            || component.last().is_some_and(|unit| *unit == 32 || *unit == 46)
            || component.iter().any(|unit| *unit < 32 || [34, 42, 47, 58, 60, 62, 63, 124].contains(unit))
            || reserved(component)
        { return Err(BridgeError::InvalidInput); }
    }
    wide.push(0);
    Ok(wide)
}

/// Atomically create a NEW file with this borrowed valid native descriptor AT
/// creation, share0 and no inherited handle. The caller enforces descriptor
/// policy, held ancestry/provider, original budget and independent outer watchdog.
/// Existing objects are refused; no open/truncate/retry/repair/delete fallback.
pub fn create_new_file_with_descriptor(path: &Path, descriptor: &SecurityDescriptor) -> Result<CreatedFile, BridgeError> {
    // All potentially allocating/fallible preparation precedes the native effect.
    let wide = wide_locator(path)?;
    let obligation = CreationObligation { path: path.to_owned() };
    let attributes = SECURITY_ATTRIBUTES {
        nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).map_err(|_| BridgeError::InvalidInput)?,
        lpSecurityDescriptor: std::ptr::from_ref(descriptor).cast::<std::ffi::c_void>().cast_mut(),
        bInheritHandle: 0,
    };
    create_native(PreparedCreation { wide, obligation, attributes, _descriptor: descriptor })
}

// Exact accepted seventh function; the existing six exceptions are untouched.
#[allow(unsafe_code)]
fn create_native(prepared: PreparedCreation<'_>) -> Result<CreatedFile, BridgeError> {
    // SAFETY: private prepared owns bounded UTF16 with exactly one terminator,
    // attributes/locator/obligation were allocated before entry, descriptor is a
    // borrowed valid pinned windows-permissions native allocation, not a layout
    // reinterpretation. All remain live through synchronous CreateFileW. Windows
    // reads the descriptor despite mutable ABI pointer; no mutable Rust alias.
    let raw = unsafe { CreateFileW(
        prepared.wide.as_ptr(), 0xc002_0000, 0, &prepared.attributes, CREATE_NEW,
        FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT, std::ptr::null_mut(),
    ) };
    if raw == INVALID_HANDLE_VALUE || raw.is_null() {
        return Err(os_failure());
    }
    // SAFETY: this is the exact newly returned owned valid file handle, adopted
    // ONCE. No fallible operation/allocation/logging/unwrap intervenes. File::from
    // transfers the same ownership safely; no extra CloseHandle call or raw getter.
    let owned = unsafe { OwnedHandle::from_raw_handle(raw) };
    Ok(CreatedFile { file: File::from(owned), obligation: prepared.obligation })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locator_rejects_devices_aliases_ads_and_traversal_before_creation() {
        for path in ["", "relative", r"C:relative", r"\rooted", r"\\server\share\file", r"\\?\C:\file", r"\\.\NUL",
            r"C:\a\..\b", r"C:\a\.\b", r"C:\a\\b", r"C:\a\", r"C:\a\file:stream", r"C:\a\NUL.txt",
            r"C:\a\con .txt", r"C:\a\COM1.bin", r"C:\a\LPT².txt", r"C:\a\CONOUT$", r"C:\a\tail.", r"C:\a\tail ",
            "C:\\a\\bad\0name", "C:/a/file"] {
            assert!(wide_locator(Path::new(path)).is_err(), "unsupported locator accepted");
        }
    }
    #[test]
    fn locator_preserves_legitimate_unicode_and_bounds_utf16_components() {
        for path in [r"C:\資料\검증-🙂.json", r"D:\δοκιμή\école.bin", r"C:\a\com10.json"] {
            let wide = wide_locator(Path::new(path)).unwrap();
            let expected: Vec<_> = path.encode_utf16().chain([0]).collect();
            assert_eq!(wide, expected);
        }
        assert!(wide_locator(Path::new(&format!("C:\\a\\{}", "x".repeat(255)))).is_ok());
        assert!(wide_locator(Path::new(&format!("C:\\a\\{}", "x".repeat(256)))).is_err());
    }
}

#[cfg(test)]
mod native_tests;

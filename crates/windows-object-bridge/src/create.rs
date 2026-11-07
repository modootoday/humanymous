use super::{BridgeError, CreatedDirectory, Path, os_failure};
use std::{mem::size_of, os::windows::ffi::OsStrExt};
use windows_sys::Win32::{Security::SECURITY_ATTRIBUTES, Storage::FileSystem::CreateDirectoryW};

pub(super) fn create(
    path: &Path,
    descriptor: &windows_permissions::SecurityDescriptor,
) -> Result<CreatedDirectory, BridgeError> {
    let wide = wide_path(path)?;
    let owned_path = path.to_owned();
    create_native(&wide, descriptor)?;
    // Creation obligation is returned before any fallible caller validation.
    Ok(CreatedDirectory { path: owned_path })
}

pub(super) fn wide_path(path: &Path) -> Result<Vec<u16>, BridgeError> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().take(32768).collect();
    if wide.is_empty() || wide.len() >= 32768 || wide.contains(&0) {
        return Err(BridgeError::InvalidInput);
    }
    wide.push(0);
    Ok(wide)
}

#[allow(unsafe_code)]
fn create_native(
    wide: &[u16],
    descriptor: &windows_permissions::SecurityDescriptor,
) -> Result<(), BridgeError> {
    let attributes = SECURITY_ATTRIBUTES {
        nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>())
            .map_err(|_| BridgeError::InvalidInput)?,
        lpSecurityDescriptor: std::ptr::from_ref(descriptor)
            .cast::<std::ffi::c_void>()
            .cast_mut(),
        bInheritHandle: 0,
    };
    // SAFETY: wide is owned NUL-terminated bounded UTF16 from private wide_path,
    // with no interior NUL. Generated attributes and descriptor allocation remain
    // live throughout this synchronous call. Descriptor is borrowed from pinned
    // windows-permissions' valid native LocalBox allocation; raw opaque pointer
    // borrow is NOT a typed layout reinterpretation or ownership conversion.
    // Windows reads (does not mutate/free) this descriptor despite mutable ABI
    // void pointer. No mutable Rust reference or foreign owning handle is created.
    let ok = unsafe { CreateDirectoryW(wide.as_ptr(), &attributes) };
    if ok == 0 {
        return Err(os_failure());
    }
    Ok(())
}

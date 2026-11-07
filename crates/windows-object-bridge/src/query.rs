use super::{BridgeError, File, OpenedObjectObservation, os_failure};
use std::{
    mem::size_of,
    os::windows::io::{AsHandle, AsRawHandle},
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_ID_INFO,
    FILE_NAME_NORMALIZED, FILE_TYPE_DISK, FileAttributeTagInfo, FileIdInfo,
    GetFileInformationByHandleEx, GetFileType, GetFinalPathNameByHandleW,
    GetVolumeInformationByHandleW, VOLUME_NAME_GUID,
};

// Documented filesystem capability bit. windows-sys exposes this value only in
// SystemServices; no new feature/dependency is needed for this query output bit.
const FILE_PERSISTENT_ACLS: u32 = 0x0000_0008;

pub(super) fn inspect(file: &File) -> Result<OpenedObjectObservation, BridgeError> {
    disk(file)?;
    let identity = identity(file)?;
    let attributes = attributes(file)?;
    if attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(BridgeError::InvalidObject);
    }
    filesystem(file)?;
    let path = final_path(file)?;
    let volume_path = volume_prefix(&path)?.to_owned();
    if identity.VolumeSerialNumber == 0 || identity.FileId.Identifier.iter().all(|b| *b == 0) {
        return Err(BridgeError::InvalidObject);
    }
    let mut wire = [0_u8; 24];
    wire[..8].copy_from_slice(&identity.VolumeSerialNumber.to_le_bytes());
    wire[8..].copy_from_slice(&identity.FileId.Identifier);
    Ok(OpenedObjectObservation {
        identity: wire,
        volume_path,
        directory: attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0,
    })
}

#[allow(unsafe_code)]
fn disk(file: &File) -> Result<(), BridgeError> {
    // SAFETY: borrowed File stays live for this synchronous query; no handle
    // ownership transfer or conversion into a new owner. No output buffer.
    let kind = unsafe { GetFileType(file.as_handle().as_raw_handle()) };
    if kind == 0 {
        return Err(os_failure());
    }
    if kind != FILE_TYPE_DISK {
        return Err(BridgeError::InvalidObject);
    }
    Ok(())
}

#[allow(unsafe_code)]
fn identity(file: &File) -> Result<FILE_ID_INFO, BridgeError> {
    let mut info = FILE_ID_INFO::default();
    let size = u32::try_from(size_of::<FILE_ID_INFO>()).map_err(|_| BridgeError::InvalidInput)?;
    // SAFETY: generated POD initialized/aligned output, exact checked byte size,
    // mutable output live for full synchronous call; same borrowed File stays live.
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_handle().as_raw_handle(),
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast(),
            size,
        )
    };
    if ok == 0 {
        return Err(os_failure());
    }
    Ok(info)
}

#[allow(unsafe_code)]
fn attributes(file: &File) -> Result<FILE_ATTRIBUTE_TAG_INFO, BridgeError> {
    let mut info = FILE_ATTRIBUTE_TAG_INFO::default();
    let size = u32::try_from(size_of::<FILE_ATTRIBUTE_TAG_INFO>())
        .map_err(|_| BridgeError::InvalidInput)?;
    // SAFETY: generated POD initialized/aligned output with exact checked size;
    // File borrow and output remain live throughout; false output is never read.
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_handle().as_raw_handle(),
            FileAttributeTagInfo,
            (&mut info as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
            size,
        )
    };
    if ok == 0 {
        return Err(os_failure());
    }
    Ok(info)
}

#[allow(unsafe_code)]
fn filesystem(file: &File) -> Result<(), BridgeError> {
    let mut name = [0_u16; 32];
    let mut flags = 0_u32;
    // SAFETY: same live borrowed File; bounded initialized UTF16 array and scalar
    // flags are writable for entire call. Optional unrequested outputs are NULL.
    let ok = unsafe {
        GetVolumeInformationByHandleW(
            file.as_handle().as_raw_handle(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut flags,
            name.as_mut_ptr(),
            name.len() as u32,
        )
    };
    if ok == 0 {
        return Err(os_failure());
    }
    let end = name
        .iter()
        .position(|c| *c == 0)
        .ok_or(BridgeError::InvalidObject)?;
    if name[..end] != [b'N' as u16, b'T' as u16, b'F' as u16, b'S' as u16]
        || flags & FILE_PERSISTENT_ACLS == 0
    {
        return Err(BridgeError::UnsupportedProvider);
    }
    Ok(())
}

const PATH_CAP: usize = 32768;
#[allow(unsafe_code)]
fn final_path(file: &File) -> Result<String, BridgeError> {
    let mut capacity = 256_usize;
    for _ in 0..3 {
        let mut buffer = vec![0_u16; capacity];
        let count = u32::try_from(capacity).map_err(|_| BridgeError::InvalidInput)?;
        // SAFETY: same borrowed File and owned initialized UTF16 storage remain
        // live; checked length is in WCHARs. Interpret length before any slicing.
        let length = unsafe {
            GetFinalPathNameByHandleW(
                file.as_handle().as_raw_handle(),
                buffer.as_mut_ptr(),
                count,
                VOLUME_NAME_GUID | FILE_NAME_NORMALIZED,
            )
        };
        if length == 0 {
            return Err(os_failure());
        }
        let length = usize::try_from(length).map_err(|_| BridgeError::InvalidObject)?;
        if length < capacity {
            if buffer[length] != 0 {
                return Err(BridgeError::InvalidObject);
            }
            return String::from_utf16(&buffer[..length]).map_err(|_| BridgeError::InvalidObject);
        }
        capacity = next_capacity(capacity, length)?;
    }
    Err(BridgeError::UnsupportedProvider)
}

fn next_capacity(current: usize, required: usize) -> Result<usize, BridgeError> {
    let next = if required > current {
        required
    } else {
        current.checked_add(1).ok_or(BridgeError::InvalidInput)?
    };
    if next > PATH_CAP {
        return Err(BridgeError::UnsupportedProvider);
    }
    Ok(next)
}

pub(super) fn volume_prefix(path: &str) -> Result<&str, BridgeError> {
    let guid = path
        .strip_prefix(r"\\?\Volume{")
        .ok_or(BridgeError::UnsupportedProvider)?;
    let bytes = guid.as_bytes();
    if bytes.len() < 38 || &bytes[36..38] != b"}\\" {
        return Err(BridgeError::UnsupportedProvider);
    }
    for (index, byte) in bytes[..36].iter().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            if *byte != b'-' {
                return Err(BridgeError::UnsupportedProvider);
            }
        } else if !byte.is_ascii_hexdigit() {
            return Err(BridgeError::UnsupportedProvider);
        }
    }
    // Prefix is ASCII through the validated closing separator, hence a UTF8 boundary.
    Ok(&path[..path.len() - guid.len() + 38])
}

#[cfg(test)]
pub(super) fn check_capacity(current: usize, required: usize) -> Result<usize, BridgeError> {
    next_capacity(current, required)
}

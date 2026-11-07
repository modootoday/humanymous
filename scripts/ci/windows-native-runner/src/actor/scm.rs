//! Temporary single-SID SCM permission delegation on a disposable VM only.
//! SAFETY: native handles have one closer. Query buffers are u64-aligned, bounded
//! by the documented8KiB descriptor budget, and borrowed only after a successful
//! Win32 query. Parsed descriptors own their native allocations until set returns.
#![allow(unsafe_code)]
use sha2::{Digest, Sha256};
use std::{io, ptr::null};
use windows_permissions::{LocalBox, SecurityDescriptor, constants::SecurityInformation, wrappers};
use windows_sys::Win32::{Security::DACL_SECURITY_INFORMATION, System::Services::*};

struct Manager(SC_HANDLE);
impl Drop for Manager {
    fn drop(&mut self) {
        unsafe {
            CloseServiceHandle(self.0);
        }
    }
}
fn descriptor(manager: &Manager) -> io::Result<Vec<u64>> {
    let mut needed = 0;
    unsafe {
        QueryServiceObjectSecurity(
            manager.0,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            0,
            &mut needed,
        );
    }
    if !(20..=8192).contains(&needed) {
        return Err(io::Error::other("SCM descriptor budget unavailable"));
    }
    let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
    let capacity = (buffer.len() * 8) as u32;
    if unsafe {
        QueryServiceObjectSecurity(
            manager.0,
            DACL_SECURITY_INFORMATION,
            buffer.as_mut_ptr().cast(),
            capacity,
            &mut needed,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    if needed > capacity {
        return Err(io::Error::other("SCM descriptor changed size"));
    }
    Ok(buffer)
}
fn borrowed(buffer: &[u64]) -> &SecurityDescriptor {
    // Caller only supplies the successful bounded OS query buffer above.
    unsafe { &*buffer.as_ptr().cast::<SecurityDescriptor>() }
}
fn text(descriptor: &SecurityDescriptor) -> io::Result<String> {
    wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
        descriptor,
        SecurityInformation::Dacl,
    )?
    .into_string()
    .map_err(|_| io::Error::other("non-Unicode SCM descriptor"))
}
fn extended(original: &str, sid: &str) -> io::Result<String> {
    let sid: LocalBox<windows_permissions::Sid> = sid.parse()?;
    let sid = sid.to_string();
    if !original.starts_with("D:")
        || !original.ends_with(')')
        || original.contains(&format!(";;;{sid})"))
    {
        return Err(io::Error::other("unexpected SCM trustee inventory"));
    }
    Ok(format!("{original}(A;;0x3;;;{sid})"))
}
pub(super) struct Delegation {
    manager: Manager,
    original: Vec<u64>,
    original_text: String,
    granted_text: String,
    restored: bool,
}
impl Delegation {
    pub(super) fn grant(sid: &str) -> io::Result<Self> {
        let handle = unsafe { OpenSCManagerW(null(), null(), 0x60000 | SC_MANAGER_CONNECT) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let manager = Manager(handle);
        let original = descriptor(&manager)?;
        let sd = borrowed(&original);
        let acl = sd
            .dacl()
            .ok_or_else(|| io::Error::other("SCM null DACL refused"))?;
        if acl.len() == 0
            || (0..acl.len()).any(|i| {
                acl.get_ace(i)
                    .is_none_or(|ace| ace.flags().bits() & 0x10 != 0)
            })
        {
            return Err(io::Error::other(
                "SCM inherited/unknown DACL requires separate qualification",
            ));
        }
        let original_text = text(sd)?;
        let granted: LocalBox<SecurityDescriptor> = extended(&original_text, sid)?.parse()?;
        let granted_text = text(&granted)?;
        let mut guard = Self {
            manager,
            original,
            original_text,
            granted_text,
            restored: false,
        };
        if unsafe {
            SetServiceObjectSecurity(
                guard.manager.0,
                DACL_SECURITY_INFORMATION,
                (&*granted as *const SecurityDescriptor).cast_mut().cast(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if text(borrowed(&descriptor(&guard.manager)?))? != guard.granted_text {
            guard.restore()?;
            return Err(io::Error::other("SCM grant readback mismatch"));
        }
        Ok(guard)
    }
    pub(super) fn restore(&mut self) -> io::Result<()> {
        if self.restored {
            return Ok(());
        }
        let current = text(borrowed(&descriptor(&self.manager)?))?;
        if current == self.original_text {
            self.restored = true;
            return Ok(());
        }
        if current != self.granted_text {
            return Err(io::Error::other(
                "SCM descriptor changed concurrently; unknown changes not overwritten",
            ));
        }
        if unsafe {
            SetServiceObjectSecurity(
                self.manager.0,
                DACL_SECURITY_INFORMATION,
                self.original.as_mut_ptr().cast(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if text(borrowed(&descriptor(&self.manager)?))? != self.original_text {
            return Err(io::Error::other("SCM restoration readback mismatch"));
        }
        self.restored = true;
        Ok(())
    }
    pub(super) fn original_sha256(&self) -> String {
        format!("{:x}", Sha256::digest(self.original_text.as_bytes()))
    }
}
impl Drop for Delegation {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grant_is_exact_connect_create_only_and_never_replaces_existing_aces() {
        let sid = "S-1-5-21-1-2-3-1000";
        let original = "D:(A;;GA;;;SY)(A;;GA;;;BA)";
        let result = extended(original, sid).unwrap();
        assert_eq!(result, format!("{original}(A;;0x3;;;{sid})"));
        assert!(extended(&result, sid).is_err());
        assert!(extended("D:", sid).is_err());
        let sd: LocalBox<SecurityDescriptor> = result.parse().unwrap();
        assert_eq!(sd.dacl().unwrap().len(), 3);
        assert_eq!(sd.dacl().unwrap().get_ace(2).unwrap().mask().bits(), 3);
    }
}

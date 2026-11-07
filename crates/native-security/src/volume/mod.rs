//! Native volume classification, not a claim of physical media provenance.
use std::io;
use winsafe::co;

/// Refuse unknown, remote, removable, optical and RAM volumes. Callers supply
/// an explicit drive-letter root including its trailing backslash.
pub fn require_fixed(root: &str) -> io::Result<()> {
    let bytes = root.as_bytes();
    if bytes.len() != 3 || !bytes[0].is_ascii_alphabetic() || &bytes[1..] != b":\\" {
        return Err(io::Error::other("explicit drive-letter root required"));
    }
    if fixed(winsafe::GetDriveType(Some(root))) {
        Ok(())
    } else {
        Err(io::Error::other("local fixed drive required"))
    }
}
fn fixed(kind: co::DRIVE) -> bool {
    kind == co::DRIVE::FIXED
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_fixed_is_eligible_and_ambiguous_roots_are_refused() {
        assert!(fixed(co::DRIVE::FIXED));
        for kind in [
            co::DRIVE::UNKNOWN,
            co::DRIVE::NO_ROOT_DIR,
            co::DRIVE::REMOVABLE,
            co::DRIVE::REMOTE,
            co::DRIVE::CDROM,
            co::DRIVE::RAMDISK,
        ] {
            assert!(!fixed(kind));
        }
        for root in [
            "",
            "C:",
            "C:/",
            "\\\\server\\share\\",
            "\\\\?\\C:\\",
            "C:\\child",
        ] {
            assert!(require_fixed(root).is_err());
        }
    }
}

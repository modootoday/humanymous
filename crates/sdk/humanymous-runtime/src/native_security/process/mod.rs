//! Actual process identity rather than caller-provided ownership assertions.
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{WindowsIdentity, current_identity};

pub(in crate::native_security) fn require_owned_service_name(name: &str) -> std::io::Result<()> {
    let valid = name.strip_prefix("humanymous-lab-").is_some_and(|tail| {
        !tail.is_empty()
            && tail.len() <= 48
            && tail
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    });
    if !valid {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid owned service name",
        ));
    }
    Ok(())
}

/// Effective native Unix identity; no shell command or environment lookup.
#[cfg(unix)]
pub fn current_uid() -> u32 {
    rustix::process::geteuid().as_raw()
}

#[cfg(any(windows, test))]
fn installer_token_allowed(elevated: bool, enabled_administrator: bool) -> bool {
    !elevated && !enabled_administrator
}
#[cfg(any(windows, test))]
fn enabled_admin_group(sid: &str, attributes: u32) -> bool {
    sid == "S-1-5-32-544" && attributes & 0x00000004 != 0
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deny_only_groups_do_not_grant_enabled_admin_authority() {
        assert!(enabled_admin_group("S-1-5-32-544", 4));
        assert!(enabled_admin_group("S-1-5-32-544", 4 | 16));
        for (sid, flags) in [
            ("S-1-5-32-544", 16),
            ("S-1-5-32-544", 0),
            ("S-1-5-21-1-2-3-1000", 4),
        ] {
            assert!(!enabled_admin_group(sid, flags));
        }
    }
    #[test]
    fn installation_requires_both_non_elevation_and_no_enabled_admin() {
        assert!(installer_token_allowed(false, false));
        for (elevated, admin) in [(true, false), (false, true), (true, true)] {
            assert!(!installer_token_allowed(elevated, admin));
        }
    }
}

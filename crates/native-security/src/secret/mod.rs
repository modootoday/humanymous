//! Read-only unattended service credentials. No plaintext or interactive fallback.
#[cfg(feature = "credential-pilot")]
pub mod pilot;
mod reference;
#[cfg(windows)]
mod windows;
pub use reference::ServiceSecretReference;
use std::io;
use zeroize::Zeroizing;

#[cfg(any(windows, test))]
fn unavailable() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "protected service credential unavailable",
    )
}
#[cfg(any(windows, test))]
fn validate_secret(secret: &[u8]) -> io::Result<()> {
    if secret.is_empty() || secret.len() > 4096 {
        return Err(unavailable());
    }
    Ok(())
}

/// Read one existing secret for the actual virtual service principal. Never
/// creates credentials, opens a UI, logs a backend error, or consults argv/env/files.
/// Non-Windows backends are explicitly unsupported in this implementation.
pub fn read_service_secret(reference: &ServiceSecretReference) -> io::Result<Zeroizing<Vec<u8>>> {
    #[cfg(windows)]
    {
        windows::read(reference)
    }
    #[cfg(not(windows))]
    {
        let _ = reference;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "protected service credential backend unsupported",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_and_oversize_secrets_fail_without_echoing_bytes() {
        assert!(validate_secret(b"fixture").is_ok());
        for bytes in [Vec::new(), vec![b'x'; 4097]] {
            let error = validate_secret(&bytes).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
            assert_eq!(
                error.to_string(),
                "protected service credential unavailable"
            );
        }
    }
    #[cfg(not(windows))]
    #[test]
    fn unimplemented_backend_is_explicitly_unsupported() {
        let reference = ServiceSecretReference::new("humanymous-lab-fixture", "audit").unwrap();
        assert_eq!(
            read_service_secret(&reference).unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
    }
}

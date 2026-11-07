//! Disposable credential fixture. WinCred has no atomic create-only/CAS-delete.
//! Require unique random targets and an exclusive actual service-principal fixture.
use super::ServiceSecretReference;
use std::io;
use zeroize::Zeroizing;

/// Caller-validated nonsecret fixture metadata, not hosted-runner attestation.
/// CLI must authenticate its signed fixture plan; external run/source/actor
/// evidence establishes the hosted execution. No CI environment inheritance assumed.
#[derive(Clone)]
pub struct DisposableServiceContext {
    service_name: String,
    run_id: u64,
    source_sha: String,
}
impl DisposableServiceContext {
    /// Validate syntax and bind one service, without claiming to verify a signature.
    pub fn new(service_name: &str, run_id: u64, source_sha: &str) -> io::Result<Self> {
        crate::native_security::process::require_owned_service_name(service_name)?;
        if run_id == 0
            || !matches!(source_sha.len(), 40 | 64)
            || !source_sha
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid disposable service context",
            ));
        }
        Ok(Self {
            service_name: service_name.to_owned(),
            run_id,
            source_sha: source_sha.to_owned(),
        })
    }
    /// Nonsecret caller-provided run identifier; not an independently verified fact.
    pub fn run_id(&self) -> u64 {
        self.run_id
    }
    /// Nonsecret caller-provided source identifier; external evidence must bind it.
    pub fn source_sha(&self) -> &str {
        &self.source_sha
    }
}
fn fixture_id(id: &str) -> bool {
    id.strip_prefix("pilot-").is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// Owns only a uniquely named disposable fixture entry. No Debug or secret accessor.
/// Call finish to observe cleanup; Drop is best effort and is not success evidence.
pub struct ServiceSecretFixture {
    #[cfg(windows)]
    reference: ServiceSecretReference,
    #[cfg(windows)]
    expected: Zeroizing<Vec<u8>>,
    completed: bool,
}
impl ServiceSecretFixture {
    /// Observe absence, then create. Precheck/write is not atomic against another
    /// same-principal writer. Existing/unknown entries are rejected without repair.
    pub fn create(
        reference: &ServiceSecretReference,
        secret: Zeroizing<Vec<u8>>,
        context: &DisposableServiceContext,
    ) -> io::Result<Self> {
        if context.service_name != reference.service_name()
            || !fixture_id(reference.credential_id())
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "explicit matching disposable service fixture required",
            ));
        }
        #[cfg(windows)]
        {
            super::validate_secret(&secret)?;
            let entry = super::windows::entry(reference)?;
            match entry.get_attributes() {
                Err(keyring_core::Error::NoEntry) => (),
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "service fixture credential already exists",
                    ));
                }
                Err(_) => return Err(super::unavailable()),
            }
            entry
                .set_secret(&secret)
                .map_err(|_| super::unavailable())?;
            let fixture = Self {
                reference: reference.clone(),
                expected: secret,
                completed: false,
            };
            let actual = super::read_service_secret(reference)?;
            if actual.as_slice() != fixture.expected.as_slice() {
                return Err(super::unavailable());
            }
            Ok(fixture)
        }
        #[cfg(not(windows))]
        {
            let _ = secret;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "service credential fixture unsupported",
            ))
        }
    }
    /// Check exact metadata and secret before delete, then observe absence. Not CAS.
    /// On mismatch/unknown state, leave the credential untouched and report failure.
    pub fn finish(mut self) -> io::Result<()> {
        let result = self.cleanup();
        self.completed = true; // A failed explicit cleanup must not be hidden by retry.
        result
    }
    fn cleanup(&self) -> io::Result<()> {
        #[cfg(windows)]
        {
            let entry = super::windows::entry(&self.reference)?;
            let target = self.reference.target();
            let attributes = match entry.get_attributes() {
                Err(keyring_core::Error::NoEntry) => return Ok(()),
                Ok(value) => value,
                Err(_) => return Err(super::unavailable()),
            };
            if !super::windows::attributes_match(&attributes, &target) {
                return Err(super::unavailable());
            }
            let actual = super::read_service_secret(&self.reference)?;
            if actual.as_slice() != self.expected.as_slice() {
                return Err(super::unavailable());
            }
            entry
                .delete_credential()
                .map_err(|_| super::unavailable())?;
            match entry.get_attributes() {
                Err(keyring_core::Error::NoEntry) => Ok(()),
                _ => Err(super::unavailable()),
            }
        }
        #[cfg(not(windows))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "service credential fixture unsupported",
            ))
        }
    }
}
impl Drop for ServiceSecretFixture {
    fn drop(&mut self) {
        if !self.completed {
            let _ = self.cleanup();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_and_random_target_syntax_are_explicit_not_hosted_attestation() {
        let context =
            DisposableServiceContext::new("humanymous-lab-test", 1, &"a".repeat(40)).unwrap();
        assert_eq!(context.run_id(), 1);
        assert_eq!(context.source_sha(), "a".repeat(40));
        assert!(DisposableServiceContext::new("humanymous-lab-test", 1, &"b".repeat(64)).is_ok());
        assert!(DisposableServiceContext::new("humanymous-lab-test", 0, &"a".repeat(40)).is_err());
        assert!(DisposableServiceContext::new("other", 1, &"a".repeat(40)).is_err());
        assert!(DisposableServiceContext::new("humanymous-lab-test", 1, "bad").is_err());
        assert!(fixture_id(&format!("pilot-{}", "a".repeat(32))));
        for id in ["audit", "pilot-a", "pilot-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"] {
            assert!(!fixture_id(id));
        }
    }
}

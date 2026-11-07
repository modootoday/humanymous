use std::{fmt, io};

/// A nonsecret selector. It never contains credential bytes or a fallback path.
#[derive(Clone)]
pub struct ServiceSecretReference {
    service_name: String,
    credential_id: String,
}
impl ServiceSecretReference {
    /// Restrict lookups to the existing owned service namespace and one exact id.
    pub fn new(service_name: &str, credential_id: &str) -> io::Result<Self> {
        crate::native_security::process::require_owned_service_name(service_name)?;
        if credential_id.is_empty()
            || credential_id.len() > 64
            || !credential_id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid service credential selector",
            ));
        }
        Ok(Self {
            service_name: service_name.to_owned(),
            credential_id: credential_id.to_owned(),
        })
    }
    /// The owned service whose actual token principal must be checked by the provider.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }
    /// Exact nonsecret credential identifier, without wildcard expansion.
    pub fn credential_id(&self) -> &str {
        &self.credential_id
    }
    #[cfg(any(windows, test))]
    pub(super) fn target(&self) -> String {
        format!(
            "humanymous/service/{}/{}",
            self.service_name, self.credential_id
        )
    }
}
impl fmt::Debug for ServiceSecretReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ServiceSecretReference([redacted selector])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selector_is_exact_owned_bounded_and_has_no_path_or_ui_fallback() {
        let good = ServiceSecretReference::new("humanymous-lab-fixture", "audit-key-1").unwrap();
        assert_eq!(
            good.target(),
            "humanymous/service/humanymous-lab-fixture/audit-key-1"
        );
        assert!(!format!("{good:?}").contains("audit-key-1"));
        for service in [
            "other-fixture",
            "humanymous-lab-",
            "humanymous-lab-A",
            "humanymous-lab-a/b",
        ] {
            assert!(ServiceSecretReference::new(service, "audit").is_err());
        }
        for id in [
            "",
            "*",
            "a/b",
            "a\\b",
            "a\0b",
            "A",
            " audit",
            "secret.txt",
            "key\n",
        ] {
            assert!(ServiceSecretReference::new("humanymous-lab-fixture", id).is_err());
        }
        assert!(ServiceSecretReference::new("humanymous-lab-fixture", &"a".repeat(65)).is_err());
    }
}

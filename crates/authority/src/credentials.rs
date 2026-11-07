//! Process-lifetime credentials; restart cannot reuse a previous replay epoch.
use crate::Rejection;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// Secret bearer value. Intentionally has no Debug/Serialize implementation.
pub struct Credential(Zeroizing<String>);
impl Credential {
    /// Generate 256 independent OS-random bits, represented as base64url ASCII.
    pub fn generate() -> Result<Self, Rejection> {
        let mut bytes = Zeroizing::new([0u8; 32]);
        getrandom::fill(&mut bytes[..]).map_err(|_| Rejection::new(503, "entropy_unavailable"))?;
        // Hex is a subset of base64url, so 64 ASCII characters satisfy the wire
        // token grammar without introducing another encoding implementation.
        Self::from_private_provisioning(Zeroizing::new(hex::encode(&bytes[..])))
    }
    /// Take a supervisor-owned secret without cloning it. Only the canonical
    /// generated grammar (64 lowercase hexadecimal bytes) is accepted. This
    /// validates representation, not entropy, scope, role or supervisor identity.
    /// Callers must provision those independently and use a fresh epoch on restart.
    pub fn from_private_provisioning(value: Zeroizing<String>) -> Result<Self, Rejection> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Rejection::new(503, "invalid_private_credential"));
        }
        Ok(Self(value))
    }
    /// Deliver exactly once through the supervisor's private readiness pipe.
    pub fn expose_for_provisioning(&self) -> &str {
        &self.0
    }
    /// Validate an exact Bearer scheme; compare only fixed-length token bytes.
    pub fn authenticates(&self, authorization: &str) -> bool {
        let Some(token) = authorization.strip_prefix("Bearer ") else {
            return false;
        };
        token.len() == self.0.len() && bool::from(token.as_bytes().ct_eq(self.0.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_epochs_and_exact_scheme() {
        let a = Credential::generate().unwrap();
        let b = Credential::generate().unwrap();
        assert_ne!(a.expose_for_provisioning(), b.expose_for_provisioning());
        assert!(a.authenticates(&format!("Bearer {}", a.expose_for_provisioning())));
        assert!(!b.authenticates(&format!("Bearer {}", a.expose_for_provisioning())));
        for s in ["", "Bearer", "bearer x", "Bearer x", "Basic x"] {
            assert!(!a.authenticates(s));
        }
    }

    #[test]
    fn private_provisioning_rejects_noncanonical_secrets_and_exact_bearer_mutations() {
        let token = "0123456789abcdef".repeat(4);
        let credential =
            Credential::from_private_provisioning(Zeroizing::new(token.clone())).unwrap();
        assert!(credential.authenticates(&format!("Bearer {token}")));
        for presented in [
            format!("bearer {token}"),
            format!("Bearer  {token}"),
            format!("Bearer {token} "),
            format!("Bearer {}", "f".repeat(64)),
        ] {
            assert!(!credential.authenticates(&presented));
        }
        for invalid in [
            String::new(),
            "a".repeat(63),
            "a".repeat(65),
            "A".repeat(64),
            "g".repeat(64),
            format!("{}\n", "a".repeat(63)),
            format!("{} ", "a".repeat(63)),
            format!("{}\0", "a".repeat(63)),
        ] {
            let error = Credential::from_private_provisioning(Zeroizing::new(invalid))
                .err()
                .unwrap();
            assert_eq!(error, Rejection::new(503, "invalid_private_credential"));
        }
    }
}

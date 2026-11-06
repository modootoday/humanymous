//! Process-lifetime credentials; restart cannot reuse a previous replay epoch.
use crate::Rejection;
use subtle::ConstantTimeEq;

/// Secret bearer value. Intentionally has no Debug/Serialize implementation.
pub struct Credential(String);
impl Credential {
    /// Generate 256 independent OS-random bits, represented as base64url ASCII.
    pub fn generate() -> Result<Self, Rejection> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| Rejection::new(503, "entropy_unavailable"))?;
        // Hex is a subset of base64url, so 64 ASCII characters satisfy the wire
        // token grammar without introducing another encoding implementation.
        Ok(Self(hex::encode(bytes)))
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
        assert_ne!(a.0, b.0);
        assert!(a.authenticates(&format!("Bearer {}", a.0)));
        assert!(!b.authenticates(&format!("Bearer {}", a.0)));
        for s in ["", "Bearer", "bearer x", "Bearer x", "Basic x"] {
            assert!(!a.authenticates(s));
        }
    }
}

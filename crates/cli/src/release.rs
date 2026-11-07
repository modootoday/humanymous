//! Offline pinned-key verification of exact release-descriptor bytes.
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io, net::SocketAddr, path::Path};

const DOMAIN: &[u8] = b"humanymous.release.v1\0";
/// Maximum descriptor size, including whitespace.
pub const MAX_DESCRIPTOR: usize = 16_384;

pub(crate) fn error(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

/// Signed non-secret execution and diagnostic-readiness contract.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    /// Exact format discriminator.
    pub schema: String,
    /// Human-readable release identifier; never interpreted as a path.
    pub version: String,
    /// Monotonic installation-local release sequence.
    pub sequence: u64,
    /// Rust OS name.
    pub os: String,
    /// Rust architecture name.
    pub arch: String,
    /// Lowercase SHA256 of the executable.
    pub artifact_sha256: String,
    /// Owned lab service name; no arbitrary host service names.
    pub service_name: String,
    /// Native argument tokens, without a shell or secret material.
    pub arguments: Vec<String>,
    /// Loopback-only diagnostic endpoint; not a management credential.
    pub readiness: SocketAddr,
    /// Exact bounded diagnostic response, not product qualification.
    pub ready_response: String,
}

impl Descriptor {
    fn validate(&self) -> io::Result<()> {
        let suffix = self
            .service_name
            .strip_prefix("humanymous-lab-")
            .unwrap_or("");
        if self.schema != "humanymous.release.v1"
            || self.sequence == 0
            || self.version.is_empty()
            || self.version.len() > 64
            || self.version.chars().any(char::is_control)
            || self.os != std::env::consts::OS
            || self.arch != std::env::consts::ARCH
            || !matches!(self.os.as_str(), "windows" | "linux" | "macos")
            || suffix.is_empty()
            || suffix.len() > 48
            || !suffix
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || self.artifact_sha256.len() != 64
            || !self
                .artifact_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.arguments.len() > 32
            || self
                .arguments
                .iter()
                .any(|s| s.len() > 1024 || s.chars().any(char::is_control))
            || !self.readiness.ip().is_loopback()
            || self.readiness.port() == 0
            || self.ready_response.is_empty()
            || self.ready_response.len() > 512
            || self.ready_response.chars().any(char::is_control)
        {
            return Err(error("unsupported or invalid signed release descriptor"));
        }
        Ok(())
    }
}

/// Exact signed envelope; serialized in private deployment state for rollback.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SignedRelease {
    /// Exact descriptor UTF-8, without normalization.
    pub document: String,
    /// Detached strict Ed25519 signature in lowercase hex.
    pub signature: String,
}

impl SignedRelease {
    /// Read a bounded regular local descriptor. No writes, locks or network.
    pub fn read(path: &Path, signature: String, trust_key: &[u8; 32]) -> io::Result<Self> {
        crate::installation::permissions::plain(path)?;
        let metadata = std::fs::symlink_metadata(path)?;
        if !metadata.is_file() || metadata.len() > MAX_DESCRIPTOR as u64 {
            return Err(error("descriptor must be a bounded regular file"));
        }
        let release = Self {
            document: std::fs::read_to_string(path)?,
            signature,
        };
        release.verify(trust_key)?;
        Ok(release)
    }

    /// Verify signature before interpreting schema. Duplicate/unknown fields fail.
    pub fn verify(&self, trust_key: &[u8; 32]) -> io::Result<Descriptor> {
        if self.document.len() > MAX_DESCRIPTOR
            || self.signature.len() != 128
            || !lower_hex(&self.signature)
        {
            return Err(error("signed envelope budget"));
        }
        let key = VerifyingKey::from_bytes(trust_key).map_err(|_| error("invalid pinned key"))?;
        let bytes = hex::decode(&self.signature).map_err(|_| error("signature hex"))?;
        let signature = Signature::from_slice(&bytes).map_err(|_| error("signature width"))?;
        key.verify_strict(&signing_bytes(self.document.as_bytes()), &signature)
            .map_err(|_| error("release signature invalid"))?;
        let descriptor: Descriptor = serde_json::from_str(&self.document)
            .map_err(|e| error(format!("release schema: {e}")))?;
        descriptor.validate()?;
        Ok(descriptor)
    }
}

/// Exact domain-separated signature input for an offline release signer.
pub fn signing_bytes(document: &[u8]) -> Vec<u8> {
    [DOMAIN, document].concat()
}

/// Parse an independently pinned key. Keys are never fetched from the descriptor.
pub fn parse_trust_key(text: &str) -> io::Result<[u8; 32]> {
    if text.len() != 64 || !lower_hex(text) {
        return Err(error("canonical lowercase trust key hex required"));
    }
    hex::decode(text)
        .map_err(|_| error("trust key hex"))?
        .try_into()
        .map_err(|_| error("trust key width"))
}
fn lower_hex(text: &str) -> bool {
    text.bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    pub(crate) fn descriptor(sequence: u64) -> Descriptor {
        Descriptor {
            schema: "humanymous.release.v1".into(),
            version: format!("fixture-{sequence}"),
            sequence,
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            artifact_sha256: digest(b"fixture"),
            service_name: "humanymous-lab-unit".into(),
            arguments: vec![],
            readiness: "127.0.0.1:19999".parse().unwrap(),
            ready_response: format!("ready-{sequence}"),
        }
    }
    pub(crate) fn signed(d: &Descriptor) -> (SignedRelease, [u8; 32]) {
        let key = SigningKey::from_bytes(&[29; 32]);
        let document = serde_json::to_string(d).unwrap();
        let signature = hex::encode(key.sign(&signing_bytes(document.as_bytes())).to_bytes());
        (
            SignedRelease {
                document,
                signature,
            },
            key.verifying_key().to_bytes(),
        )
    }
    #[test]
    fn signature_binds_exact_bytes_key_target_and_domain() {
        let (mut release, key) = signed(&descriptor(1));
        assert!(release.verify(&key).is_ok());
        assert!(
            release
                .verify(&SigningKey::from_bytes(&[30; 32]).verifying_key().to_bytes())
                .is_err()
        );
        release.document.push(' ');
        assert!(release.verify(&key).is_err());
        let mut d = descriptor(1);
        d.arch = "wrong-architecture".into();
        let (release, key) = signed(&d);
        assert!(release.verify(&key).is_err());
    }
    #[test]
    fn signed_invalid_schema_never_becomes_authority() {
        for field in ["unknown", "sequence"] {
            let (mut release, key) = signed(&descriptor(1));
            release.document.pop();
            release.document.push_str(&format!(",\"{field}\":1}}"));
            let signer = SigningKey::from_bytes(&[29; 32]);
            release.signature = hex::encode(
                signer
                    .sign(&signing_bytes(release.document.as_bytes()))
                    .to_bytes(),
            );
            assert!(release.verify(&key).is_err());
        }
        let mut d = descriptor(0);
        d.readiness = "192.0.2.1:9".parse().unwrap();
        let (release, key) = signed(&d);
        assert!(release.verify(&key).is_err());
        d = descriptor(1);
        d.service_name = "unrelated-service".into();
        let (release, key) = signed(&d);
        assert!(release.verify(&key).is_err());
    }
}

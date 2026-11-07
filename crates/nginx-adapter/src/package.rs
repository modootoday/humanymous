//! Bounded module provenance. This is not a signature or an ABI-load permit.
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read};
use std::path::Path;

/// The initial supported build family; qualification needs a separate receipt.
pub const TARGET: &str = "x86_64-unknown-linux-gnu";
/// Exact upstream SDK source used by the module manifest.
pub const SDK_REV: &str = "ebbcd514ee7a336fa4b622232170d8a16763a9ca";
/// Exact upstream nginx source archive.
pub const NGINX_SOURCE_SHA256: &str =
    "3cf46f5ed7504d460125866b7bea99fb357f31d887e2e63b35503310e9b8b6c8";

/// One regular artifact, committed by exact bytes rather than its mutable name.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    /// Lowercase SHA256.
    pub sha256: String,
    /// Complete file length, bounded before reading.
    pub bytes: u64,
}
/// No credentials or installation/admin configuration belong in this document.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Versioned provenance schema.
    pub schema: String,
    /// Exact module package identity.
    pub package: String,
    /// Linux GNU tuple; an OS label alone cannot establish compatibility.
    pub target: String,
    /// Authored Rust toolchain requirement.
    pub rust: String,
    /// Exact nginx release, separately checked by its loader.
    pub nginx: String,
    /// Upstream source hash, not a claim about arbitrary vendor builds.
    pub nginx_source_sha256: String,
    /// Upstream SDK revision.
    pub sdk_revision: String,
    /// Actual engine -V output, including compiler/configure inputs.
    pub engine_build: String,
    /// Exact checkout and source closure supplied by the hosted owner.
    pub source_revision: String,
    /// Digest of the frozen input manifest.
    pub source_manifest: String,
    /// Actual lock bytes used by the builder.
    pub cargo_lock_sha256: String,
    /// Saved native-feature module, never the standalone pilot's library output.
    pub module: Artifact,
    /// Exact engine executable used by the compatibility pilot.
    pub engine: Artifact,
    /// Always false: the subsequent actual load/run receipt is independent.
    pub abi_load_qualified: bool,
    /// Always false: provenance is not an authenticated release.
    pub signed_release: bool,
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl Manifest {
    /// Reject contradictory provenance or attempts to promote metadata to trust.
    pub fn validate(&self) -> Result<(), &'static str> {
        crate::report::source_identity(&self.source_revision, &self.source_manifest)?;
        if self.schema != "humanymous.nginx.package/0.1"
            || self.package != "humanymous-nginx-adapter"
            || self.target != TARGET
            || self.rust != "1.99.1"
            || self.nginx != "1.30.6"
            || self.nginx_source_sha256 != NGINX_SOURCE_SHA256
            || self.sdk_revision != SDK_REV
            || self.abi_load_qualified
            || self.signed_release
            || !digest(&self.cargo_lock_sha256)
            || self.engine_build.len() > 8192
            || !self
                .engine_build
                .starts_with("nginx version: nginx/1.30.6\n")
            || !self.engine_build.contains("--with-compat")
            || !self.engine_build.contains("--with-threads")
            || !self.engine_build.contains("--without-http_rewrite_module")
            || !self.engine_build.contains("--without-http_gzip_module")
            || [&self.module, &self.engine]
                .iter()
                .any(|a| !digest(&a.sha256) || a.bytes == 0 || a.bytes > 64 * 1024 * 1024)
        {
            return Err("unsupported or contradictory nginx package provenance");
        }
        Ok(())
    }
    /// Fixed-schema visitors reject unknown/duplicate fields, including escapes.
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > 32768 {
            return Err("package manifest limit");
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| "package manifest schema")?;
        value.validate()?;
        Ok(value)
    }
    /// Compare all mutable files at consumption; no compatibility is inferred.
    pub fn matches(
        &self,
        module: &Artifact,
        engine: &Artifact,
        lock: &str,
    ) -> Result<(), &'static str> {
        self.validate()?;
        if self.module.sha256 != module.sha256
            || self.module.bytes != module.bytes
            || self.engine.sha256 != engine.sha256
            || self.engine.bytes != engine.bytes
            || self.cargo_lock_sha256 != lock
        {
            return Err("nginx package artifact mismatch");
        }
        Ok(())
    }
}
/// Stream a bounded regular build artifact; never read a credential or raw trace.
/// Trusted build directories must resist replacement; this is not an installer.
pub fn artifact(path: &Path, limit: u64) -> io::Result<Artifact> {
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() == 0 || meta.len() > limit {
        return Err(io::Error::other("bounded regular artifact required"));
    }
    let mut file = std::fs::File::open(path)?;
    let opened = file.metadata()?;
    if !opened.is_file() || opened.len() != meta.len() {
        return Err(io::Error::other("artifact changed"));
    }
    let mut hash = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        size += n as u64;
        if size > limit {
            return Err(io::Error::other("artifact limit"));
        }
        hash.update(&buffer[..n]);
    }
    if size != meta.len() {
        return Err(io::Error::other("artifact length changed"));
    }
    Ok(Artifact {
        sha256: hex::encode(hash.finalize()),
        bytes: size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Manifest {
        Manifest {schema:"humanymous.nginx.package/0.1".into(),package:"humanymous-nginx-adapter".into(),target:TARGET.into(),rust:"1.99.1".into(),nginx:"1.30.6".into(),nginx_source_sha256:NGINX_SOURCE_SHA256.into(),sdk_revision:SDK_REV.into(),engine_build:"nginx version: nginx/1.30.6\nconfigure arguments: --with-compat --with-threads --without-http_rewrite_module --without-http_gzip_module\n".into(),source_revision:"a".repeat(40),source_manifest:"b".repeat(64),cargo_lock_sha256:"c".repeat(64),module:Artifact{sha256:"d".repeat(64),bytes:1024},engine:Artifact{sha256:"e".repeat(64),bytes:2048},abi_load_qualified:false,signed_release:false}
    }
    #[test]
    fn provenance_roundtrip_never_grants_load_or_release() {
        let value = fixture();
        let parsed = Manifest::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(!parsed.abi_load_qualified && !parsed.signed_release);
        assert!(
            parsed
                .matches(&value.module, &value.engine, &value.cargo_lock_sha256)
                .is_ok()
        );
    }
    #[test]
    fn mismatch_or_unsigned_metadata_promotion_rejects() {
        let mut value = fixture();
        value.module.sha256 = "f".repeat(64);
        assert!(
            value
                .matches(&fixture().module, &value.engine, &value.cargo_lock_sha256)
                .is_err()
        );
        let mut value = fixture();
        value.abi_load_qualified = true;
        assert!(value.validate().is_err());
        let mut value = fixture();
        value.signed_release = true;
        assert!(value.validate().is_err());
    }
    #[test]
    fn incompatible_abi_inputs_and_unbound_source_reject() {
        let mut value = fixture();
        value.target = "aarch64-unknown-linux-gnu".into();
        assert!(value.validate().is_err());
        let mut value = fixture();
        value.sdk_revision = "0".repeat(40);
        assert!(value.validate().is_err());
        let mut value = fixture();
        value.source_revision = "unknown".into();
        assert!(value.validate().is_err());
    }
    #[test]
    fn duplicate_and_unknown_manifest_fields_reject() {
        let value = serde_json::to_string(&fixture()).unwrap();
        let duplicate = value.replacen('{', "{\"schema\":\"other\",", 1);
        assert!(Manifest::parse(duplicate.as_bytes()).is_err());
        let unknown = value.replacen('{', "{\"credential\":\"forbidden\",", 1);
        assert!(Manifest::parse(unknown.as_bytes()).is_err());
    }
}

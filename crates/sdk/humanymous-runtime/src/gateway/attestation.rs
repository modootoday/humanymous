//! Pinned-key, fresh-nonce target attestation for one provisioned installation.
//!
//! The shipped runtime binary signs (installation, own executable SHA-256,
//! configuration SHA-256, challenger nonce) with an installation-local Ed25519
//! key it generated itself. A challenger that pinned the enrollment public key
//! refuses any other key, installation, binary, configuration or nonce.
//!
//! This proves possession of the installation key and the reported digests at
//! answer time. It is not hardware, OS-loader, listener or remote-transport
//! attestation, and it never changes a request verdict.
use crate::gateway::configuration::Configuration;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

/// Signed statement document schema.
pub const SCHEMA: &str = "humanymous.target-attestation/1";
/// Private challenge/answer request schema.
pub const REQUEST_SCHEMA: &str = "humanymous.target-attestation-request/1";
/// Enrollment output schema; the only place the public key is first observed.
pub const ENROLLMENT_SCHEMA: &str = "humanymous.target-attestation-enrollment/1";
/// Installation-local private key; never leaves the installation directory.
pub const KEY_FILE: &str = "target-attestation.key";
/// Exact installation configuration bytes whose digest is attested.
pub const CONFIGURATION_FILE: &str = "configuration.json";
const DOMAIN: &[u8] = b"humanymous.target-attestation/1\0";
const CONFIGURATION_BUDGET: u64 = 65_536;
const BINARY_BUDGET: u64 = 268_435_456;
const REQUEST_BUDGET: usize = 8_192;

/// Fail-closed reason. Codes are stable and contain no secret material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Malformed request, statement, encoding or schema.
    Malformed,
    /// Statement public key is not the pinned enrollment key.
    UnpinnedKey,
    /// Signature does not verify strictly under the pinned key.
    Signature,
    /// Signed installation differs from the expected target.
    InstallationMismatch,
    /// Signed executable digest differs from the expected binary.
    BinaryMismatch,
    /// Signed configuration digest differs from the expected configuration.
    ConfigurationMismatch,
    /// Signed nonce is not the challenger's fresh nonce.
    NonceMismatch,
    /// Installation directory, key or configuration custody is unsafe/absent.
    Custody,
    /// Key already provisioned; enrollment never replaces an existing key.
    AlreadyProvisioned,
    /// Platform without an implemented private key custody check.
    Unsupported,
}
impl Refusal {
    /// Stable public refusal code.
    pub fn code(self) -> &'static str {
        match self {
            Self::Malformed => "malformed",
            Self::UnpinnedKey => "unpinned_key",
            Self::Signature => "signature",
            Self::InstallationMismatch => "installation_mismatch",
            Self::BinaryMismatch => "binary_mismatch",
            Self::ConfigurationMismatch => "configuration_mismatch",
            Self::NonceMismatch => "nonce_mismatch",
            Self::Custody => "custody",
            Self::AlreadyProvisioned => "already_provisioned",
            Self::Unsupported => "unsupported",
        }
    }
}

/// Exactly the signed fields; hex values are lowercase and fixed length.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statement {
    /// Configuration scope installation identity.
    pub installation: String,
    /// SHA-256 of the answering executable image.
    pub binary_sha256: String,
    /// SHA-256 of the exact installation configuration bytes.
    pub configuration_sha256: String,
    /// Challenger nonce (32 bytes, hex).
    pub nonce: String,
    /// Ed25519 public key of the installation (32 bytes, hex).
    pub public_key: String,
}

/// Signed statement as emitted by the runtime binary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attestation {
    /// Always [`SCHEMA`].
    pub schema: String,
    /// Signed fields.
    pub statement: Statement,
    /// Strict Ed25519 signature over the domain-separated statement (hex).
    pub signature: String,
}

/// What the challenger independently knows before contacting the target.
#[derive(Clone, Copy, Debug)]
pub struct Expected<'a> {
    /// Intended installation identity.
    pub installation: &'a str,
    /// Expected shipped executable digest.
    pub binary_sha256: &'a str,
    /// Expected configuration digest.
    pub configuration_sha256: &'a str,
    /// Fresh nonce sent with this challenge.
    pub nonce: &'a str,
}

/// Private request for both enrollment (no nonce) and answer (nonce present).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    installation_directory: PathBuf,
    #[serde(default)]
    installation: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
}

fn hex32(value: &str) -> Result<[u8; 32], Refusal> {
    let mut bytes = [0; 32];
    if value.len() != 64 || value.bytes().any(|b| b.is_ascii_uppercase()) {
        return Err(Refusal::Malformed);
    }
    hex::decode_to_slice(value, &mut bytes).map_err(|_| Refusal::Malformed)?;
    Ok(bytes)
}

fn valid_installation(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}

/// Domain-separated, length-prefixed bytes; no JSON canonicalization ambiguity.
pub fn message(statement: &Statement) -> Vec<u8> {
    let mut bytes = DOMAIN.to_vec();
    for field in [
        &statement.installation,
        &statement.binary_sha256,
        &statement.configuration_sha256,
        &statement.nonce,
        &statement.public_key,
    ] {
        bytes.extend_from_slice(&(field.len() as u32).to_be_bytes());
        bytes.extend_from_slice(field.as_bytes());
    }
    bytes
}

fn well_formed(statement: &Statement) -> Result<(), Refusal> {
    if !valid_installation(&statement.installation) {
        return Err(Refusal::Malformed);
    }
    for value in [
        &statement.binary_sha256,
        &statement.configuration_sha256,
        &statement.nonce,
        &statement.public_key,
    ] {
        hex32(value)?;
    }
    Ok(())
}

/// Public key encoding for an installation seed.
pub fn public_key(seed: &[u8; 32]) -> String {
    hex::encode(SigningKey::from_bytes(seed).verifying_key().to_bytes())
}

/// Sign a statement whose public key must belong to this seed.
pub fn sign(seed: &[u8; 32], statement: Statement) -> Result<Attestation, Refusal> {
    well_formed(&statement)?;
    let key = SigningKey::from_bytes(seed);
    if statement.public_key != hex::encode(key.verifying_key().to_bytes()) {
        return Err(Refusal::Malformed);
    }
    let signature = hex::encode(key.sign(&message(&statement)).to_bytes());
    Ok(Attestation {
        schema: SCHEMA.into(),
        statement,
        signature,
    })
}

/// Challenger verification against the pinned enrollment key and its own
/// expectations. Every field is checked; the first refusal is returned.
pub fn verify(
    pinned: &[u8; 32],
    expected: &Expected<'_>,
    attestation: &Attestation,
) -> Result<(), Refusal> {
    if attestation.schema != SCHEMA {
        return Err(Refusal::Malformed);
    }
    let statement = &attestation.statement;
    well_formed(statement)?;
    hex32(expected.nonce)?;
    if hex32(&statement.public_key)? != *pinned {
        return Err(Refusal::UnpinnedKey);
    }
    let key = VerifyingKey::from_bytes(pinned).map_err(|_| Refusal::UnpinnedKey)?;
    let mut raw = [0; 64];
    if attestation.signature.len() != 128
        || attestation
            .signature
            .bytes()
            .any(|b| b.is_ascii_uppercase())
        || hex::decode_to_slice(&attestation.signature, &mut raw).is_err()
    {
        return Err(Refusal::Signature);
    }
    let signature = Signature::from_slice(&raw).map_err(|_| Refusal::Signature)?;
    key.verify_strict(&message(statement), &signature)
        .map_err(|_| Refusal::Signature)?;
    if statement.installation != expected.installation {
        return Err(Refusal::InstallationMismatch);
    }
    if statement.binary_sha256 != expected.binary_sha256 {
        return Err(Refusal::BinaryMismatch);
    }
    if statement.configuration_sha256 != expected.configuration_sha256 {
        return Err(Refusal::ConfigurationMismatch);
    }
    if statement.nonce != expected.nonce {
        return Err(Refusal::NonceMismatch);
    }
    Ok(())
}

fn parse_request(bytes: &[u8]) -> Result<Request, Refusal> {
    if bytes.len() > REQUEST_BUDGET {
        return Err(Refusal::Malformed);
    }
    let request: Request = serde_json::from_slice(bytes).map_err(|_| Refusal::Malformed)?;
    if request.schema != REQUEST_SCHEMA || !request.installation_directory.is_absolute() {
        return Err(Refusal::Malformed);
    }
    Ok(request)
}

fn read_bounded(path: &Path, budget: u64) -> Result<Zeroizing<Vec<u8>>, Refusal> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|_| Refusal::Custody)?;
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(budget + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Refusal::Custody)?;
    if bytes.len() as u64 > budget {
        return Err(Refusal::Custody);
    }
    Ok(bytes)
}

#[cfg(unix)]
fn private(path: &Path, directory: bool) -> Result<(), Refusal> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(path).map_err(|_| Refusal::Custody)?;
    let kind = if directory {
        metadata.is_dir()
    } else {
        metadata.is_file()
    };
    let mode = metadata.mode() & 0o777;
    let allowed = if directory { 0o700 } else { 0o600 };
    if !kind
        || metadata.file_type().is_symlink()
        || mode & !allowed != 0
        || metadata.uid() != rustix::process::geteuid().as_raw()
    {
        return Err(Refusal::Custody);
    }
    Ok(())
}
#[cfg(not(unix))]
fn private(_: &Path, _: bool) -> Result<(), Refusal> {
    Err(Refusal::Unsupported)
}

/// Exact configuration bytes must be a valid runtime configuration.
fn configuration(directory: &Path) -> Result<(String, String), Refusal> {
    let path = directory.join(CONFIGURATION_FILE);
    private(&path, false)?;
    let bytes = read_bounded(&path, CONFIGURATION_BUDGET)?;
    let parsed: Configuration = serde_json::from_slice(&bytes).map_err(|_| Refusal::Custody)?;
    parsed.validate().map_err(|_| Refusal::Custody)?;
    let installation = parsed.scope.installation.clone();
    if !valid_installation(&installation) {
        return Err(Refusal::Custody);
    }
    Ok((installation, hex::encode(Sha256::digest(&*bytes))))
}

fn own_binary_sha256() -> Result<String, Refusal> {
    let path = if cfg!(target_os = "linux") {
        PathBuf::from("/proc/self/exe")
    } else {
        std::env::current_exe().map_err(|_| Refusal::Custody)?
    };
    let bytes = read_bounded(&path, BINARY_BUDGET)?;
    Ok(hex::encode(Sha256::digest(&*bytes)))
}

/// Generate the installation key inside the installation and report only its
/// public half. An existing key is never replaced.
pub fn provision(request: &[u8]) -> Result<Vec<u8>, Refusal> {
    let request = parse_request(request)?;
    if request.nonce.is_some() || request.installation.is_some() {
        return Err(Refusal::Malformed);
    }
    let directory = &request.installation_directory;
    private(directory, true)?;
    let (installation, configuration_sha256) = configuration(directory)?;
    let mut seed = Zeroizing::new([0u8; 32]);
    getrandom::fill(&mut *seed).map_err(|_| Refusal::Custody)?;
    write_new_key(&directory.join(KEY_FILE), &seed)?;
    let public = public_key(&seed);
    serde_json::to_vec(
        &serde_json::json!({"schema":ENROLLMENT_SCHEMA,"installation":installation,
        "configuration_sha256":configuration_sha256,"public_key":public}),
    )
    .map_err(|_| Refusal::Malformed)
}

#[cfg(unix)]
fn write_new_key(path: &Path, seed: &[u8; 32]) -> Result<(), Refusal> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        // O_CREAT|O_EXCL also refuses an existing or dangling symlink.
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                Refusal::AlreadyProvisioned
            } else {
                Refusal::Custody
            }
        })?;
    let encoded = Zeroizing::new(hex::encode(seed));
    file.write_all(encoded.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|_| Refusal::Custody)
}
#[cfg(not(unix))]
fn write_new_key(_: &Path, _: &[u8; 32]) -> Result<(), Refusal> {
    Err(Refusal::Unsupported)
}

/// Answer one challenge. The target itself refuses a challenge addressed to a
/// different installation; the challenger still verifies every field.
pub fn answer(request: &[u8]) -> Result<Vec<u8>, Refusal> {
    let request = parse_request(request)?;
    let (Some(expected_installation), Some(nonce)) = (request.installation, request.nonce) else {
        return Err(Refusal::Malformed);
    };
    hex32(&nonce)?;
    let directory = &request.installation_directory;
    private(directory, true)?;
    let (installation, configuration_sha256) = configuration(directory)?;
    if installation != expected_installation {
        return Err(Refusal::InstallationMismatch);
    }
    let key_path = directory.join(KEY_FILE);
    private(&key_path, false)?;
    let encoded = read_bounded(&key_path, 64)?;
    let text = std::str::from_utf8(&encoded).map_err(|_| Refusal::Custody)?;
    let seed = Zeroizing::new(hex32(text).map_err(|_| Refusal::Custody)?);
    let statement = Statement {
        installation,
        binary_sha256: own_binary_sha256()?,
        configuration_sha256,
        nonce,
        public_key: public_key(&seed),
    };
    serde_json::to_vec(&sign(&seed, statement)?).map_err(|_| Refusal::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn statement(seed: &[u8; 32]) -> Statement {
        Statement {
            installation: "pilot".into(),
            binary_sha256: "b".repeat(64),
            configuration_sha256: "c".repeat(64),
            nonce: "0".repeat(63) + "1",
            public_key: public_key(seed),
        }
    }
    fn expected(statement: &Statement) -> Expected<'_> {
        Expected {
            installation: &statement.installation,
            binary_sha256: &statement.binary_sha256,
            configuration_sha256: &statement.configuration_sha256,
            nonce: &statement.nonce,
        }
    }

    #[test]
    fn pinned_fresh_statement_verifies_and_every_field_is_bound() {
        let seed = [7; 32];
        let pinned = hex32(&public_key(&seed)).unwrap();
        let base = statement(&seed);
        let signed = sign(&seed, base.clone()).unwrap();
        assert_eq!(verify(&pinned, &expected(&base), &signed), Ok(()));
        let other = "d".repeat(64);
        let cases: [(Expected<'_>, Refusal); 4] = [
            (
                Expected {
                    installation: "foreign",
                    ..expected(&base)
                },
                Refusal::InstallationMismatch,
            ),
            (
                Expected {
                    binary_sha256: &other,
                    ..expected(&base)
                },
                Refusal::BinaryMismatch,
            ),
            (
                Expected {
                    configuration_sha256: &other,
                    ..expected(&base)
                },
                Refusal::ConfigurationMismatch,
            ),
            (
                Expected {
                    nonce: &other,
                    ..expected(&base)
                },
                Refusal::NonceMismatch,
            ),
        ];
        for (expectation, refusal) in cases {
            assert_eq!(verify(&pinned, &expectation, &signed), Err(refusal));
        }
    }

    #[test]
    fn unknown_key_and_rewritten_fields_are_refused() {
        let seed = [7; 32];
        let pinned = hex32(&public_key(&seed)).unwrap();
        let base = statement(&seed);
        // A different installation key is never accepted, even with valid fields.
        let foreign_seed = [9; 32];
        let foreign = sign(&foreign_seed, statement(&foreign_seed)).unwrap();
        let foreign_expected = statement(&foreign_seed);
        assert_eq!(
            verify(&pinned, &expected(&foreign_expected), &foreign),
            Err(Refusal::UnpinnedKey)
        );
        // Claiming the pinned key without its private half fails the signature.
        let mut impostor = foreign.clone();
        impostor.statement.public_key = base.public_key.clone();
        assert_eq!(
            verify(&pinned, &expected(&base), &impostor),
            Err(Refusal::Signature)
        );
        // Any signed field rewritten after signing invalidates the signature.
        let signed = sign(&seed, base.clone()).unwrap();
        for rewrite in [
            |s: &mut Statement| s.installation = "other".into(),
            |s: &mut Statement| s.binary_sha256 = "e".repeat(64),
            |s: &mut Statement| s.configuration_sha256 = "e".repeat(64),
            |s: &mut Statement| s.nonce = "e".repeat(64),
        ] {
            let mut tampered = signed.clone();
            rewrite(&mut tampered.statement);
            let mut expectation = tampered.statement.clone();
            expectation.public_key = base.public_key.clone();
            assert_eq!(
                verify(&pinned, &expected(&expectation), &tampered),
                Err(Refusal::Signature)
            );
        }
        let mut flipped = signed.clone();
        flipped.signature.replace_range(0..2, "00");
        if flipped.signature == signed.signature {
            flipped.signature.replace_range(0..2, "01");
        }
        assert_eq!(
            verify(&pinned, &expected(&base), &flipped),
            Err(Refusal::Signature)
        );
    }

    #[test]
    fn malformed_statements_requests_and_mismatched_seed_are_refused() {
        let seed = [7; 32];
        let pinned = hex32(&public_key(&seed)).unwrap();
        let base = statement(&seed);
        let mut upper = base.clone();
        upper.nonce = "A".repeat(64);
        assert_eq!(sign(&seed, upper), Err(Refusal::Malformed));
        let mut wrong_key = base.clone();
        wrong_key.public_key = public_key(&[1; 32]);
        assert_eq!(sign(&seed, wrong_key), Err(Refusal::Malformed));
        let mut bad_installation = base.clone();
        bad_installation.installation = "a/b".into();
        assert_eq!(sign(&seed, bad_installation), Err(Refusal::Malformed));
        let mut schema = sign(&seed, base.clone()).unwrap();
        schema.schema = "humanymous.target-attestation/0".into();
        assert_eq!(
            verify(&pinned, &expected(&base), &schema),
            Err(Refusal::Malformed)
        );
        let short = sign(&seed, base.clone()).unwrap();
        assert_eq!(
            verify(
                &pinned,
                &Expected {
                    nonce: "00",
                    ..expected(&base)
                },
                &short
            ),
            Err(Refusal::Malformed)
        );
        for request in [
            &b"{}"[..],
            br#"{"schema":"humanymous.target-attestation-request/1","installation_directory":"relative"}"#,
            br#"{"schema":"other","installation_directory":"/x"}"#,
            br#"{"schema":"humanymous.target-attestation-request/1","installation_directory":"/x","extra":1}"#,
        ] {
            assert_eq!(provision(request), Err(Refusal::Malformed));
            assert_eq!(answer(request), Err(Refusal::Malformed));
        }
        let oversized = vec![b' '; REQUEST_BUDGET + 1];
        assert_eq!(answer(&oversized), Err(Refusal::Malformed));
        assert_eq!(
            answer(
                br#"{"schema":"humanymous.target-attestation-request/1","installation_directory":"/x","installation":"pilot"}"#
            ),
            Err(Refusal::Malformed)
        );
        assert_eq!(
            provision(
                br#"{"schema":"humanymous.target-attestation-request/1","installation_directory":"/x","nonce":"00"}"#
            ),
            Err(Refusal::Malformed)
        );
    }

    #[cfg(unix)]
    mod custody {
        use super::super::*;
        use std::os::unix::fs::PermissionsExt;

        struct Directory(PathBuf);
        impl Drop for Directory {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        fn installation(name: &str, installation: &str) -> Directory {
            let mut random = [0; 8];
            getrandom::fill(&mut random).unwrap();
            let path = std::env::temp_dir()
                .canonicalize()
                .unwrap()
                .join(format!("hm-attestation-{name}-{}", hex::encode(random)));
            std::fs::create_dir(&path).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            let config = serde_json::json!({"listen":"127.0.0.1:0","origin":"127.0.0.1:9",
                "scope":{"installation":installation,"tenant":"t","origin":"o","revision":"r1","authority":"owned.test"},
                "routes":[{"method":"POST","path":"/effect","required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}],
                "audit":{"directory":path,"node_id":"n","keys":{"directory":path,"passphrase":"p"},"allow_new":true,"trusted_head":null},
                "owner":{"kind":"unix","uid":0},"session_capacity":1,"session_ttl_ms":1000,"timeout_ms":1000,"rate_per_second":1});
            let file = path.join(CONFIGURATION_FILE);
            std::fs::write(&file, serde_json::to_vec(&config).unwrap()).unwrap();
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
            Directory(path)
        }
        fn request(directory: &Path, installation: Option<&str>, nonce: Option<&str>) -> Vec<u8> {
            let mut value =
                serde_json::json!({"schema":REQUEST_SCHEMA,"installation_directory":directory});
            if let Some(installation) = installation {
                value["installation"] = serde_json::json!(installation);
            }
            if let Some(nonce) = nonce {
                value["nonce"] = serde_json::json!(nonce);
            }
            serde_json::to_vec(&value).unwrap()
        }

        #[test]
        fn provisioned_key_answers_and_is_never_replaced() {
            let target = installation("own", "pilot");
            let enrollment: serde_json::Value =
                serde_json::from_slice(&provision(&request(&target.0, None, None)).unwrap())
                    .unwrap();
            assert_eq!(enrollment["schema"], ENROLLMENT_SCHEMA);
            assert_eq!(
                provision(&request(&target.0, None, None)),
                Err(Refusal::AlreadyProvisioned)
            );
            let mode = std::fs::metadata(target.0.join(KEY_FILE))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
            let pinned = hex32(enrollment["public_key"].as_str().unwrap()).unwrap();
            let nonce = "5".repeat(64);
            let attestation: Attestation = serde_json::from_slice(
                &answer(&request(&target.0, Some("pilot"), Some(&nonce))).unwrap(),
            )
            .unwrap();
            let binary = own_binary_sha256().unwrap();
            let configuration = hex::encode(Sha256::digest(
                std::fs::read(target.0.join(CONFIGURATION_FILE)).unwrap(),
            ));
            assert_eq!(
                configuration,
                enrollment["configuration_sha256"].as_str().unwrap()
            );
            let expectation = Expected {
                installation: "pilot",
                binary_sha256: &binary,
                configuration_sha256: &configuration,
                nonce: &nonce,
            };
            assert_eq!(verify(&pinned, &expectation, &attestation), Ok(()));
            assert_eq!(
                answer(&request(&target.0, Some("foreign"), Some(&nonce))),
                Err(Refusal::InstallationMismatch)
            );
        }

        #[test]
        fn unsafe_custody_and_invalid_configuration_are_refused() {
            let target = installation("unsafe", "pilot");
            let nonce = "6".repeat(64);
            // Answer before enrollment has no key.
            assert_eq!(
                answer(&request(&target.0, Some("pilot"), Some(&nonce))),
                Err(Refusal::Custody)
            );
            provision(&request(&target.0, None, None)).unwrap();
            let key = target.0.join(KEY_FILE);
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert_eq!(
                answer(&request(&target.0, Some("pilot"), Some(&nonce))),
                Err(Refusal::Custody)
            );
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
            std::fs::set_permissions(&target.0, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert_eq!(
                answer(&request(&target.0, Some("pilot"), Some(&nonce))),
                Err(Refusal::Custody)
            );
            std::fs::set_permissions(&target.0, std::fs::Permissions::from_mode(0o700)).unwrap();
            let file = target.0.join(CONFIGURATION_FILE);
            std::fs::write(&file, b"{\"listen\":\"0.0.0.0:1\"}").unwrap();
            assert_eq!(
                answer(&request(&target.0, Some("pilot"), Some(&nonce))),
                Err(Refusal::Custody)
            );
            let linked = installation("link", "pilot");
            let alias = linked.0.join("alias");
            std::os::unix::fs::symlink(&target.0, &alias).unwrap();
            assert_eq!(
                provision(&request(&alias, None, None)),
                Err(Refusal::Custody)
            );
        }
    }
}

//! Historical sealed-key compatibility, subject linkage and exclusive native
//! sealed custody. Operational dual-control remains a separate boundary.
pub mod backups;
pub mod custody;
pub mod vault;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

const LIMIT: usize = 4 << 20;

/// Bounded, non-sensitive failure categories. Wrong passwords and altered
/// ciphertext deliberately share one error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyError {
    /// Invalid JSON/base64/framing or an unsupported field.
    Malformed,
    /// Authenticated decryption failed.
    CannotUnseal,
    /// Wrong seed/key width or missing mandatory identity material.
    InvalidMaterial,
    /// Empty or excessive passphrase for normal operation.
    Passphrase,
    /// Input exceeds the bounded local envelope size.
    SizeLimit,
    /// System entropy unavailable; never generate a predictable identity.
    Entropy,
}
impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Malformed => "invalid sealed key format",
            Self::CannotUnseal => "cannot unseal key material",
            Self::InvalidMaterial => "invalid key material",
            Self::Passphrase => "invalid unseal passphrase",
            Self::SizeLimit => "sealed key size limit",
            Self::Entropy => "key entropy unavailable",
        })
    }
}
impl std::error::Error for KeyError {}

/// Secret key material has no Debug/Clone implementation. Zeroization covers
/// owned buffers, not OS snapshots, allocator copies or previously persisted
/// backups. A co-located witness seed is legacy reference state, not independent
/// witness custody.
pub struct KeyMaterial {
    signing_seed: Zeroizing<[u8; 32]>,
    hmac_key: Zeroizing<Vec<u8>>,
    vault: Option<Zeroizing<Vec<u8>>>,
    witness_seed: Option<Zeroizing<[u8; 32]>>,
}
impl KeyMaterial {
    /// Construct validated material; preserve null versus empty vault bytes for
    /// historical serialized compatibility.
    pub fn new(
        seed: [u8; 32],
        hmac: Vec<u8>,
        vault: Option<Vec<u8>>,
        witness: Option<[u8; 32]>,
    ) -> Result<Self, KeyError> {
        let signing_seed = Zeroizing::new(seed);
        let hmac_key = Zeroizing::new(hmac);
        let vault = vault.map(Zeroizing::new);
        let witness_seed = witness.map(Zeroizing::new);
        if hmac_key.is_empty()
            || hmac_key.len() > 1024
            || vault.as_ref().is_some_and(|v| v.len() > LIMIT / 2)
        {
            return Err(KeyError::InvalidMaterial);
        }
        Ok(Self {
            signing_seed,
            hmac_key,
            vault,
            witness_seed,
        })
    }
    /// Stable writer seed, to provision a signer without replacement on boot.
    pub fn signing_seed(&self) -> &[u8; 32] {
        &self.signing_seed
    }
    /// Secondary record authentication key.
    pub fn hmac_key(&self) -> &[u8] {
        &self.hmac_key
    }
    /// Optional serialized per-subject linkage vault; treat as secret.
    pub fn vault(&self) -> Option<&[u8]> {
        self.vault.as_deref().map(|v| v.as_slice())
    }
    /// Legacy co-located witness seed. Its presence is not key independence.
    pub fn witness_seed(&self) -> Option<&[u8; 32]> {
        self.witness_seed.as_deref()
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Plain {
    signing_seed: Zeroizing<String>,
    hmac_key: Zeroizing<String>,
    vault: Option<Zeroizing<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    witness_seed: Option<Zeroizing<String>>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Blob {
    salt: String,
    nonce: String,
    ct: String,
}

fn decode(s: &str) -> Result<Zeroizing<Vec<u8>>, KeyError> {
    let bytes = Zeroizing::new(STANDARD.decode(s).map_err(|_| KeyError::Malformed)?);
    if STANDARD.encode(&bytes) != s {
        return Err(KeyError::Malformed);
    }
    Ok(bytes)
}
fn encode(bytes: &[u8]) -> Zeroizing<String> {
    Zeroizing::new(STANDARD.encode(bytes))
}
fn material_bytes(m: &KeyMaterial) -> Result<Zeroizing<Vec<u8>>, KeyError> {
    let plain = Plain {
        signing_seed: encode(&m.signing_seed[..]),
        hmac_key: encode(&m.hmac_key),
        vault: m.vault.as_ref().map(|v| encode(v)),
        witness_seed: m.witness_seed.as_ref().map(|v| encode(&v[..])),
    };
    serde_json::to_vec(&plain)
        .map(Zeroizing::new)
        .map_err(|_| KeyError::Malformed)
}
fn derive(passphrase: &[u8], salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, KeyError> {
    let mut key = Zeroizing::new([0; 32]);
    let params = scrypt::Params::new(15, 8, 1).map_err(|_| KeyError::Malformed)?;
    scrypt::scrypt(passphrase, salt, &params, &mut key[..]).map_err(|_| KeyError::Malformed)?;
    Ok(key)
}
fn check_passphrase(passphrase: &[u8], legacy: bool) -> Result<(), KeyError> {
    if (!legacy && passphrase.is_empty()) || passphrase.len() > 4096 {
        return Err(KeyError::Passphrase);
    }
    Ok(())
}

/// Encrypt with fresh OS-generated salt and nonce. This returns an envelope;
/// durable atomic file replacement and native ACL checks are not implied.
pub fn seal(material: &KeyMaterial, passphrase: &[u8]) -> Result<Vec<u8>, KeyError> {
    check_passphrase(passphrase, false)?;
    let mut salt = [0; 16];
    let mut nonce = [0; 12];
    getrandom::fill(&mut salt).map_err(|_| KeyError::Entropy)?;
    getrandom::fill(&mut nonce).map_err(|_| KeyError::Entropy)?;
    seal_with_entropy(material, passphrase, salt, nonce)
}
fn seal_with_entropy(
    material: &KeyMaterial,
    passphrase: &[u8],
    salt: [u8; 16],
    nonce: [u8; 12],
) -> Result<Vec<u8>, KeyError> {
    let key = derive(passphrase, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key[..]).map_err(|_| KeyError::InvalidMaterial)?;
    let plain = material_bytes(material)?;
    let nonce_array: Nonce<aes_gcm::aead::consts::U12> = nonce.into();
    let ct = cipher
        .encrypt(&nonce_array, plain.as_slice())
        .map_err(|_| KeyError::CannotUnseal)?;
    serde_json::to_vec(&Blob {
        salt: STANDARD.encode(salt),
        nonce: STANDARD.encode(nonce),
        ct: STANDARD.encode(ct),
    })
    .map_err(|_| KeyError::Malformed)
}

/// Authenticate and decode a historical scrypt/AES256-GCM envelope. No parse,
/// password or authentication failure creates replacement keys or an empty vault.
pub fn open(blob: &[u8], passphrase: &[u8]) -> Result<KeyMaterial, KeyError> {
    open_inner(blob, passphrase, false)
}
/// Explicit migration-only import for legacy empty-passphrase stores. Normal
/// seal/open reject empty passwords; immediately re-seal imported material.
pub fn import_legacy(blob: &[u8], passphrase: &[u8]) -> Result<KeyMaterial, KeyError> {
    open_inner(blob, passphrase, true)
}
fn open_inner(blob: &[u8], passphrase: &[u8], legacy: bool) -> Result<KeyMaterial, KeyError> {
    check_passphrase(passphrase, legacy)?;
    if blob.len() > LIMIT {
        return Err(KeyError::SizeLimit);
    }
    let b: Blob = serde_json::from_slice(blob).map_err(|_| KeyError::Malformed)?;
    let salt = decode(&b.salt)?;
    let nonce = decode(&b.nonce)?;
    let ct = decode(&b.ct)?;
    if salt.len() != 16 || nonce.len() != 12 || ct.len() < 16 {
        return Err(KeyError::Malformed);
    }
    let key = derive(passphrase, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key[..]).map_err(|_| KeyError::InvalidMaterial)?;
    let nonce: Nonce<aes_gcm::aead::consts::U12> = <&[u8; 12]>::try_from(nonce.as_slice())
        .map_err(|_| KeyError::Malformed)?
        .to_owned()
        .into();
    let plain = Zeroizing::new(
        cipher
            .decrypt(&nonce, ct.as_slice())
            .map_err(|_| KeyError::CannotUnseal)?,
    );
    let wire: Plain = serde_json::from_slice(&plain).map_err(|_| KeyError::InvalidMaterial)?;
    let seed = decode(&wire.signing_seed)?;
    let seed: [u8; 32] = seed
        .as_slice()
        .try_into()
        .map_err(|_| KeyError::InvalidMaterial)?;
    let hmac = decode(&wire.hmac_key)?;
    let vault = wire.vault.as_ref().map(|v| decode(v)).transpose()?;
    let witness = wire
        .witness_seed
        .as_ref()
        .map(|v| {
            let bytes = decode(v)?;
            if bytes.is_empty() {
                return Ok(None);
            }
            <[u8; 32]>::try_from(bytes.as_slice())
                .map(Some)
                .map_err(|_| KeyError::InvalidMaterial)
        })
        .transpose()?
        .flatten();
    KeyMaterial::new(seed, hmac.to_vec(), vault.map(|v| v.to_vec()), witness)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decrypts_and_reencrypts_frozen_go_envelopes() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/audit/keys.json")).unwrap();
        for case in fixture["sealed"].as_array().unwrap() {
            let blob = serde_json::to_vec(&case["blob"]).unwrap();
            let pass = case["passphrase"].as_str().unwrap().as_bytes();
            let material = import_legacy(&blob, pass).unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&material_bytes(&material).unwrap())
                    .unwrap(),
                case["material"]
            );
            let salt = decode(case["blob"]["salt"].as_str().unwrap()).unwrap();
            let nonce = decode(case["blob"]["nonce"].as_str().unwrap()).unwrap();
            let resealed = seal_with_entropy(
                &material,
                pass,
                salt.as_slice().try_into().unwrap(),
                nonce.as_slice().try_into().unwrap(),
            )
            .unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&resealed).unwrap(),
                case["blob"]
            );
            assert!(matches!(
                open(&blob, b"wrong-fixture-passphrase"),
                Err(KeyError::CannotUnseal)
            ));
        }
    }
    #[test]
    fn corruption_and_empty_secret_do_not_create_identity() {
        let material = KeyMaterial::new([7; 32], vec![1; 32], None, None).unwrap();
        assert!(matches!(seal(&material, b""), Err(KeyError::Passphrase)));
        let first = seal(&material, b"fixture").unwrap();
        let second = seal(&material, b"fixture").unwrap();
        assert_ne!(first, second);
        assert_eq!(open(&first, b"fixture").unwrap().signing_seed(), &[7; 32]);
        let mut value: serde_json::Value = serde_json::from_slice(&first).unwrap();
        let mut ciphertext = STANDARD.decode(value["ct"].as_str().unwrap()).unwrap();
        *ciphertext.last_mut().unwrap() ^= 1;
        value["ct"] = serde_json::json!(STANDARD.encode(ciphertext));
        assert!(matches!(
            open(&serde_json::to_vec(&value).unwrap(), b"fixture"),
            Err(KeyError::CannotUnseal)
        ));
        value["nonce"] = serde_json::json!("AA==");
        assert!(matches!(
            open(&serde_json::to_vec(&value).unwrap(), b"fixture"),
            Err(KeyError::Malformed)
        ));
        assert!(matches!(open(b"{}", b"fixture"), Err(KeyError::Malformed)));
        assert!(matches!(
            open(&vec![b' '; LIMIT + 1], b"fixture"),
            Err(KeyError::SizeLimit)
        ));
    }
}

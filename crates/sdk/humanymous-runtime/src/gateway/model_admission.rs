//! Startup admission of the behavioral model bundle (legacy cmd/server/ml_setup.go
//! `setupBehavioralML` admission half and the Ed25519 half of
//! internal/mlcorrect/bundle.go `NewBundleManager`/`Stage`, baseline 981d25c).
//!
//! Admission reads the bundle, the optional PKIX Ed25519 public key and the
//! optional detached signature, then stages and promotes through the core
//! [`BundleManager`], so the schema pin, the optional SHA-256 digest pin and,
//! when a key is configured, the signature are enforced on the exact bytes that
//! are decoded. Any failure is non-fatal for the caller: the result is
//! [`Admission::Refused`] and the model seam keeps abstaining (heuristics only).
//!
//! [`lab_control`] ports the rest of `setupBehavioralML` as a LAB control
//! plane: it builds the self-calibration controller with the legacy
//! parameters, arms the canary whose auto-rollback reverts the admitted lab
//! identity through the [`BundleManager`], and loads the shadow candidate.
//!
//! Not ported here: installing the controller as threshold provider, feature
//! observer or shadow observer, and installing the admitted or shadow model
//! into the serving inference runtime. Under owner decision #12 (issue #75
//! comment 7017558784) startup never activates a model: a served model is
//! staged with `gateway::admin_plane::models::Models::stage` and activated
//! only by the audited, dual-controlled admin promote, which installs no
//! threshold provider.
//!
//! Deliberate fail-closed differences from legacy, each pinned by a test:
//! - A configured public-key file that is empty is refused (legacy treated an
//!   empty file as "no key", admitted the bundle unsigned and logged
//!   `signed=true`).
//! - Signatures use `verify_strict` (no small-order keys or non-canonical
//!   points); a signature from the legacy `ed25519.Sign` verifies identically.
//! - Only an Ed25519 SubjectPublicKeyInfo is parsed; another PKIX algorithm is
//!   [`KeyError::NotEd25519`] (legacy parsed it, then refused it the same way).
//!   PEM blocks with RFC 1421 headers are refused rather than skipped.
//! - Files are read with the core bundle bound; diagnostics carry no paths or
//!   bytes.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, VerifyingKey};
use humanymous_core::model::correction::{
    BundleIdentity, BundleManager, BundleVerifier, CorrectionError, MAX_BUNDLE_BYTES,
};
use humanymous_core::model::correction::{CanaryBudget, Controller};
use humanymous_core::model::inference::Mlp;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

/// DER prefix of an Ed25519 SubjectPublicKeyInfo (RFC 8410): SEQUENCE,
/// AlgorithmIdentifier { OID 1.3.101.112 }, BIT STRING with no unused bits.
const ED25519_SPKI_PREFIX: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];
/// Largest public-key or signature file accepted (a PEM key is ~113 bytes).
pub(in crate::gateway) const MAX_SMALL_FILE: usize = 16 * 1024;

/// Static public-key refusals; no key bytes are interpolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyError {
    /// No PEM block, an unterminated block, headers or invalid base64.
    Pem,
    /// The block is not a well-formed SubjectPublicKeyInfo.
    Der,
    /// A well-formed SubjectPublicKeyInfo for another algorithm.
    NotEd25519,
    /// The Ed25519 key bytes are not a valid curve point.
    Point,
}

/// Ed25519 verifier over a PKIX (`PUBLIC KEY`) PEM public key.
#[derive(Debug, Clone)]
pub struct Ed25519Verifier {
    key: VerifyingKey,
}

impl Ed25519Verifier {
    /// Parse the first PEM block as a PKIX Ed25519 public key (legacy
    /// `pem.Decode` + `x509.ParsePKIXPublicKey` + ed25519 type check).
    pub fn from_pkix_pem(pem: &[u8]) -> Result<Self, KeyError> {
        let der = pem_block(pem)?;
        let key = spki_ed25519(&der)?;
        let key = VerifyingKey::from_bytes(&key).map_err(|_| KeyError::Point)?;
        Ok(Self { key })
    }
}

impl BundleVerifier for Ed25519Verifier {
    fn verify(&self, bundle_bytes: &[u8], signature: &[u8]) -> bool {
        Signature::from_slice(signature)
            .is_ok_and(|signature| self.key.verify_strict(bundle_bytes, &signature).is_ok())
    }
}

/// First `-----BEGIN <type>-----` block, base64 body decoded. As in legacy
/// `pem.Decode`, text before the block is skipped and the type is not checked.
fn pem_block(text: &[u8]) -> Result<Vec<u8>, KeyError> {
    let text = std::str::from_utf8(text).map_err(|_| KeyError::Pem)?;
    let start = text.find("-----BEGIN ").ok_or(KeyError::Pem)?;
    let rest = &text[start + "-----BEGIN ".len()..];
    let (label, rest) = rest.split_once("-----").ok_or(KeyError::Pem)?;
    if label.is_empty() || label.contains(['\r', '\n']) {
        return Err(KeyError::Pem);
    }
    let end = format!("-----END {label}-----");
    let (body, _) = rest.split_once(end.as_str()).ok_or(KeyError::Pem)?;
    if body.contains(':') {
        return Err(KeyError::Pem);
    }
    let body: String = body.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    STANDARD.decode(body).map_err(|_| KeyError::Pem)
}

/// Exact Ed25519 SPKI, or classify a different well-formed algorithm.
fn spki_ed25519(der: &[u8]) -> Result<[u8; 32], KeyError> {
    if der.len() == 44 && der[..12] == ED25519_SPKI_PREFIX {
        let mut key = [0; 32];
        key.copy_from_slice(&der[12..]);
        return Ok(key);
    }
    // SEQUENCE { SEQUENCE { OBJECT IDENTIFIER, ... }, BIT STRING } with short
    // or long definite lengths: anything else is not a SubjectPublicKeyInfo.
    let (tag, outer, rest) = tlv(der).ok_or(KeyError::Der)?;
    if tag != 0x30 || !rest.is_empty() {
        return Err(KeyError::Der);
    }
    let (tag, algorithm, rest) = tlv(outer).ok_or(KeyError::Der)?;
    let (bits, _, rest) = tlv(rest).ok_or(KeyError::Der)?;
    if tag != 0x30 || bits != 0x03 || !rest.is_empty() {
        return Err(KeyError::Der);
    }
    let (tag, oid, _) = tlv(algorithm).ok_or(KeyError::Der)?;
    if tag != 0x06 || oid.is_empty() {
        return Err(KeyError::Der);
    }
    if oid == [0x2b, 0x65, 0x70] {
        // Ed25519 with parameters or a wrong key length.
        return Err(KeyError::Der);
    }
    Err(KeyError::NotEd25519)
}

/// One DER tag-length-value: (tag, value, remainder).
fn tlv(input: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, input) = input.split_first()?;
    let (&first, mut input) = input.split_first()?;
    let length = if first < 0x80 {
        usize::from(first)
    } else {
        let count = usize::from(first & 0x7f);
        if count == 0 || count > 2 || input.len() < count {
            return None;
        }
        let mut length = 0usize;
        for &byte in &input[..count] {
            length = length << 8 | usize::from(byte);
        }
        input = &input[count..];
        length
    };
    (input.len() >= length).then(|| (tag, &input[..length], &input[length..]))
}

/// Behavioral-model startup configuration (legacy `mlFlags` admission fields).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Flags {
    /// `-ml-bundle`: bundle JSON path; `None` disables the model.
    pub bundle: Option<PathBuf>,
    /// `-ml-pubkey`: PKIX Ed25519 PEM path; when set, a signature is required.
    pub public_key: Option<PathBuf>,
    /// `-ml-bundle-sig`: detached signature over the exact bundle bytes.
    pub signature: Option<PathBuf>,
    /// `-ml-bundle-digest`: lowercase hex SHA-256 pin; empty means unset, as
    /// with the legacy flag default.
    pub digest: String,
    /// `-ml-shadow-bundle`: only reported here (shadow scoring is not ported).
    pub shadow_bundle: Option<PathBuf>,
}

/// Why admission refused the bundle; the model seam keeps abstaining.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The public-key file could not be read, was empty or over the bound.
    PublicKeyRead,
    /// The public key is not a usable PKIX Ed25519 key.
    PublicKey(KeyError),
    /// The signature file could not be read or was over the bound.
    SignatureRead,
    /// The bundle file could not be read or was empty or over the bound.
    BundleRead,
    /// Digest, signature, shape or schema verification failed.
    Stage(CorrectionError),
    /// Promotion of the staged identity failed.
    Promote(CorrectionError),
}

/// An admitted bundle. Promotion moved only the lab identity; installing
/// [`Admitted::model`] into a serving seam is the caller's separate decision.
#[derive(Debug)]
pub struct Admitted {
    /// Lifecycle owner of the admitted identity (rollback target); shared
    /// with the canary's rollback notification.
    pub bundles: Arc<BundleManager>,
    /// Active identity: bundle version and SHA-256 of the admitted bytes.
    pub identity: BundleIdentity,
    /// Decoded from the same bytes that were verified.
    pub model: Mlp,
    /// A public key was configured and the signature verified.
    pub signed: bool,
    /// Neither a key nor a digest pin was configured: integrity rests on the
    /// schema pin alone (legacy logged a production WARNING).
    pub schema_pin_only: bool,
}

/// Startup admission result.
#[derive(Debug)]
pub enum Admission {
    /// No bundle configured; `shadow_ignored` reports a shadow bundle that
    /// legacy also ignored without `-ml-bundle`.
    Disabled {
        /// A shadow bundle was configured without a served bundle.
        shadow_ignored: bool,
    },
    /// Admission failed closed; the model stays disabled.
    Refused(Refusal),
    /// The bundle passed every configured check and is the active identity.
    Admitted(Box<Admitted>),
}

impl Admission {
    /// The admitted bundle, if any.
    pub fn admitted(&self) -> Option<&Admitted> {
        match self {
            Self::Admitted(admitted) => Some(admitted),
            _ => None,
        }
    }
}

pub(in crate::gateway) fn read_bounded(path: &PathBuf, limit: usize) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() <= limit).then_some(bytes)
}

/// Admit the configured bundle (legacy `setupBehavioralML` up to `Promote`).
pub fn admit(flags: &Flags) -> Admission {
    let Some(bundle) = &flags.bundle else {
        return Admission::Disabled {
            shadow_ignored: flags.shadow_bundle.is_some(),
        };
    };
    let bundles = match &flags.public_key {
        None => BundleManager::unsigned_lab(),
        Some(path) => {
            let Some(pem) = read_bounded(path, MAX_SMALL_FILE).filter(|pem| !pem.is_empty()) else {
                return Admission::Refused(Refusal::PublicKeyRead);
            };
            match Ed25519Verifier::from_pkix_pem(&pem) {
                Ok(verifier) => BundleManager::with_verifier(Box::new(verifier)),
                Err(error) => return Admission::Refused(Refusal::PublicKey(error)),
            }
        }
    };
    let signature = match &flags.signature {
        None => None,
        Some(path) => match read_bounded(path, MAX_SMALL_FILE) {
            Some(signature) => Some(signature),
            None => return Admission::Refused(Refusal::SignatureRead),
        },
    };
    let Some(bytes) = read_bounded(bundle, MAX_BUNDLE_BYTES).filter(|bytes| !bytes.is_empty())
    else {
        return Admission::Refused(Refusal::BundleRead);
    };
    let digest = (!flags.digest.is_empty()).then_some(flags.digest.as_str());
    if let Err(error) = bundles.stage_signed(&bytes, digest, signature.as_deref()) {
        return Admission::Refused(Refusal::Stage(error));
    }
    let identity = match bundles.promote() {
        Ok(identity) => identity,
        Err(error) => return Admission::Refused(Refusal::Promote(error)),
    };
    // Same bytes the stage decoded and schema-checked; cannot fail after it.
    let Ok(model) = Mlp::from_json(&bytes) else {
        return Admission::Refused(Refusal::Stage(CorrectionError::InvalidBundle));
    };
    let signed = bundles.signed();
    Admission::Admitted(Box::new(Admitted {
        bundles: Arc::new(bundles),
        identity,
        model,
        signed,
        schema_pin_only: !signed && digest.is_none(),
    }))
}

/// Lab control-plane configuration (legacy `mlFlags` control fields).
#[derive(Debug, Clone, PartialEq)]
pub struct ControlFlags {
    /// `-ml-fp-budget`: tolerated human false-positive rate (default 0.005).
    pub fp_budget: f64,
    /// `-ml-canary`: put the admitted model on probation.
    pub canary: bool,
    /// `-ml-canary-max-fp`: realized human-FP ceiling (default 0.02).
    pub canary_max_fp: f64,
    /// `-ml-canary-probation`: confirmed humans before graduation (default 200).
    pub canary_probation: i64,
}

impl Default for ControlFlags {
    fn default() -> Self {
        Self {
            fp_budget: 0.005,
            canary: false,
            canary_max_fp: 0.02,
            canary_probation: 200,
        }
    }
}

/// Shadow candidate outcome (legacy `-ml-shadow-bundle`).
#[derive(Debug)]
pub enum ShadowCandidate {
    /// No shadow bundle configured.
    NotConfigured,
    /// Loaded and shape/schema-validated; never served. Installing it into the
    /// inference runtime's candidate seam is the caller's separate decision.
    Loaded(Box<Mlp>),
    /// Unreadable, oversized or invalid: shadow evaluation stays disabled
    /// (legacy logged and continued).
    Refused,
}

/// The lab control plane for an admitted bundle.
#[derive(Debug)]
pub struct LabControl {
    /// Self-calibration controller (`NewController(budget, 0.5, 0.01)`).
    pub controller: Arc<Controller>,
    /// The canary was armed; on its first breach it rolls the admitted lab
    /// identity back through [`Admitted::bundles`].
    pub canary_armed: bool,
    /// Shadow candidate.
    pub shadow: ShadowCandidate,
}

/// Build the lab control plane for an admitted bundle (legacy
/// `setupBehavioralML` after `Promote`). A nonfinite budget or canary
/// ceiling is refused (legacy narrowed it to float32 and continued); a
/// negative probation wraps like Go's `uint64(int)`, which never graduates.
pub fn lab_control(
    admitted: &Admitted,
    flags: &ControlFlags,
    shadow_bundle: Option<&PathBuf>,
) -> Result<LabControl, CorrectionError> {
    let controller = Arc::new(Controller::new(flags.fp_budget as f32, 0.5, 0.01)?);
    if flags.canary {
        let bundles = Arc::clone(&admitted.bundles);
        controller.arm_canary(
            CanaryBudget {
                max_human_fp: flags.canary_max_fp as f32,
                probation_humans: flags.canary_probation as u64,
                rollback_on_drift: true,
                rollback_on_pass_anomaly: true,
            },
            Some(Arc::new(move |_reason: &str| {
                // Revert the lab identity; with nothing to revert to, the
                // seam already abstains (legacy forced abstention).
                let _ = bundles.rollback();
            })),
        )?;
    }
    let shadow = match shadow_bundle {
        None => ShadowCandidate::NotConfigured,
        Some(path) => read_bounded(path, MAX_BUNDLE_BYTES)
            .and_then(|bytes| Mlp::from_json(&bytes).ok())
            .map_or(ShadowCandidate::Refused, |model| {
                ShadowCandidate::Loaded(Box::new(model))
            }),
    };
    Ok(LabControl {
        controller,
        canary_armed: flags.canary,
        shadow,
    })
}

#[cfg(test)]
mod tests;

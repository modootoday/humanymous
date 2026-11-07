//! Pure signed dual-control policy. A verified authorization is not a one-time
//! execution permit: the durable issuer must consume it before any side effect.
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const APPROVAL_DOMAIN: &[u8] = b"humanymous/admin-approval/v1\0";
const TARGET_DOMAIN: &[u8] = b"humanymous/admin-target/v1\0";
const DIGEST_DOMAIN: &[u8] = b"humanymous/admin-proposal-digest/v1\0";
const POLICY_DOMAIN: &[u8] = b"humanymous/admin-policy/v1\0";
const MAX_ADMINS: usize = 128;

/// Exact installation and node boundary, compared as provisioned UTF-8 bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    /// Provisioned installation identity.
    pub installation: String,
    /// Provisioned node identity within that installation.
    pub node: String,
}

/// Destructive or custody-changing operation requiring two independent keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Remove the identified subject's linkage key and retain its tombstone.
    EraseSubject,
    /// Replace the custody envelope's passphrase while preserving key identity.
    RotatePassphrase,
    /// Retire one specifically inventoried backup object.
    RetireBackup,
    /// Activate the scoped runtime kill switch.
    KillSwitch,
    /// Permanently ban one identified subject within the configured scope.
    PermanentBan,
}
impl Action {
    fn code(self) -> u8 {
        match self {
            Self::EraseSubject => 1,
            Self::RotatePassphrase => 2,
            Self::RetireBackup => 3,
            Self::KillSwitch => 4,
            Self::PermanentBan => 5,
        }
    }
}

/// Complete approval payload. IDs are canonical lowercase 32-byte hex strings.
/// The issuer creates operation_id from OS entropy; durable consumption, not
/// caller-selected uniqueness or this pure validator, prevents replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    /// Exact provisioned installation/node pair.
    pub scope: Scope,
    /// Operation approved by both administrators.
    pub action: Action,
    /// Domain-separated commitment to the operation's canonical target bytes.
    pub target: String,
    /// Fresh 256-bit proposal identifier, generated at the issuance boundary.
    pub operation_id: String,
    /// Trusted UTC Unix seconds at creation.
    pub issued_at: u64,
    /// Exclusive expiration in trusted UTC Unix seconds.
    pub expires_at: u64,
}
impl Proposal {
    /// Bytes signed by each administrator. Frozen order is domain, length-
    /// prefixed installation/node UTF-8, action byte, target32, operation32,
    /// issued-at and expires-at big-endian u64. Invalid framing is rejected.
    pub fn signing_bytes(&self) -> Result<Vec<u8>, ControlError> {
        valid_scope(&self.scope)?;
        let target = canonical_hex::<32>(&self.target)?;
        let operation = canonical_hex::<32>(&self.operation_id)?;
        if operation == [0; 32] {
            return Err(ControlError::Malformed);
        }
        let mut bytes = Vec::with_capacity(256);
        bytes.extend_from_slice(APPROVAL_DOMAIN);
        push_field(&mut bytes, self.scope.installation.as_bytes());
        push_field(&mut bytes, self.scope.node.as_bytes());
        bytes.push(self.action.code());
        bytes.extend_from_slice(&target);
        bytes.extend_from_slice(&operation);
        bytes.extend_from_slice(&self.issued_at.to_be_bytes());
        bytes.extend_from_slice(&self.expires_at.to_be_bytes());
        Ok(bytes)
    }

    /// Stable digest of the complete signed proposal, suitable for a consumed
    /// ledger entry. Approval ordering and signatures do not change this digest.
    pub fn digest(&self) -> Result<String, ControlError> {
        let mut hash = Sha256::new();
        hash.update(DIGEST_DOMAIN);
        hash.update(self.signing_bytes()?);
        Ok(hex::encode(hash.finalize()))
    }
}

/// One strict Ed25519 approval from a provisioned administrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    /// Pinned administrator identity, not a caller-provided public key.
    pub administrator: String,
    /// Canonical lowercase 64-byte Ed25519 signature in hex.
    pub signature: String,
}

/// Non-sensitive policy rejection categories; no rejected payload is logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlError {
    /// Invalid scope/key registry, insufficient independent keys or unsafe TTL.
    Configuration,
    /// Invalid bounded/canonical proposal or signature framing.
    Malformed,
    /// Proposal belongs to a different installation or node.
    WrongScope,
    /// Proposal was issued in the future, expired, or exceeds its allowed TTL.
    TimeWindow,
    /// Approval refers to an unprovisioned administrator.
    UnknownAdministrator,
    /// Fewer than two distinct administrator approvals were supplied.
    Quorum,
    /// A repeated administrator identity does not count twice toward quorum.
    DuplicateAdministrator,
    /// An administrator signature failed strict verification.
    InvalidSignature,
}
impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Configuration => "invalid dual-control configuration",
            Self::Malformed => "invalid dual-control framing",
            Self::WrongScope => "dual-control scope mismatch",
            Self::TimeWindow => "dual-control proposal outside permitted time window",
            Self::UnknownAdministrator => "unknown dual-control administrator",
            Self::Quorum => "dual-control quorum missing",
            Self::DuplicateAdministrator => "duplicate dual-control administrator",
            Self::InvalidSignature => "dual-control signature invalid",
        })
    }
}
impl std::error::Error for ControlError {}

/// Immutable provisioned verification authority. Administrative signing keys
/// live outside this object; two IDs sharing a key cannot impersonate quorum.
pub struct Administrators {
    scope: Scope,
    keys: BTreeMap<String, VerifyingKey>,
    max_lifetime_secs: u64,
}
impl Administrators {
    /// Pin at least two distinct, nonweak Ed25519 keys for one exact scope.
    /// max_lifetime_secs is bounded to one day; production may choose less.
    pub fn new(
        scope: Scope,
        keys: BTreeMap<String, [u8; 32]>,
        max_lifetime_secs: u64,
    ) -> Result<Self, ControlError> {
        valid_scope(&scope).map_err(|_| ControlError::Configuration)?;
        if !(2..=MAX_ADMINS).contains(&keys.len())
            || max_lifetime_secs == 0
            || max_lifetime_secs > 86_400
        {
            return Err(ControlError::Configuration);
        }
        let mut unique = BTreeSet::new();
        let mut verified = BTreeMap::new();
        for (id, bytes) in keys {
            if !valid_identity(&id, 128) || !unique.insert(bytes) {
                return Err(ControlError::Configuration);
            }
            let key = VerifyingKey::from_bytes(&bytes).map_err(|_| ControlError::Configuration)?;
            if key.is_weak() {
                return Err(ControlError::Configuration);
            }
            verified.insert(id, key);
        }
        Ok(Self {
            scope,
            keys: verified,
            max_lifetime_secs,
        })
    }

    /// Verify all supplied approvals, rejecting unknown/duplicate/invalid
    /// extras rather than ignoring them once quorum has been reached. This
    /// proves signatures only; the durable issuer must still enforce clock
    /// monotonicity and consume the operation ID before constructing a permit.
    pub fn authorize(
        &self,
        proposal: &Proposal,
        approvals: &[Approval],
        now: u64,
    ) -> Result<Authorization, ControlError> {
        if proposal.scope != self.scope {
            return Err(ControlError::WrongScope);
        }
        let bytes = proposal.signing_bytes()?;
        let _lifetime = proposal
            .expires_at
            .checked_sub(proposal.issued_at)
            .filter(|ttl| *ttl > 0 && *ttl <= self.max_lifetime_secs)
            .ok_or(ControlError::TimeWindow)?;
        if now < proposal.issued_at || now >= proposal.expires_at {
            return Err(ControlError::TimeWindow);
        }
        if approvals.len() < 2 || approvals.len() > self.keys.len() {
            return Err(ControlError::Quorum);
        }
        let mut seen = BTreeSet::new();
        for approval in approvals {
            let key = self
                .keys
                .get(&approval.administrator)
                .ok_or(ControlError::UnknownAdministrator)?;
            if !seen.insert(approval.administrator.clone()) {
                return Err(ControlError::DuplicateAdministrator);
            }
            let raw = canonical_hex::<64>(&approval.signature)?;
            let signature = Signature::from_bytes(&raw);
            key.verify_strict(&bytes, &signature)
                .map_err(|_| ControlError::InvalidSignature)?;
        }
        Ok(Authorization {
            proposal: proposal.clone(),
            digest: proposal.digest()?,
            administrators: seen.into_iter().collect(),
        })
    }

    /// Exact immutable authority scope.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Stable domain-separated identity of the exact scope, sorted pinned
    /// administrator IDs/keys and maximum proposal lifetime. Durable issuers
    /// pin this fingerprint so a configuration change cannot reset old state.
    pub fn fingerprint(&self) -> String {
        let mut bytes = POLICY_DOMAIN.to_vec();
        push_field(&mut bytes, self.scope.installation.as_bytes());
        push_field(&mut bytes, self.scope.node.as_bytes());
        bytes.extend_from_slice(&self.max_lifetime_secs.to_be_bytes());
        bytes.extend_from_slice(&(self.keys.len() as u64).to_be_bytes());
        for (id, key) in &self.keys {
            push_field(&mut bytes, id.as_bytes());
            bytes.extend_from_slice(key.as_bytes());
        }
        hex::encode(Sha256::digest(bytes))
    }
}

/// Verified signed proposal. No public constructor or deserializer exists;
/// durable consumption is still required before issuing execution capabilities.
pub struct Authorization {
    pub(crate) proposal: Proposal,
    pub(crate) digest: String,
    pub(crate) administrators: Vec<String>,
}
impl Authorization {
    /// Read-only complete signed operation binding.
    pub fn proposal(&self) -> &Proposal {
        &self.proposal
    }
    /// Domain-separated proposal digest for the durable consumption record.
    pub fn digest(&self) -> &str {
        &self.digest
    }
    /// Sorted distinct identities whose signatures passed strict verification.
    pub fn administrators(&self) -> &[String] {
        &self.administrators
    }
}

/// Commit canonical operation-target bytes without placing plaintext target
/// data in approval messages. Callers define the target schema, including any
/// generation/object ID needed to prevent substitution within that operation.
pub fn target_commitment(target: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(TARGET_DOMAIN);
    hash.update((target.len() as u64).to_be_bytes());
    hash.update(target);
    hex::encode(hash.finalize())
}
fn push_field(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    output.extend_from_slice(bytes);
}
fn canonical_hex<const N: usize>(encoded: &str) -> Result<[u8; N], ControlError> {
    if encoded.len() != N * 2
        || encoded
            .bytes()
            .any(|b| !b.is_ascii_digit() && !(b'a'..=b'f').contains(&b))
    {
        return Err(ControlError::Malformed);
    }
    let bytes = hex::decode(encoded).map_err(|_| ControlError::Malformed)?;
    bytes.try_into().map_err(|_| ControlError::Malformed)
}
fn valid_identity(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
fn valid_scope(scope: &Scope) -> Result<(), ControlError> {
    if !valid_identity(&scope.installation, 512) || !valid_identity(&scope.node, 512) {
        return Err(ControlError::Malformed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn scope() -> Scope {
        Scope {
            installation: "installation".into(),
            node: "node".into(),
        }
    }
    fn keys() -> [SigningKey; 3] {
        [
            SigningKey::from_bytes(&[1; 32]),
            SigningKey::from_bytes(&[2; 32]),
            SigningKey::from_bytes(&[3; 32]),
        ]
    }
    fn policy(keys: &[SigningKey; 3]) -> Administrators {
        Administrators::new(
            scope(),
            keys.iter()
                .enumerate()
                .map(|(i, k)| (format!("admin-{i}"), k.verifying_key().to_bytes()))
                .collect(),
            60,
        )
        .unwrap()
    }
    fn proposal() -> Proposal {
        Proposal {
            scope: scope(),
            action: Action::EraseSubject,
            target: target_commitment(b"subject-fixture"),
            operation_id: "ab".repeat(32),
            issued_at: 100,
            expires_at: 160,
        }
    }
    fn approve(proposal: &Proposal, key: &SigningKey, id: usize) -> Approval {
        Approval {
            administrator: format!("admin-{id}"),
            signature: hex::encode(key.sign(&proposal.signing_bytes().unwrap()).to_bytes()),
        }
    }
    #[test]
    fn two_independent_signatures_bind_every_operation_field() {
        let keys = keys();
        let policy = policy(&keys);
        let original = proposal();
        let approvals = [
            approve(&original, &keys[0], 0),
            approve(&original, &keys[1], 1),
        ];
        let accepted = policy.authorize(&original, &approvals, 100).unwrap();
        assert_eq!(accepted.digest(), original.digest().unwrap());
        assert_eq!(accepted.administrators(), &["admin-0", "admin-1"]);
        assert!(policy.authorize(&original, &approvals, 159).is_ok());
        for changed in [
            Proposal {
                action: Action::KillSwitch,
                ..original.clone()
            },
            Proposal {
                target: target_commitment(b"other-subject"),
                ..original.clone()
            },
            Proposal {
                operation_id: "cd".repeat(32),
                ..original.clone()
            },
            Proposal {
                issued_at: 101,
                ..original.clone()
            },
            Proposal {
                expires_at: 159,
                ..original.clone()
            },
        ] {
            assert_eq!(
                policy.authorize(&changed, &approvals, 110).err(),
                Some(ControlError::InvalidSignature)
            );
        }
        let mut changed = original.clone();
        changed.scope.node = "other-node".into();
        assert_eq!(
            policy.authorize(&changed, &approvals, 110).err(),
            Some(ControlError::WrongScope)
        );
        changed.scope = scope();
        changed.scope.installation = "other-installation".into();
        assert_eq!(
            policy.authorize(&changed, &approvals, 110).err(),
            Some(ControlError::WrongScope)
        );
    }
    #[test]
    fn missing_duplicate_unknown_and_invalid_approvals_fail_closed() {
        let keys = keys();
        let policy = policy(&keys);
        let proposal = proposal();
        let first = approve(&proposal, &keys[0], 0);
        let second = approve(&proposal, &keys[1], 1);
        assert_eq!(
            policy
                .authorize(&proposal, std::slice::from_ref(&first), 110)
                .err(),
            Some(ControlError::Quorum)
        );
        assert_eq!(
            policy
                .authorize(&proposal, &[first.clone(), first.clone()], 110)
                .err(),
            Some(ControlError::DuplicateAdministrator)
        );
        let mut bad = second.clone();
        bad.administrator = "unknown".into();
        assert_eq!(
            policy
                .authorize(&proposal, &[first.clone(), bad], 110)
                .err(),
            Some(ControlError::UnknownAdministrator)
        );
        let mut bad = second.clone();
        bad.signature = "00".repeat(64);
        assert_eq!(
            policy
                .authorize(&proposal, &[first.clone(), bad], 110)
                .err(),
            Some(ControlError::InvalidSignature)
        );
        let mut bad = second.clone();
        bad.signature = bad.signature.to_uppercase();
        assert_eq!(
            policy
                .authorize(&proposal, &[first.clone(), bad], 110)
                .err(),
            Some(ControlError::Malformed)
        );
        let mut bad = approve(&proposal, &keys[2], 2);
        bad.signature = "00".repeat(64);
        assert_eq!(
            policy
                .authorize(&proposal, &[first, second, bad], 110)
                .err(),
            Some(ControlError::InvalidSignature)
        );
    }
    #[test]
    fn exact_time_bounds_and_canonical_ids_are_required() {
        let keys = keys();
        let policy = policy(&keys);
        let original = proposal();
        let approvals = [
            approve(&original, &keys[0], 0),
            approve(&original, &keys[1], 1),
        ];
        for now in [0, 99, 160, u64::MAX] {
            assert_eq!(
                policy.authorize(&original, &approvals, now).err(),
                Some(ControlError::TimeWindow)
            );
        }
        for expires_at in [0, 100, 161, u64::MAX] {
            let changed = Proposal {
                expires_at,
                ..original.clone()
            };
            assert_eq!(
                policy.authorize(&changed, &approvals, 110).err(),
                Some(ControlError::TimeWindow)
            );
        }
        for operation_id in ["00".repeat(32), "AB".repeat(32), "short".into()] {
            assert_eq!(
                Proposal {
                    operation_id,
                    ..original.clone()
                }
                .signing_bytes()
                .err(),
                Some(ControlError::Malformed)
            );
        }
        assert_ne!(target_commitment(b"ab"), target_commitment(b"a\0b"));
        assert_ne!(
            original.digest().unwrap(),
            hex::encode(Sha256::digest(original.signing_bytes().unwrap()))
        );
    }
    #[test]
    fn aliases_and_weak_keys_cannot_form_quorum() {
        let key = SigningKey::from_bytes(&[1; 32]).verifying_key().to_bytes();
        let same = [("one".into(), key), ("two".into(), key)]
            .into_iter()
            .collect();
        assert!(matches!(
            Administrators::new(scope(), same, 60),
            Err(ControlError::Configuration)
        ));
        let mut weak = [0; 32];
        weak[0] = 1;
        let keys = [("one".into(), key), ("two".into(), weak)]
            .into_iter()
            .collect();
        assert!(matches!(
            Administrators::new(scope(), keys, 60),
            Err(ControlError::Configuration)
        ));
    }

    #[test]
    fn pinned_configuration_fingerprint_is_order_independent_and_ttl_bound() {
        let keys = keys();
        let original = policy(&keys);
        let reversed = keys
            .iter()
            .enumerate()
            .rev()
            .map(|(i, key)| (format!("admin-{i}"), key.verifying_key().to_bytes()))
            .collect();
        assert_eq!(
            original.fingerprint(),
            Administrators::new(scope(), reversed, 60)
                .unwrap()
                .fingerprint()
        );
        let changed = keys
            .iter()
            .enumerate()
            .map(|(i, key)| (format!("admin-{i}"), key.verifying_key().to_bytes()))
            .collect();
        assert_ne!(
            original.fingerprint(),
            Administrators::new(scope(), changed, 59)
                .unwrap()
                .fingerprint()
        );
    }
}

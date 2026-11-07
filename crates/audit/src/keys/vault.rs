//! Per-subject linkage keys. Persistence must be sealed; erasure requires a
//! durable signed dual-control issuer, not an unrestricted public delete method.
use super::{KeyError, LIMIT};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};
use zeroize::Zeroizing;

/// Immutable pseudonym algorithm selection; restoration must use the provisioned
/// policy because historical snapshots did not encode the stretch setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Historical per-subject HMAC-SHA256.
    Hmac,
    /// Historical scrypt N=4096/r=8/p=1 over subject key with value as salt.
    Scrypt,
}
/// Scope/subject-bound capability issued after durable signed dual control.
/// No public constructor or deserializer can mint this authority.
pub struct ErasurePermit {
    target: String,
    scope: crate::control::Scope,
    operation: String,
}
impl ErasurePermit {
    pub(crate) fn issued(subject: &str, operation: &str, scope: &crate::control::Scope) -> Self {
        Self {
            target: crate::control::target_commitment(subject.as_bytes()),
            scope: scope.clone(),
            operation: operation.into(),
        }
    }
    /// Durable approved operation ID, for correlation without subject disclosure.
    pub fn operation_id(&self) -> &str {
        &self.operation
    }
    fn matches(&self, subject: &str) -> bool {
        self.target == crate::control::target_commitment(subject.as_bytes())
    }
}

struct State {
    keys: BTreeMap<String, Zeroizing<[u8; 32]>>,
    shredded: BTreeSet<String>,
    index: BTreeMap<String, String>,
}
/// In-process reference linkage vault. Its mutex covers derivation and erasure
/// so a removed key cannot be concurrently used after shred returns. KMS custody,
/// durable backup removal and audited re-identification are separate obligations.
pub struct Vault {
    state: Mutex<State>,
    mode: Mode,
    capacity: usize,
    scope: Option<crate::control::Scope>,
}
impl Vault {
    /// Create an explicitly empty vault with bounded subject/index capacity.
    pub fn new(mode: Mode, capacity: usize) -> Result<Self, KeyError> {
        if capacity == 0 {
            return Err(KeyError::InvalidMaterial);
        }
        Ok(Self {
            state: Mutex::new(State {
                keys: BTreeMap::new(),
                shredded: BTreeSet::new(),
                index: BTreeMap::new(),
            }),
            mode,
            capacity,
            scope: None,
        })
    }
    /// Provision a scope-bound vault eligible for independently approved erasure.
    /// Scope is trusted external configuration, like the frozen stretch mode.
    pub fn new_scoped(
        mode: Mode,
        capacity: usize,
        scope: crate::control::Scope,
    ) -> Result<Self, KeyError> {
        if scope.installation.is_empty() || scope.node.is_empty() {
            return Err(KeyError::InvalidMaterial);
        }
        let mut vault = Self::new(mode, capacity)?;
        vault.scope = Some(scope);
        Ok(vault)
    }
    /// Restore the historical snapshot using explicitly provisioned tenant scope.
    pub fn load_scoped(
        blob: &[u8],
        mode: Mode,
        capacity: usize,
        scope: crate::control::Scope,
    ) -> Result<Self, KeyError> {
        if scope.installation.is_empty() || scope.node.is_empty() {
            return Err(KeyError::InvalidMaterial);
        }
        let mut vault = Self::load(blob, mode, capacity)?;
        vault.scope = Some(scope);
        Ok(vault)
    }
    pub(crate) fn authorizes(&self, permit: &ErasurePermit) -> bool {
        self.scope.as_ref() == Some(&permit.scope)
    }
    /// Strictly restore a secret snapshot. Empty means an explicitly new vault;
    /// corrupt data, invalid keys or resurrected tombstones never become fresh.
    pub fn load(blob: &[u8], mode: Mode, capacity: usize) -> Result<Self, KeyError> {
        let vault = Self::new(mode, capacity)?;
        if blob.is_empty() {
            return Ok(vault);
        }
        if blob.len() > LIMIT / 2 {
            return Err(KeyError::SizeLimit);
        }
        let snapshot: Snapshot = serde_json::from_slice(blob).map_err(|_| KeyError::Malformed)?;
        if snapshot.keys.0.len() + snapshot.shredded.len() > capacity
            || snapshot.index.0.len() > capacity
        {
            return Err(KeyError::SizeLimit);
        }
        let mut state = vault.state.lock().map_err(|_| KeyError::InvalidMaterial)?;
        for (id, key) in snapshot.keys.0 {
            if id.is_empty() || snapshot.shredded.contains(&id) {
                return Err(KeyError::InvalidMaterial);
            }
            let bytes =
                Zeroizing::new(hex::decode(key.as_str()).map_err(|_| KeyError::InvalidMaterial)?);
            let key: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| KeyError::InvalidMaterial)?;
            state.keys.insert(id, Zeroizing::new(key));
        }
        for (handle, id) in snapshot.index.0 {
            if handle.is_empty() || !state.keys.contains_key(&id) || snapshot.shredded.contains(&id)
            {
                return Err(KeyError::InvalidMaterial);
            }
            state.index.insert(handle, id);
        }
        if snapshot.shredded.iter().any(String::is_empty) {
            return Err(KeyError::InvalidMaterial);
        }
        state.shredded = snapshot.shredded;
        drop(state);
        Ok(vault)
    }
    /// Deterministic subject/value mapping with OS-random per-subject keys.
    /// A destroyed subject returns its historical stable tombstone and is
    /// never re-minted. Entropy/KDF failures are errors, never HMAC fallbacks.
    pub fn pseudonymize(&self, subject: &str, value: &str) -> Result<String, KeyError> {
        if subject.is_empty() || subject.len() > 4096 || value.len() > LIMIT / 2 {
            return Err(KeyError::InvalidMaterial);
        }
        let mut state = self.state.lock().map_err(|_| KeyError::InvalidMaterial)?;
        if state.shredded.contains(subject) {
            return Ok(format!(
                "shredded:{}",
                hex::encode(&Sha256::digest(subject.as_bytes())[..6])
            ));
        }
        if !state.keys.contains_key(subject) {
            if state.keys.len() + state.shredded.len() >= self.capacity {
                return Err(KeyError::SizeLimit);
            }
            let mut key = Zeroizing::new([0; 32]);
            getrandom::fill(&mut key[..]).map_err(|_| KeyError::Entropy)?;
            state.keys.insert(subject.to_owned(), key);
        }
        let key = state.keys.get(subject).ok_or(KeyError::InvalidMaterial)?;
        match self.mode {
            Mode::Hmac => {
                let mut mac = Hmac::<Sha256>::new_from_slice(&key[..])
                    .map_err(|_| KeyError::InvalidMaterial)?;
                mac.update(value.as_bytes());
                Ok(hex::encode(mac.finalize().into_bytes()))
            }
            Mode::Scrypt => {
                let mut derived = Zeroizing::new([0; 32]);
                let params =
                    scrypt::Params::new(12, 8, 1).map_err(|_| KeyError::InvalidMaterial)?;
                scrypt::scrypt(&key[..], value.as_bytes(), &params, &mut derived[..])
                    .map_err(|_| KeyError::InvalidMaterial)?;
                Ok(hex::encode(&derived[..]))
            }
        }
    }
    /// Bind a display handle to an already keyed subject. Requires an authorized
    /// caller; conflicting handle ownership is refused rather than overwritten.
    pub fn link(&self, handle: &str, subject: &str) -> Result<(), KeyError> {
        let mut state = self.state.lock().map_err(|_| KeyError::InvalidMaterial)?;
        if handle.is_empty()
            || subject.is_empty()
            || !state.keys.contains_key(subject)
            || state.shredded.contains(subject)
        {
            return Err(KeyError::InvalidMaterial);
        }
        if let Some(existing) = state.index.get(handle) {
            return if existing == subject {
                Ok(())
            } else {
                Err(KeyError::InvalidMaterial)
            };
        }
        if state.index.len() >= self.capacity {
            return Err(KeyError::SizeLimit);
        }
        state.index.insert(handle.into(), subject.into());
        Ok(())
    }
    /// Resolve the isolated reverse index. Runtime must authenticate/audit this
    /// operation; this data object is not a public management endpoint.
    pub fn resolve(&self, handle: &str) -> Result<Option<String>, KeyError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| KeyError::InvalidMaterial)?
            .index
            .get(handle)
            .cloned())
    }
    /// Remove the owned key and reverse links and retain a non-remint tombstone.
    /// Production permits bind a durably approved exact subject and scope.
    /// Persist the resulting snapshot before acknowledging runtime erasure.
    pub fn shred(&self, subject: &str, permit: &ErasurePermit) -> Result<bool, KeyError> {
        if subject.is_empty() || !self.authorizes(permit) || !permit.matches(subject) {
            return Err(KeyError::InvalidMaterial);
        }
        let mut state = self.state.lock().map_err(|_| KeyError::InvalidMaterial)?;
        if !state.keys.contains_key(subject)
            && !state.shredded.contains(subject)
            && state.keys.len() + state.shredded.len() >= self.capacity
        {
            return Err(KeyError::SizeLimit);
        }
        let existed = state.keys.remove(subject).is_some();
        state.shredded.insert(subject.into());
        state.index.retain(|_, id| id != subject);
        Ok(existed)
    }
    /// Secret snapshot, never log it. Keys retain the historical hex encoding;
    /// caller seals and durably persists it, including all shred tombstones.
    pub fn snapshot(&self) -> Result<Zeroizing<Vec<u8>>, KeyError> {
        let state = self.state.lock().map_err(|_| KeyError::InvalidMaterial)?;
        let snapshot = Snapshot {
            keys: UniqueMap(
                state
                    .keys
                    .iter()
                    .map(|(id, key)| (id.clone(), Zeroizing::new(hex::encode(&key[..]))))
                    .collect(),
            ),
            shredded: state.shredded.clone(),
            index: UniqueMap(state.index.clone()),
        };
        let bytes = Zeroizing::new(serde_json::to_vec(&snapshot).map_err(|_| KeyError::Malformed)?);
        if bytes.len() > LIMIT / 2 {
            return Err(KeyError::SizeLimit);
        }
        Ok(bytes)
    }
}

#[derive(Serialize)]
#[serde(transparent)]
struct UniqueMap<T>(BTreeMap<String, T>);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for UniqueMap<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
            type Value = UniqueMap<T>;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("unique object keys")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut access: A,
            ) -> Result<Self::Value, A::Error> {
                let mut result = BTreeMap::new();
                while let Some((key, value)) = access.next_entry::<String, T>()? {
                    if result.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom("duplicate vault key"));
                    }
                }
                Ok(UniqueMap(result))
            }
        }
        d.deserialize_map(Visitor(std::marker::PhantomData))
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    keys: UniqueMap<Zeroizing<String>>,
    #[serde(deserialize_with = "null_tombstones")]
    shredded: BTreeSet<String>,
    index: UniqueMap<String>,
}
fn null_tombstones<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeSet<String>, D::Error> {
    Ok(Option::<BTreeSet<String>>::deserialize(d)?.unwrap_or_default())
}

// Custody cannot use snapshot replacement to bypass the erasure capability.
// Existing keys are immutable; tombstones are monotonic, including on recovery.
pub(super) fn validate_update(
    old: Option<&[u8]>,
    new: &[u8],
    erasure: Option<&ErasurePermit>,
) -> Result<(), KeyError> {
    let old = Vault::load(old.unwrap_or_default(), Mode::Hmac, usize::MAX)?;
    let new = Vault::load(new, Mode::Hmac, usize::MAX)?;
    let old = old.state.lock().map_err(|_| KeyError::InvalidMaterial)?;
    let new = new.state.lock().map_err(|_| KeyError::InvalidMaterial)?;
    if !old.shredded.is_subset(&new.shredded)
        || new
            .shredded
            .difference(&old.shredded)
            .any(|subject| !erasure.is_some_and(|permit| permit.matches(subject)))
        || old.keys.iter().any(|(id, key)| match new.keys.get(id) {
            Some(next) => key.as_ref() != next.as_ref(),
            None => !erasure.is_some_and(|permit| permit.matches(id)) || !new.shredded.contains(id),
        })
    {
        return Err(KeyError::InvalidMaterial);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scope() -> crate::control::Scope {
        crate::control::Scope {
            installation: "test".into(),
            node: "test".into(),
        }
    }
    fn permit(subject: &str) -> ErasurePermit {
        ErasurePermit::issued(subject, "test-operation", &scope())
    }
    fn test_vault(mode: Mode, capacity: usize) -> Result<Vault, KeyError> {
        Vault::new_scoped(mode, capacity, scope())
    }
    #[test]
    fn plain_and_stretched_pseudonyms_match_frozen_source() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/keys.json")).unwrap();
        for case in fixture["pseudonyms"].as_array().unwrap() {
            let mode = if case["stretch"] == true {
                Mode::Scrypt
            } else {
                Mode::Hmac
            };
            let vault =
                Vault::load(&serde_json::to_vec(&case["snapshot"]).unwrap(), mode, 100).unwrap();
            assert_eq!(
                vault
                    .pseudonymize(
                        case["subject"].as_str().unwrap(),
                        case["value"].as_str().unwrap()
                    )
                    .unwrap(),
                case["expected"].as_str().unwrap()
            );
        }
    }
    #[test]
    fn shred_snapshot_restart_and_corrupt_restore_fail_closed() {
        assert!(
            Vault::load(
                b"{\"keys\":{},\"shredded\":null,\"index\":{}}",
                Mode::Hmac,
                10
            )
            .is_ok()
        );
        let vault = test_vault(Mode::Scrypt, 10).unwrap();
        let before = vault.pseudonymize("subject-A", "1.2.3.4").unwrap();
        vault.link("h", "subject-A").unwrap();
        assert_eq!(vault.resolve("h").unwrap(), Some("subject-A".into()));
        assert!(vault.shred("subject-A", &permit("subject-A")).unwrap());
        assert!(vault.resolve("h").unwrap().is_none());
        assert!(vault.link("new-h", "subject-A").is_err());
        let restored = Vault::load(&vault.snapshot().unwrap(), Mode::Scrypt, 10).unwrap();
        let tombstone = restored.pseudonymize("subject-A", "1.2.3.4").unwrap();
        assert_ne!(before, tombstone);
        assert!(tombstone.starts_with("shredded:"));
        for blob in [
            b"bad".as_slice(),
            b"{\"keys\":{},\"keys\":{},\"shredded\":[],\"index\":{}}",
            b"{\"keys\":{\"s\":\"00\"},\"shredded\":[],\"index\":{}}",
        ] {
            assert!(Vault::load(blob, Mode::Hmac, 10).is_err());
        }
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/keys.json")).unwrap();
        let restored = Vault::load(
            &serde_json::to_vec(&fixture["shredded_snapshot"]).unwrap(),
            Mode::Hmac,
            100,
        )
        .unwrap();
        assert_eq!(
            restored.pseudonymize("subject-A", "any").unwrap(),
            fixture["tombstone"].as_str().unwrap()
        );
    }

    #[test]
    fn snapshot_conflicts_and_capacity_cannot_resurrect_linkage() {
        let vault = test_vault(Mode::Hmac, 2).unwrap();
        vault.pseudonymize("a", "value").unwrap();
        vault.pseudonymize("b", "value").unwrap();
        vault.link("handle", "a").unwrap();
        assert!(vault.link("handle", "b").is_err());
        assert_eq!(vault.resolve("handle").unwrap(), Some("a".into()));
        let mut snapshot: serde_json::Value =
            serde_json::from_slice(&vault.snapshot().unwrap()).unwrap();
        snapshot["shredded"] = serde_json::json!(["a"]);
        assert!(matches!(
            Vault::load(&serde_json::to_vec(&snapshot).unwrap(), Mode::Hmac, 10),
            Err(KeyError::InvalidMaterial)
        ));
        snapshot["shredded"] = serde_json::json!([]);
        snapshot["index"]["dangling"] = serde_json::json!("no-such-subject");
        assert!(matches!(
            Vault::load(&serde_json::to_vec(&snapshot).unwrap(), Mode::Hmac, 10),
            Err(KeyError::InvalidMaterial)
        ));
        assert!(matches!(
            Vault::load(
                b"{\"keys\":{\"s\":\"00\",\"s\":\"00\"},\"shredded\":[],\"index\":{}}",
                Mode::Hmac,
                10
            ),
            Err(KeyError::Malformed)
        ));
        vault.shred("a", &permit("a")).unwrap();
        assert!(matches!(
            vault.pseudonymize("c", "value"),
            Err(KeyError::SizeLimit)
        ));
        assert!(
            vault
                .pseudonymize("a", "value")
                .unwrap()
                .starts_with("shredded:")
        );
    }
    #[test]
    fn custody_updates_require_erasure_authority_and_monotonic_tombstones() {
        let vault = test_vault(Mode::Hmac, 10).unwrap();
        vault.pseudonymize("subject", "value").unwrap();
        let before = vault.snapshot().unwrap();
        vault.shred("subject", &permit("subject")).unwrap();
        let after = vault.snapshot().unwrap();
        assert!(validate_update(Some(&before), &after, None).is_err());
        assert!(validate_update(Some(&before), &after, Some(&permit("subject"))).is_ok());
        assert!(validate_update(Some(&after), &before, Some(&permit("subject"))).is_err());
        let replacement = Vault::new(Mode::Hmac, 10).unwrap();
        replacement.pseudonymize("subject", "value").unwrap();
        assert!(
            validate_update(
                Some(&before),
                &replacement.snapshot().unwrap(),
                Some(&permit("subject"))
            )
            .is_err()
        );
    }
}

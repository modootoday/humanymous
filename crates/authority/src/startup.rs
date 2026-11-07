//! Open one audit owner before listener creation; no key provisioning or fallback.
pub mod observer;

use crate::{
    configuration::{Audit, Peer, UnlockSource},
    decision::DurableAudit,
    transport::private_directory,
};
use humanymous_audit::{
    keys::custody::Owner,
    log::{Config, Log, PinnedWitness, SealedLog, SealedLogOptions, UnlockCredential},
    storage::{Wal, WalOptions},
};
use zeroize::Zeroizing;

/// Open the configured source. A required witness is supplied only by a trusted
/// broker; startup JSON cannot mint one. Errors never downgrade sealed custody.
pub fn open_audit(
    audit: &mut Audit,
    peer: &Peer,
    witness: Option<PinnedWitness>,
) -> Result<Box<dyn DurableAudit>, &'static str> {
    audit
        .validate_source()
        .map_err(|_| "invalid audit source")?;
    if let Some(keys) = &mut audit.keys {
        let owner = match peer {
            #[cfg(unix)]
            Peer::Unix { .. } => Owner::CurrentUser,
            #[cfg(windows)]
            Peer::Windows { sid, .. } => Owner::WindowsSid(sid.clone()),
            _ => return Err("audit owner platform mismatch"),
        };
        // Shared owner checks both directories; preserve authority's configured
        // principal check as well, independently of the current process owner.
        if audit.require_external_witness && witness.is_none() {
            return Err("external witness required");
        }
        private_directory(&audit.directory, peer)
            .map_err(|_| "audit directory is not private and canonical")?;
        private_directory(&keys.directory, peer)
            .map_err(|_| "key directory is not private and canonical")?;
        let options = SealedLogOptions {
            key_directory: keys.directory.clone(),
            wal_directory: audit.directory.clone(),
            owner,
            node_id: audit.node_id.clone(),
            checkpoint_every: 1,
            allow_new_history: audit.allow_new,
            trusted_head: audit.trusted_head.clone(),
            require_external_witness: audit.require_external_witness,
        };
        let result = match &mut keys.unlock {
            UnlockSource::InheritedPrivatePipe { passphrase } => {
                SealedLog::open(&options, UnlockCredential::Utf8(passphrase), witness)
            }
            UnlockSource::OsServiceStore {
                service_name,
                credential_id,
            } => {
                #[cfg(windows)]
                {
                    use humanymous_native_security::secret::{
                        ServiceSecretReference, read_service_secret,
                    };
                    let reference = ServiceSecretReference::new(service_name, credential_id)
                        .map_err(|_| "invalid protected credential selector")?;
                    let mut bytes = read_service_secret(&reference)
                        .map_err(|_| "protected service credential unavailable")?;
                    SealedLog::open(&options, UnlockCredential::Bytes(&mut bytes), witness)
                }
                #[cfg(not(windows))]
                {
                    let _ = (service_name, credential_id);
                    return Err("protected service credential backend unsupported");
                }
            }
        };
        return result
            .map(|log| Box::new(log) as Box<dyn DurableAudit>)
            .map_err(|_| "sealed audit authentication/recovery failed");
    }
    if witness.is_some() {
        return Err("inline audit cannot accept external witness");
    }
    private_directory(&audit.directory, peer)
        .map_err(|_| "audit directory is not private and canonical")?;
    let encoded_key = Zeroizing::new(std::mem::take(&mut audit.hmac_key_hex));
    let encoded_seed = Zeroizing::new(std::mem::take(&mut audit.signing_seed_hex));
    let key = Zeroizing::new(hex::decode(encoded_key.as_str()).map_err(|_| "invalid audit key")?);
    let seed =
        Zeroizing::new(hex::decode(encoded_seed.as_str()).map_err(|_| "invalid audit seed")?);
    if !(32..=256).contains(&key.len()) || seed.len() != 32 {
        return Err("invalid audit key size");
    }
    let log = Log::open(
        Wal::open(&audit.directory, WalOptions::default()).map_err(|_| "audit WAL unavailable")?,
        Config {
            node_id: audit.node_id.clone(),
            hmac_key: key.to_vec(),
            signing_seed: seed
                .as_slice()
                .try_into()
                .map_err(|_| "invalid audit seed size")?,
            checkpoint_every: 1,
            witness: None,
        },
        audit.trusted_head.as_ref(),
        audit.allow_new,
    )
    .map_err(|_| "audit authentication/recovery failed")?;
    Ok(Box::new(log))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sealed() -> serde_json::Value {
        json!({"directory":"nonexistent-wal","node_id":"fixture","allow_new":false,
            "trusted_head":null,"require_external_witness":true,
            "keys":{"directory":"nonexistent-keys","unlock":{"mode":"inherited-private-pipe","passphrase":"fixture-secret"}}})
    }

    #[test]
    fn explicit_sources_reject_mixed_absent_and_unknown_unlock() {
        let base = sealed();
        let audit: Audit = serde_json::from_value(base.clone()).unwrap();
        assert!(audit.validate_source().is_ok());
        let mut mixed = base.clone();
        mixed["hmac_key_hex"] = "aa".into();
        assert!(
            serde_json::from_value::<Audit>(mixed)
                .unwrap()
                .validate_source()
                .is_err()
        );
        let mut missing = base.clone();
        missing.as_object_mut().unwrap().remove("keys");
        assert!(
            serde_json::from_value::<Audit>(missing)
                .unwrap()
                .validate_source()
                .is_err()
        );
        for mode in ["file", "env", "private", "unknown"] {
            let mut invalid = base.clone();
            invalid["keys"]["unlock"]["mode"] = mode.into();
            assert!(serde_json::from_value::<Audit>(invalid).is_err());
        }
    }

    #[test]
    fn missing_witness_does_not_consume_unlock_or_open_state() {
        let mut audit: Audit = serde_json::from_value(sealed()).unwrap();
        #[cfg(unix)]
        let peer = Peer::Unix { uid: 0 };
        #[cfg(windows)]
        let peer = Peer::Windows {
            sid: "S-1-5-18".into(),
            pid: 1,
        };
        assert_eq!(
            open_audit(&mut audit, &peer, None).err(),
            Some("external witness required")
        );
        let UnlockSource::InheritedPrivatePipe { passphrase } =
            &audit.keys.as_ref().unwrap().unlock
        else {
            panic!("source changed")
        };
        assert_eq!(passphrase.as_str(), "fixture-secret");
    }
}

use serde::Deserialize;
use std::path::PathBuf;
use zeroize::Zeroizing;

/// Audit metadata and an explicit preprovisioned sealed key source.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Audit {
    /// Private durable WAL directory.
    pub directory: PathBuf,
    /// Stable writer node identifier.
    pub node_id: String,
    /// Existing sealed custody; never initialized by runtime startup.
    pub keys: SealedKeys,
    /// Permission to initialize audit history, not key material.
    pub allow_new: bool,
    /// Independently retained anchor, when provisioned.
    pub trusted_head: Option<crate::audit::checkpoint::Checkpoint>,
    /// Refuse writer-only startup when a trusted broker must supply a witness.
    #[serde(default)]
    pub require_external_witness: bool,
    /// Supervisor-inherited witness pipes for the shipped binary entrypoint.
    #[serde(default)]
    pub witness_input: Option<WitnessInput>,
}

/// Largest accepted pinned-head consistency proof, in hashes.
pub const MAX_WITNESS_PROOF: usize = 64;

/// Inherited witness pipe pair and pinned identities. Private stdin
/// configuration is the only source; there is no environment fallback.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WitnessInput {
    /// Inherited read end carrying witness-to-runtime frames.
    pub read_fd: u32,
    /// Inherited write end carrying runtime-to-witness frames.
    pub write_fd: u32,
    /// Pinned witness public key.
    pub witness_public: [u8; 32],
    /// Pinned expected writer public key.
    pub writer_public: [u8; 32],
    /// Required witness persistence strength.
    pub durability: WitnessInputDurability,
    /// Proof from the configured trusted head to the witness's current head.
    #[serde(default)]
    pub consistency_proof: Vec<crate::audit::merkle::Hash>,
}

/// Configured witness persistence strength.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WitnessInputDurability {
    /// File and directory entry are synced.
    FileAndDirectory,
    /// File synced; namespace durability is not confirmed.
    FileSyncedNamespaceUnconfirmed,
}

impl Audit {
    /// A configured witness input must be required and structurally bounded.
    pub fn witness_input_valid(&self) -> bool {
        let Some(input) = &self.witness_input else {
            return true;
        };
        self.require_external_witness
            && input.read_fd >= 3
            && input.write_fd >= 3
            && input.read_fd != input.write_fd
            && i32::try_from(input.read_fd).is_ok()
            && i32::try_from(input.write_fd).is_ok()
            && input.witness_public != input.writer_public
            && input.consistency_proof.len() <= MAX_WITNESS_PROOF
    }
}
/// Secret startup source; neither Debug nor Serialize is implemented.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedKeys {
    /// Existing private custody directory.
    pub directory: PathBuf,
    /// Private inherited passphrase, consumed before listening.
    pub passphrase: Zeroizing<String>,
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn inline_keys_and_unknown_sources_are_rejected() {
        let mut value = json!({"directory":"private-audit","node_id":"node","keys":{"directory":"private-keys","passphrase":"fixture"},"allow_new":true,"trusted_head":null});
        assert!(serde_json::from_value::<Audit>(value.clone()).is_ok());
        value["hmac_key_hex"] = json!("11");
        assert!(serde_json::from_value::<Audit>(value.clone()).is_err());
        value.as_object_mut().unwrap().remove("hmac_key_hex");
        value["keys"]["allow_create"] = json!(true);
        assert!(serde_json::from_value::<Audit>(value).is_err());
    }
    #[test]
    fn witness_input_requires_flag_distinct_descriptors_and_keys() {
        let (one, two, short) = ([1u8; 32], [2u8; 32], [1u8; 31]);
        let base = json!({"directory":"a","node_id":"n","keys":{"directory":"k","passphrase":"p"},"allow_new":true,"trusted_head":null,
            "require_external_witness":true,
            "witness_input":{"read_fd":3,"write_fd":4,"witness_public":one,"writer_public":two,"durability":"file_and_directory"}});
        let audit: Audit = serde_json::from_value(base.clone()).unwrap();
        assert!(audit.witness_input_valid());
        let input = audit.witness_input.as_ref().unwrap();
        assert_eq!(input.durability, WitnessInputDurability::FileAndDirectory);
        assert!(input.consistency_proof.is_empty());
        let invalid = |path: &[&str], value: serde_json::Value| {
            let mut changed = base.clone();
            let mut slot = &mut changed;
            for key in path {
                slot = &mut slot[*key];
            }
            *slot = value;
            !serde_json::from_value::<Audit>(changed).is_ok_and(|a| a.witness_input_valid())
        };
        assert!(invalid(&["require_external_witness"], json!(false)));
        assert!(invalid(&["witness_input", "read_fd"], json!(2)));
        assert!(invalid(&["witness_input", "write_fd"], json!(0)));
        assert!(invalid(&["witness_input", "write_fd"], json!(3)));
        assert!(invalid(&["witness_input", "read_fd"], json!(-1)));
        assert!(invalid(&["witness_input", "read_fd"], json!(u32::MAX)));
        assert!(invalid(&["witness_input", "writer_public"], json!(one)));
        assert!(invalid(&["witness_input", "witness_public"], json!(short)));
        assert!(invalid(&["witness_input", "durability"], json!("none")));
        assert!(invalid(&["witness_input", "path"], json!("/run/witness")));
        assert!(invalid(
            &["witness_input", "consistency_proof"],
            json!(vec![[0u8; 32]; MAX_WITNESS_PROOF + 1])
        ));
        // Absent input keeps the existing behavior for both flag values.
        let mut legacy = base;
        legacy.as_object_mut().unwrap().remove("witness_input");
        assert!(
            serde_json::from_value::<Audit>(legacy)
                .unwrap()
                .witness_input_valid()
        );
    }
}

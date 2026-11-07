use crate::audit::{
    checkpoint::Checkpoint,
    record::Record,
    verification::{self, Report},
};

/// Explicit startup trust requirements. Public keys must come from trusted
/// installation state, never from the WAL being verified.
pub struct RecoveryPolicy<'a> {
    /// Exact owning node, applied to every record and checkpoint.
    pub node_id: &'a str,
    /// Required secondary record authentication key; empty is rejected.
    pub hmac_key: &'a [u8],
    /// Expected writer identity.
    pub writer_public: &'a [u8; 32],
    /// If configured, all persisted checkpoints need this independent witness.
    pub witness_public: Option<&'a [u8; 32]>,
    /// Previously trusted signed head, from independently protected state.
    /// Its exact presence is required, preventing rollback before this anchor.
    pub trusted_head: Option<&'a Checkpoint>,
    /// Explicit first-install permission. Empty storage is otherwise an error.
    pub allow_new: bool,
}

/// Authenticated startup history. This does not prove that no unanchored tail
/// was deleted: rollback resistance extends only through the trusted head.
pub struct VerifiedHistory {
    records: Vec<Record>,
    checkpoints: Vec<Checkpoint>,
    report: Option<Report>,
}
impl VerifiedHistory {
    /// Verified records in contiguous sequence order.
    pub fn records(&self) -> &[Record] {
        &self.records
    }
    /// Verified monotonically growing checkpoints.
    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }
    /// Last authenticated sequence, or zero for an explicitly new install.
    pub fn sequence(&self) -> u64 {
        self.records.last().map_or(0, |r| r.seq)
    }
    /// Last authenticated linear commitment.
    pub fn head(&self) -> &str {
        self.records.last().map_or("", |r| r.record_hash.as_str())
    }
    /// Detailed verification coverage; None only for an explicitly new install.
    pub fn report(&self) -> Option<&Report> {
        self.report.as_ref()
    }
}

/// Authenticate parsed storage before reconstructing mutable chain state.
/// Errors contain static diagnostics and never record contents or key material.
pub fn verify_history(
    records: Vec<Record>,
    checkpoints: Vec<Checkpoint>,
    policy: RecoveryPolicy<'_>,
) -> Result<VerifiedHistory, &'static str> {
    if policy.node_id.is_empty() || policy.hmac_key.is_empty() {
        return Err("missing audit recovery identity or HMAC key");
    }
    if records.is_empty() {
        if !policy.allow_new || !checkpoints.is_empty() || policy.trusted_head.is_some() {
            return Err("unexpected empty audit history");
        }
        return Ok(VerifiedHistory {
            records,
            checkpoints,
            report: None,
        });
    }
    if records
        .iter()
        .any(|record| record.node_id != policy.node_id)
        || checkpoints.iter().any(|cp| cp.node_id != policy.node_id)
    {
        return Err("audit history belongs to another node");
    }
    let mut previous_size = 0;
    for cp in &checkpoints {
        if cp.tree_size <= previous_size
            || cp.merkle_root.len() != 64
            || !cp
                .merkle_root
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("invalid audit checkpoint progression or Merkle commitment");
        }
        previous_size = cp.tree_size;
    }
    if let Some(anchor) = policy.trusted_head
        && !checkpoints.contains(anchor)
    {
        return Err("trusted audit head missing: rollback or fork");
    }
    let report = match policy.witness_public {
        Some(witness) => verification::verify_with_witness(
            &records,
            &checkpoints,
            policy.hmac_key,
            Some(policy.writer_public),
            witness,
        ),
        None => verification::verify(
            &records,
            &checkpoints,
            policy.hmac_key,
            Some(policy.writer_public),
        ),
    };
    if !report.ok || !report.scope.hmac_checked {
        return Err("audit history authentication failed");
    }
    if policy.witness_public.is_some() && report.scope.witnessed_checkpoints == 0 {
        return Err("required independent audit anchor missing");
    }
    Ok(VerifiedHistory {
        records,
        checkpoints,
        report: Some(report),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{
        merkle::Tree,
        record::{record_hash, record_mac},
    };
    use ed25519_dalek::SigningKey;
    #[test]
    fn recovery_authenticates_identity_keys_anchor_and_required_scope() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let public = key.verifying_key().to_bytes();
        let policy = || RecoveryPolicy {
            node_id: "node",
            hmac_key: b"secret",
            writer_public: &public,
            witness_public: None,
            trusted_head: None,
            allow_new: false,
        };
        assert!(verify_history(vec![], vec![], policy()).is_err());
        let mut initial = policy();
        initial.allow_new = true;
        assert_eq!(
            verify_history(vec![], vec![], initial).unwrap().sequence(),
            0
        );
        let mut record = Record {
            seq: 1,
            node_id: "node".into(),
            ..Default::default()
        };
        record.record_hash = record_hash(&record, "");
        record.hmac = record_mac(&record.record_hash, b"secret").unwrap();
        let leaves = vec![record.record_hash.as_bytes().to_vec()];
        let mut cp = Checkpoint {
            node_id: "node".into(),
            tree_size: 1,
            root: record.record_hash.clone(),
            merkle_root: hex::encode(Tree::new(&leaves).root()),
            ..Default::default()
        };
        cp.sign(&key);
        let mut anchored = policy();
        anchored.trusted_head = Some(&cp);
        let recovered = verify_history(vec![record.clone()], vec![cp.clone()], anchored).unwrap();
        assert_eq!(recovered.sequence(), 1);
        assert_eq!(recovered.head(), record.record_hash);
        let mut missing = policy();
        missing.trusted_head = Some(&cp);
        assert!(verify_history(vec![record.clone()], vec![], missing).is_err());
        let mut wrong_key = policy();
        wrong_key.hmac_key = b"wrong";
        assert!(verify_history(vec![record.clone()], vec![cp.clone()], wrong_key).is_err());
        let mut wrong_node = policy();
        wrong_node.node_id = "other";
        assert!(verify_history(vec![record.clone()], vec![cp.clone()], wrong_node).is_err());
        let mut witnessed = policy();
        witnessed.witness_public = Some(&public);
        assert!(verify_history(vec![record.clone()], vec![], witnessed).is_err());
        assert!(verify_history(vec![record], vec![cp.clone(), cp], policy()).is_err());
    }
}

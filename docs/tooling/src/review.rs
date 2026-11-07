//! Review triage only: never renews receipts or implies author acceptance.
use crate::snapshot::{self, FileIdentity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Deserialize)]
struct Receipt {
    schema: u32,
    reviews: Vec<Entry>,
}
#[derive(Deserialize)]
struct Entry {
    source: String,
    sha256: String,
}

/// Exact working-byte changes relative to the authored source receipt.
#[derive(Debug, Serialize)]
pub struct Delta {
    /// Explicit triage report version.
    pub schema: &'static str,
    /// Exact baseline receipt bytes; equality alone cannot accept their content.
    pub receipt_sha256: String,
    /// Snapshot's observed repository revision; dirty files are separate.
    pub source_sha: String,
    /// Whether the captured repository contained changes.
    pub dirty: bool,
    /// Sources absent from the prior receipt.
    pub added: Vec<String>,
    /// Receipt sources no longer present in the current governed inventory.
    pub removed: Vec<String>,
    /// Current working bytes differ from the receipt's exact byte commitment.
    pub changed: Vec<String>,
    /// Number whose exact working bytes match; not evidence of adequate prose.
    pub byte_identical: usize,
    /// Always false. A comparison cannot approve an authored source review.
    pub documentation_review_accepted: bool,
}

fn governed(path: &str) -> bool {
    matches!(path, "Cargo.toml" | "Cargo.lock")
        || ["crates/", "adapters/", "web/", "build/"]
            .iter()
            .any(|prefix| path.starts_with(prefix))
}

fn compare(bytes: &[u8], current: &[FileIdentity]) -> Result<Delta, String> {
    let receipt: Receipt = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if receipt.schema != 1 {
        return Err("unsupported authored review schema".into());
    }
    let mut previous = BTreeMap::new();
    for entry in receipt.reviews {
        if !governed(&entry.source)
            || entry.sha256.len() != 64
            || !entry
                .sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || previous.insert(entry.source, entry.sha256).is_some()
        {
            return Err("invalid or duplicate authored review entry".into());
        }
    }
    let mut result = Delta {
        schema: "humanymous.pages.review-delta.v1",
        receipt_sha256: hex::encode(Sha256::digest(bytes)),
        source_sha: String::new(),
        dirty: false,
        added: Vec::new(),
        removed: Vec::new(),
        changed: Vec::new(),
        byte_identical: 0,
        documentation_review_accepted: false,
    };
    for file in current.iter().filter(|f| governed(&f.source)) {
        match previous.remove(&file.source) {
            None => result.added.push(file.source.clone()),
            Some(hash) if hash != file.working_sha256 => result.changed.push(file.source.clone()),
            Some(_) => result.byte_identical += 1,
        }
    }
    result.removed = previous.into_keys().collect();
    result.added.sort();
    result.changed.sort();
    Ok(result)
}

/// Compare a stable source capture with the existing receipt, without writing it.
pub fn delta(root: &Path) -> Result<Delta, String> {
    let receipt = root.join("docs/reviews.json");
    let before = fs::read(&receipt).map_err(|e| e.to_string())?;
    let snapshot = snapshot::capture(root)?;
    if fs::read(&receipt).map_err(|e| e.to_string())? != before {
        return Err("review receipt changed during source capture".into());
    }
    let mut result = compare(&before, &snapshot.files)?;
    result.source_sha = snapshot.source_sha;
    result.dirty = snapshot.dirty;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity(path: &str, working: &str) -> FileIdentity {
        FileIdentity {
            source: path.into(),
            working_sha256: working.into(),
            canonical_sha256: "a".repeat(64),
            git_index_sha256: Some("a".repeat(64)),
            canonical_matches_index: true,
            byte_policy: "utf8-lf".into(),
        }
    }
    #[test]
    fn exact_working_bytes_and_inventory_do_not_approve_review() {
        let receipt = serde_json::json!({"schema":1,"reviews":[
            {"source":"crates/changed.rs","sha256":"a".repeat(64)},
            {"source":"crates/same.rs","sha256":"a".repeat(64)},
            {"source":"adapters/deleted.rs","sha256":"a".repeat(64)}]});
        let bytes = serde_json::to_vec(&receipt).unwrap();
        let delta = compare(
            &bytes,
            &[
                identity("crates/changed.rs", &"b".repeat(64)),
                identity("crates/same.rs", &"a".repeat(64)),
                identity("crates/new.rs", &"c".repeat(64)),
                identity("docs/not-governed.md", &"c".repeat(64)),
            ],
        )
        .unwrap();
        assert_eq!(delta.added, ["crates/new.rs"]);
        assert_eq!(delta.removed, ["adapters/deleted.rs"]);
        assert_eq!(delta.changed, ["crates/changed.rs"]);
        assert_eq!(delta.byte_identical, 1);
        assert!(!delta.documentation_review_accepted);
        let mut duplicate = receipt.clone();
        duplicate["reviews"]
            .as_array_mut()
            .unwrap()
            .push(receipt["reviews"][0].clone());
        assert!(compare(&serde_json::to_vec(&duplicate).unwrap(), &[]).is_err());
    }
}

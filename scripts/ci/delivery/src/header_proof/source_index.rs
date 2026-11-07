//! Independent canonical Git blob bytes, never an archive with export attributes.
use super::admission::{Expected, Input};
use crate::{Result, command, require, source};
use std::{collections::BTreeMap, path::Path};

fn public_path(path: &str) -> bool {
    source::safe_relative(path)
        && !path
            .split('/')
            .any(|part| matches!(part, ".spec.local" | ".git"))
        && source::SCOPES.iter().any(|scope| {
            path == *scope
                || path
                    .strip_prefix(scope)
                    .is_some_and(|rest| rest.starts_with('/'))
        })
}
fn inventory(text: &str) -> Result<BTreeMap<String, (String, u64)>> {
    let mut result = BTreeMap::new();
    let mut bytes = 0u64;
    for row in text.lines() {
        let (metadata, path) = row
            .split_once('\t')
            .ok_or("header canonical tree row refused")?;
        let fields: Vec<_> = metadata.split_whitespace().collect();
        require(
            fields.len() == 4
                && matches!(fields[0], "100644" | "100755")
                && fields[1] == "blob"
                && crate::lab_receipts::digest(&serde_json::Value::String(fields[2].into()), 40)
                && public_path(path),
            "header canonical tree input refused",
        )?;
        let size = fields[3]
            .parse::<u64>()
            .map_err(|_| "header canonical blob size refused")?;
        bytes = bytes
            .checked_add(size)
            .ok_or("header canonical input size overflow")?;
        require(
            bytes <= 64 * 1024 * 1024 && result.len() < 4096,
            "header canonical input bound refused",
        )?;
        require(
            result
                .insert(path.into(), (fields[2].into(), size))
                .is_none(),
            "header canonical duplicate path refused",
        )?;
    }
    require(!result.is_empty(), "header canonical source absent")?;
    Ok(result)
}

impl Expected {
    pub(super) fn git(root: &Path, sha: &str, run: u64, attempt: u64, job: &str) -> Result<Self> {
        require(
            crate::lab_receipts::digest(&serde_json::Value::String(sha.into()), 40)
                && run > 0
                && attempt > 0
                && job == "header",
            "header expected Git/run identity refused",
        )?;
        let mut args = vec!["ls-tree", "-r", "-l", "--full-tree", sha, "--"];
        args.extend_from_slice(source::SCOPES);
        let tree = command::text(root, "git", &args)
            .map_err(|_| "header canonical Git inventory failed")?;
        let rows = inventory(&tree)?;
        let mut canonical_args = vec!["ls-tree", "-r", "--full-tree", sha, "--"];
        canonical_args.extend_from_slice(source::SCOPES);
        let canonical = command::text(root, "git", &canonical_args)
            .map_err(|_| "header canonical Git tree failed")?;
        let mut inputs = BTreeMap::new();
        for (path, (object_id, size)) in rows {
            let object = format!("{sha}:{path}");
            let output = command::output(root, "git", &["show", &object])?;
            require(
                output.status.success() && output.stdout.len() as u64 == size,
                "header canonical original Git blob read failed",
            )?;
            inputs.insert(
                path,
                Input {
                    git_object_id: object_id,
                    bytes: size,
                    sha256: source::sha(&output.stdout),
                },
            );
        }
        Ok(Self {
            source_sha: sha.into(),
            run_id: run,
            run_attempt: attempt,
            job_name: job.into(),
            canonical_tree_sha256: source::sha(canonical.as_bytes()),
            inputs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_inventory_refuses_private_symlink_duplicate_and_oversize_inputs() {
        let hash = "a".repeat(40);
        let row = format!("100644 blob {hash} 3\tCargo.lock\n");
        assert_eq!(inventory(&row).unwrap().len(), 1);
        assert!(inventory(&(row.clone() + &row)).is_err());
        for mode in ["120000", "160000"] {
            assert!(inventory(&row.replace("100644", mode)).is_err());
        }
        for path in [
            "../Cargo.lock",
            ".spec.local/private",
            "crates/.spec.local/private",
            "/Cargo.lock",
            "private.env",
        ] {
            assert!(inventory(&row.replace("Cargo.lock", path)).is_err());
        }
        assert!(inventory(&row.replace(" 3\t", " 67108865\t")).is_err());
        assert!(inventory("").is_err());
    }
}

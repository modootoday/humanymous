//! Select only checksum-verified public crates.io archives, never source or Git.
use crate::{Result, require};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};
pub const MAX_BYTES: u64 = 1_073_741_824;
pub struct Archives {
    pub paths: Vec<PathBuf>,
    pub bytes: u64,
}
fn public_source(source: &str) -> bool {
    matches!(
        source,
        "registry+https://github.com/rust-lang/crates.io-index"
            | "registry+sparse+https://index.crates.io/"
    )
}
fn expected(lock: &toml::Value) -> Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    for package in lock["package"].as_array().ok_or("lock_packages_absent")? {
        if !package
            .get("source")
            .and_then(toml::Value::as_str)
            .is_some_and(public_source)
        {
            continue;
        }
        let name = package["name"].as_str().ok_or("archive_identity_invalid")?;
        let version = package["version"]
            .as_str()
            .ok_or("archive_identity_invalid")?;
        require(
            [name, version].iter().all(|value| {
                !value.is_empty()
                    && value.bytes().all(|c| {
                        c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.' | b'+')
                    })
            }),
            "archive_identity_invalid",
        )?;
        let hash = package["checksum"]
            .as_str()
            .ok_or("public_checksum_absent")?;
        require(
            hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit()),
            "public_checksum_invalid",
        )?;
        let file = format!("{name}-{version}.crate");
        if let Some(previous) = result.insert(file, hash.into()) {
            require(previous == hash, "public_checksum_ambiguous")?;
        }
    }
    Ok(result)
}
fn checksum(reader: &mut impl Read, limit: u64) -> Result<String> {
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|_| "archive_read_failed")?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or("archive_size_invalid")?;
        require(total <= limit, "archive_changed_during_read")?;
        hash.update(&buffer[..count]);
    }
    require(total == limit, "archive_changed_during_read")?;
    Ok(format!("{:x}", hash.finalize()))
}
pub fn collect(cache: &Path, lock: &toml::Value) -> Result<Archives> {
    let expected = expected(lock)?;
    let root = cache
        .canonicalize()
        .map_err(|_| "archive_directory_unavailable")?;
    require(
        !cache
            .symlink_metadata()
            .map_err(|_| "archive_directory_unavailable")?
            .file_type()
            .is_symlink(),
        "archive_symlink_refused",
    )?;
    let mut paths = Vec::new();
    let mut bytes = 0u64;
    for entry in std::fs::read_dir(cache).map_err(|_| "archive_directory_unavailable")? {
        let entry = entry.map_err(|_| "archive_directory_unavailable")?;
        let directory = entry.path();
        if !entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with("index.crates.io-"))
        {
            continue;
        }
        let metadata = directory
            .symlink_metadata()
            .map_err(|_| "archive_metadata_unavailable")?;
        require(
            metadata.is_dir()
                && !metadata.file_type().is_symlink()
                && directory
                    .canonicalize()
                    .map_err(|_| "archive_metadata_unavailable")?
                    .starts_with(&root),
            "archive_symlink_refused",
        )?;
        for entry in std::fs::read_dir(&directory).map_err(|_| "archive_directory_unavailable")? {
            let entry = entry.map_err(|_| "archive_directory_unavailable")?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let Some(wanted) = expected.get(name) else {
                continue;
            };
            let path = entry.path();
            let metadata = path
                .symlink_metadata()
                .map_err(|_| "archive_metadata_unavailable")?;
            require(
                metadata.is_file()
                    && !metadata.file_type().is_symlink()
                    && path
                        .canonicalize()
                        .map_err(|_| "archive_metadata_unavailable")?
                        .starts_with(&root),
                "archive_symlink_refused",
            )?;
            bytes = bytes
                .checked_add(metadata.len())
                .ok_or("archive_size_invalid")?;
            require(bytes <= MAX_BYTES, "archive_size_limit")?;
            let mut file = std::fs::File::open(&path).map_err(|_| "archive_read_failed")?;
            require(
                checksum(&mut file, metadata.len())? == *wanted,
                "archive_checksum_mismatch",
            )?;
            let text = path.to_str().ok_or("archive_path_invalid")?;
            require(!text.contains(['\n', '\r']), "archive_path_invalid")?;
            paths.push(path);
        }
    }
    paths.sort();
    require(!paths.is_empty(), "no_verified_public_archives")?;
    Ok(Archives { paths, bytes })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_git_and_alternate_registries_are_not_cache_inputs() {
        let lock:toml::Value=toml::from_str(&format!("[[package]]\nname='public'\nversion='1.0.0'\nsource='registry+https://github.com/rust-lang/crates.io-index'\nchecksum='{}'\n[[package]]\nname='private'\nversion='1.0.0'\nsource='git+https://private.invalid/repo'\n", "a".repeat(64))).unwrap();
        let names = expected(&lock).unwrap();
        assert_eq!(names.len(), 1);
        assert!(names.contains_key("public-1.0.0.crate"));
        assert!(!public_source("registry+https://private.invalid/index"));
    }
    #[test]
    fn archive_hashing_has_exact_bounded_byte_accounting() {
        let bytes = b"public fixture";
        assert_eq!(
            checksum(&mut &bytes[..], bytes.len() as u64).unwrap(),
            crate::source::sha(bytes)
        );
        assert!(checksum(&mut &bytes[..], 1).is_err());
        assert!(checksum(&mut &bytes[..], 100).is_err());
    }
}

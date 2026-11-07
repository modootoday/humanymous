//! Inspect the actual finished compressed archive, never just the source recipe.
use super::{ARCHIVE, HASH, verify_bytes};
use crate::{Result, require};
use serde_json::{Value, json};
use std::{io::Read, path::Path};

fn inspect(reader: impl Read) -> Result<Value> {
    let mut archive = tar::Archive::new(reader);
    let mut observed = None;
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        if entry.path().map_err(|e| e.to_string())?.as_ref() != Path::new(ARCHIVE) {
            continue;
        }
        require(
            observed.is_none()
                && entry.header().entry_type().is_file()
                && entry.size() <= 32 * 1024,
            "distribution notice duplicated, nonregular or oversized",
        )?;
        let mut bytes = Vec::new();
        entry
            .by_ref()
            .take(32 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        verify_bytes(&bytes)?;
        observed = Some(bytes.len());
    }
    let size = observed.ok_or("known Go notice absent from distribution archive")?;
    Ok(json!({"status":"verified","sha256":HASH,"bytes":size}))
}
pub fn verify(path: &Path) -> Result<Value> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    require(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "archive must be a regular file",
    )?;
    inspect_gzip(std::fs::File::open(path).map_err(|e| e.to_string())?)
}
fn inspect_gzip(reader: impl Read) -> Result<Value> {
    let mut decoder = flate2::read::GzDecoder::new(reader);
    let proof = inspect(&mut decoder)?;
    // Tar stops at its end markers; consume the gzip footer to check CRC/length.
    std::io::copy(&mut decoder, &mut std::io::sink()).map_err(|e| e.to_string())?;
    Ok(proof)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(rows: &[(tar::EntryType, &[u8])]) -> Vec<u8> {
        let mut archive = tar::Builder::new(Vec::new());
        for (kind, bytes) in rows {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(*kind);
            header.set_mode(0o644);
            header.set_size(bytes.len() as u64);
            if kind.is_symlink() {
                header.set_link_name("unexpected").unwrap();
            }
            header.set_cksum();
            archive.append_data(&mut header, ARCHIVE, *bytes).unwrap();
        }
        archive.into_inner().unwrap()
    }
    #[test]
    fn finished_archive_requires_one_regular_exact_notice() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let bytes = std::fs::read(root.join(super::super::SOURCE)).unwrap();
        let one = [(tar::EntryType::Regular, bytes.as_slice())];
        assert_eq!(inspect(fixture(&one).as_slice()).unwrap()["sha256"], HASH);
        assert!(inspect(fixture(&[]).as_slice()).is_err());
        assert!(inspect(fixture(&[one[0], one[0]]).as_slice()).is_err());
        assert!(inspect(fixture(&[(tar::EntryType::Regular, b"changed")]).as_slice()).is_err());
        assert!(inspect(fixture(&[(tar::EntryType::Symlink, &[])]).as_slice()).is_err());
        assert!(
            inspect(fixture(&[(tar::EntryType::Regular, &vec![0; 32 * 1024 + 1])]).as_slice())
                .is_err()
        );
        use std::io::Write;
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&fixture(&one)).unwrap();
        let mut gzip = encoder.finish().unwrap();
        assert!(inspect_gzip(gzip.as_slice()).is_ok());
        let footer = gzip.len() - 8;
        gzip[footer] ^= 1;
        assert!(inspect_gzip(gzip.as_slice()).is_err());
        assert!(inspect_gzip(&gzip[..footer]).is_err());
    }
}

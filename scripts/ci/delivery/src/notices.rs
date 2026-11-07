//! Exact distribution of the known adapted Go audit-sort notice.
use crate::{Result, read_json, require, source};
use serde_json::{Value, json};
use std::{io::Write, path::Path};

const SOURCE: &str = "crates/audit/LICENSE-Go.txt";
const HASH: &str = "84ed4f9bae702363a97e52bf76c97cdb809b699254f26670c77080b71a5d7738";
const ARCHIVE: &str = "licenses/Go.txt";
const IMAGE: &str = "/usr/share/licenses/humanymous/Go.txt";

pub fn inventory(root: &Path) -> Result<Value> {
    let inventory = read_json(&root.join("build/rust/distribution-notices.json"))?;
    require(
        inventory["schema"] == "humanymous.distribution-notices/1"
            && inventory["notices"]
                == json!([{
                    "source":SOURCE,"sha256":HASH,"native_archive_path":ARCHIVE,
                    "runtime_image_path":IMAGE
                }])
            && source::file(&root.join(SOURCE))? == HASH,
        "known Go notice source or distribution inventory changed",
    )?;
    Ok(inventory)
}
pub fn image(root: &Path, dockerfile: &str) -> Result<()> {
    inventory(root)?;
    require(
        dockerfile
            .lines()
            .filter(|line| *line == format!("COPY --from=compiler /source/{SOURCE} {IMAGE}"))
            .count()
            == 1,
        "runtime image must include the exact known Go notice",
    )
}
pub fn append<W: Write>(root: &Path, archive: &mut tar::Builder<W>) -> Result<Value> {
    let inventory = inventory(root)?;
    let bytes = std::fs::read(root.join(SOURCE)).map_err(|e| e.to_string())?;
    require(
        source::sha(&bytes) == HASH,
        "Go notice bytes changed before packaging",
    )?;
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    archive
        .append_data(&mut header, ARCHIVE, bytes.as_slice())
        .map_err(|e| e.to_string())?;
    Ok(inventory)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distribution_carries_exact_known_go_notice_bytes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut writer = tar::Builder::new(Vec::new());
        append(&root, &mut writer).unwrap();
        let bytes = writer.into_inner().unwrap();
        let mut archive = tar::Archive::new(bytes.as_slice());
        let mut entries = archive.entries().unwrap();
        let mut entry = entries.next().unwrap().unwrap();
        assert_eq!(entry.path().unwrap().as_ref(), Path::new(ARCHIVE));
        let mut notice = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut notice).unwrap();
        assert_eq!(source::sha(&notice), HASH);
        assert_eq!(notice, std::fs::read(root.join(SOURCE)).unwrap());
        assert!(entries.next().is_none());
        let dockerfile =
            std::fs::read_to_string(root.join("build/rust/runtime-candidate.Dockerfile")).unwrap();
        assert!(image(&root, &dockerfile).is_ok());
        assert!(
            image(
                &root,
                &dockerfile.replace(
                    &format!("COPY --from=compiler /source/{SOURCE} {IMAGE}"),
                    ""
                )
            )
            .is_err()
        );
    }
}

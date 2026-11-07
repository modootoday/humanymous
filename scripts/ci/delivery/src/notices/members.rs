//! Match every native archive member to the bytes observed before packaging.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::Path,
};

pub type Members = BTreeMap<String, (u64, String)>;
#[derive(Debug)]
pub struct Failure {
    member: Option<&'static str>,
    predicate: &'static str,
    expected_bytes: Option<u64>,
    observed_bytes: Option<u64>,
    entry_type: Option<&'static str>,
}
impl Failure {
    fn unknown(predicate: &'static str) -> Self {
        Self {
            member: None,
            predicate,
            expected_bytes: None,
            observed_bytes: None,
            entry_type: None,
        }
    }
    pub fn public(&self) -> Value {
        json!({"status":"failed","member":self.member,"predicate":self.predicate,
            "expected_bytes":self.expected_bytes,"observed_bytes":self.observed_bytes,"entry_type":self.entry_type})
    }
}
fn class(name: &str) -> Option<&'static str> {
    match name {
        "humanymous" | "humanymous.exe" => Some("native-cli"),
        "humanymous-runtime" | "humanymous-runtime.exe" => Some("native-runtime"),
        "LICENSE" => Some("license"),
        "licenses/Go.txt" => Some("go-notice"),
        _ => None,
    }
}
fn kind(kind: tar::EntryType) -> &'static str {
    if kind.is_file() {
        "regular"
    } else if kind.is_hard_link() {
        "hardlink"
    } else if kind.is_symlink() {
        "symlink"
    } else if kind.is_dir() {
        "directory"
    } else if kind.is_gnu_sparse() {
        "gnu-sparse"
    } else {
        "other"
    }
}
fn inspect(reader: impl Read, expected: &Members) -> Result<Value, Failure> {
    let mut tar = tar::Archive::new(reader);
    let mut seen = BTreeSet::new();
    for entry in tar.entries().map_err(|_| Failure::unknown("archive_io"))? {
        let mut entry = entry.map_err(|_| Failure::unknown("archive_io"))?;
        let name = entry
            .path()
            .map_err(|_| Failure::unknown("archive_path"))?
            .to_str()
            .ok_or_else(|| Failure::unknown("archive_path"))?
            .to_owned();
        let (size, hash) = expected
            .get(&name)
            .ok_or_else(|| Failure::unknown("unexpected_member"))?;
        let member = class(&name).ok_or_else(|| Failure::unknown("unexpected_expected_member"))?;
        let failure = |predicate| Failure {
            member: Some(member),
            predicate,
            expected_bytes: Some(*size),
            observed_bytes: Some(entry.size()),
            entry_type: Some(kind(entry.header().entry_type())),
        };
        if !seen.insert(name) {
            return Err(failure("duplicate"));
        }
        if !entry.header().entry_type().is_file() {
            return Err(failure("nonregular"));
        }
        if entry.size() != *size {
            return Err(failure("size_mismatch"));
        }
        let mut digest = Sha256::new();
        let mut buffer = [0; 8192];
        loop {
            let count = entry
                .read(&mut buffer)
                .map_err(|_| Failure::unknown("member_io"))?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        if format!("{:x}", digest.finalize()) != *hash {
            return Err(Failure {
                member: Some(member),
                predicate: "hash_mismatch",
                expected_bytes: Some(*size),
                observed_bytes: Some(entry.size()),
                entry_type: Some(kind(entry.header().entry_type())),
            });
        }
    }
    if seen.len() != expected.len() {
        let missing = expected
            .keys()
            .find(|name| !seen.contains(*name))
            .and_then(|name| class(name));
        return Err(Failure {
            member: missing,
            ..Failure::unknown("missing_member")
        });
    }
    Ok(
        json!({"status":"verified","members":expected.iter().map(|(name,(bytes,sha256))|
        json!({"name":name,"bytes":bytes,"sha256":sha256})).collect::<Vec<_>>()}),
    )
}
pub fn verify(path: &Path, expected: &Members) -> Result<Value, Failure> {
    let mut decoder = flate2::read::GzDecoder::new(
        std::fs::File::open(path).map_err(|_| Failure::unknown("archive_io"))?,
    );
    let proof = inspect(&mut decoder, expected)?;
    std::io::copy(&mut decoder, &mut std::io::sink())
        .map_err(|_| Failure::unknown("gzip_footer"))?;
    Ok(proof)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(names: &[(&str, &[u8])]) -> Vec<u8> {
        let mut tar = tar::Builder::new(Vec::new());
        for (name, bytes) in names {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append_data(&mut header, name, *bytes).unwrap();
        }
        tar.into_inner().unwrap()
    }
    #[test]
    fn archive_contents_cannot_substitute_duplicate_or_omit_observed_binary() {
        let expected = [("humanymous".into(), (3, crate::source::sha(b"bin")))].into();
        assert!(inspect(fixture(&[("humanymous", b"bin")]).as_slice(), &expected).is_ok());
        for rows in [
            vec![],
            vec![("humanymous", b"bad".as_slice())],
            vec![("private-canary/path", b"bin".as_slice())],
            vec![
                ("humanymous", b"bin".as_slice()),
                ("humanymous", b"bin".as_slice()),
            ],
        ] {
            let failure = inspect(fixture(&rows).as_slice(), &expected)
                .unwrap_err()
                .public();
            assert_eq!(failure["status"], "failed");
            assert!(!failure.to_string().contains("private-canary"));
        }
        let wrong_size = inspect(fixture(&[("humanymous", b"longer")]).as_slice(), &expected)
            .unwrap_err()
            .public();
        assert_eq!(wrong_size["member"], "native-cli");
        assert_eq!(wrong_size["predicate"], "size_mismatch");
        assert_eq!(wrong_size["expected_bytes"], 3);
        assert_eq!(wrong_size["observed_bytes"], 6);
        let mut links = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_link_name("private-canary").unwrap();
        header.set_size(0);
        header.set_cksum();
        links
            .append_data(&mut header, "humanymous", &[][..])
            .unwrap();
        let failure = inspect(links.into_inner().unwrap().as_slice(), &expected)
            .unwrap_err()
            .public();
        assert_eq!(failure["predicate"], "nonregular");
        assert_eq!(failure["entry_type"], "symlink");
        assert!(!failure.to_string().contains("private-canary"));
    }
}

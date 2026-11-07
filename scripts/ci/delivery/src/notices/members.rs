//! Match every native archive member to the bytes observed before packaging.
use crate::{Result, require};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::Path,
};

pub type Members = BTreeMap<String, (u64, String)>;
fn inspect(reader: impl Read, expected: &Members) -> Result<Value> {
    let mut tar = tar::Archive::new(reader);
    let mut seen = BTreeSet::new();
    for entry in tar.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let name = entry
            .path()
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("non-UTF8 native archive name")?
            .to_owned();
        let (size, hash) = expected
            .get(&name)
            .ok_or("unexpected native archive member")?;
        require(
            seen.insert(name) && entry.header().entry_type().is_file() && entry.size() == *size,
            "duplicate, nonregular or wrong-size native member",
        )?;
        let mut digest = Sha256::new();
        let mut buffer = [0; 8192];
        loop {
            let count = entry.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        require(
            format!("{:x}", digest.finalize()) == *hash,
            "native archived bytes changed",
        )?;
    }
    require(
        seen.len() == expected.len(),
        "native archive member missing",
    )?;
    Ok(
        json!({"status":"verified","members":expected.iter().map(|(name,(bytes,sha256))|
        json!({"name":name,"bytes":bytes,"sha256":sha256})).collect::<Vec<_>>()}),
    )
}
pub fn verify(path: &Path, expected: &Members) -> Result<Value> {
    let mut decoder =
        flate2::read::GzDecoder::new(std::fs::File::open(path).map_err(|e| e.to_string())?);
    let proof = inspect(&mut decoder, expected)?;
    std::io::copy(&mut decoder, &mut std::io::sink()).map_err(|e| e.to_string())?;
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
            vec![("other", b"bin".as_slice())],
            vec![
                ("humanymous", b"bin".as_slice()),
                ("humanymous", b"bin".as_slice()),
            ],
        ] {
            assert!(inspect(fixture(&rows).as_slice(), &expected).is_err());
        }
    }
}

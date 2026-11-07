//! Only deterministic regular-file USTAR emitted by this package is accepted.
use super::{Files, MAX_ARCHIVE, MAX_FILE, MAX_FILES, MAX_TOTAL, public_path, verify_files};
use std::collections::BTreeSet;

fn octal(field: &mut [u8], value: usize) -> Result<(), String> {
    let value = format!("{value:0width$o}", width = field.len() - 1);
    if value.len() != field.len() - 1 {
        return Err("tar integer budget".into());
    }
    field[..value.len()].copy_from_slice(value.as_bytes());
    Ok(())
}
fn header(name: &str, size: usize) -> Result<[u8; 512], String> {
    if !public_path(name, false) || size > MAX_FILE {
        return Err("tar entry outside public policy".into());
    }
    let (prefix, leaf) = if name.len() <= 100 {
        ("", name)
    } else {
        name.rmatch_indices('/')
            .find_map(|(index, _)| {
                (index <= 155 && name.len() - index - 1 <= 100)
                    .then_some((&name[..index], &name[index + 1..]))
            })
            .ok_or("tar path exceeds USTAR fields")?
    };
    let mut out = [0u8; 512];
    out[..leaf.len()].copy_from_slice(leaf.as_bytes());
    octal(&mut out[100..108], 0o644)?;
    octal(&mut out[108..116], 0)?;
    octal(&mut out[116..124], 0)?;
    octal(&mut out[124..136], size)?;
    octal(&mut out[136..148], 0)?;
    out[148..156].fill(b' ');
    out[156] = b'0';
    out[257..263].copy_from_slice(b"ustar\0");
    out[263..265].copy_from_slice(b"00");
    out[345..345 + prefix.len()].copy_from_slice(prefix.as_bytes());
    let checksum: usize = out.iter().map(|b| usize::from(*b)).sum();
    let digits = format!("{checksum:06o}");
    if digits.len() != 6 {
        return Err("tar checksum budget".into());
    }
    out[148..154].copy_from_slice(digits.as_bytes());
    out[154] = 0;
    out[155] = b' ';
    Ok(out)
}
pub(super) fn encode(files: &Files) -> Result<Vec<u8>, String> {
    verify_files(files)?;
    let mut out = Vec::new();
    for (name, bytes) in files {
        out.extend_from_slice(&header(name, bytes.len())?);
        out.extend_from_slice(bytes);
        out.resize(out.len() + (512 - bytes.len() % 512) % 512, 0);
    }
    out.resize(out.len() + 1024, 0);
    if out.len() > MAX_ARCHIVE {
        return Err("tar archive budget".into());
    }
    Ok(out)
}
fn field(bytes: &[u8]) -> Result<&str, String> {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    if bytes[end..].iter().any(|b| *b != 0) {
        return Err("noncanonical tar field".into());
    }
    std::str::from_utf8(&bytes[..end]).map_err(|e| e.to_string())
}
pub(super) fn decode(bytes: &[u8]) -> Result<Files, String> {
    if bytes.len() > MAX_ARCHIVE || bytes.len() < 1024 || !bytes.len().is_multiple_of(512) {
        return Err("invalid tar archive bounds".into());
    }
    let mut files = Files::new();
    let mut folded = BTreeSet::new();
    let mut offset = 0usize;
    let mut total = 0usize;
    loop {
        let block = bytes
            .get(offset..offset + 512)
            .ok_or("tar end marker missing")?;
        if block.iter().all(|b| *b == 0) {
            if bytes.len() - offset != 1024 || bytes[offset..].iter().any(|b| *b != 0) {
                return Err("noncanonical tar trailer".into());
            }
            break;
        }
        let leaf = field(&block[..100])?;
        let prefix = field(&block[345..500])?;
        let name = if prefix.is_empty() {
            leaf.to_owned()
        } else {
            format!("{prefix}/{leaf}")
        };
        let digits = field(&block[124..136])?;
        if digits.len() != 11 || !digits.bytes().all(|b| (b'0'..=b'7').contains(&b)) {
            return Err("invalid tar size".into());
        }
        let size = usize::from_str_radix(digits, 8).map_err(|e| e.to_string())?;
        if block != header(&name, size)? {
            return Err("unsupported tar header, checksum or entry type".into());
        }
        total = total.checked_add(size).ok_or("tar total overflow")?;
        if total > MAX_TOTAL
            || files.len() >= MAX_FILES
            || !folded.insert(name.to_ascii_lowercase())
        {
            return Err("tar inventory duplicate or budget".into());
        }
        offset += 512;
        let end = offset.checked_add(size).ok_or("tar size overflow")?;
        let content = bytes.get(offset..end).ok_or("truncated tar content")?;
        let padded = end
            .checked_add((512 - size % 512) % 512)
            .ok_or("tar padding overflow")?;
        if bytes
            .get(end..padded)
            .ok_or("truncated tar padding")?
            .iter()
            .any(|b| *b != 0)
        {
            return Err("nonzero tar padding".into());
        }
        files.insert(name, content.to_vec());
        offset = padded;
    }
    verify_files(&files)?;
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn files() -> Files {
        Files::from([
            ("index.html".into(), b"page".to_vec()),
            (".well-known/security.txt".into(), b"contact".to_vec()),
        ])
    }
    #[test]
    fn unsupported_headers_truncation_padding_and_trailing_data_refuse() {
        let valid = encode(&files()).unwrap();
        for index in [100, 148, 156, 157, 257, 500] {
            let mut bad = valid.clone();
            bad[index] ^= 1;
            assert!(decode(&bad).is_err(), "offset {index}");
        }
        let mut bad = valid.clone();
        bad[512 + 7] = 1;
        assert!(decode(&bad).is_err());
        assert!(decode(&valid[..valid.len() - 512]).is_err());
        let mut bad = valid;
        bad.extend_from_slice(&[0; 512]);
        assert!(decode(&bad).is_err());
    }
    #[test]
    fn duplicate_entries_and_missing_hidden_disclosure_refuse() {
        let valid = encode(&files()).unwrap();
        let mut bad = valid[..1024].to_vec();
        bad.extend_from_slice(&valid);
        assert!(decode(&bad).is_err());
        let mut missing = files();
        missing.remove(".well-known/security.txt");
        assert!(encode(&missing).is_err());
    }
    #[test]
    fn ustar_prefix_preserves_long_public_paths_and_rejects_file_budget() {
        let mut source = files();
        source.insert(
            format!("assets/{}/{}.css", "a".repeat(110), "b".repeat(80)),
            b"x".to_vec(),
        );
        assert_eq!(decode(&encode(&source).unwrap()).unwrap(), source);
        assert!(header("assets/data.json", MAX_FILE + 1).is_err());
    }
}

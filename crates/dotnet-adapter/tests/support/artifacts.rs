//! Stock OS digest tools and one bounded, unambiguous SHA256 output parser.
use std::{path::Path, process::Command};

pub fn hash_command(path: &Path) -> Command {
    #[cfg(windows)]
    {
        let root = std::env::var_os("SystemRoot").expect("actual Windows system root");
        let mut command =
            Command::new(std::path::PathBuf::from(root).join("System32/certutil.exe"));
        command.arg("-hashfile").arg(path).arg("SHA256");
        command
    }
    #[cfg(unix)]
    {
        let mut command = Command::new("/usr/bin/sha256sum");
        command.arg(path);
        command
    }
}

pub fn parse_sha256(bytes: &[u8]) -> Result<String, &'static str> {
    if bytes.len() > 65_536 {
        return Err("digest_output_budget");
    }
    let text = String::from_utf8_lossy(bytes);
    let hashes: Vec<_> = text
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .collect();
    if hashes.len() != 1 {
        return Err("ambiguous_or_missing_digest");
    }
    Ok(hashes[0].to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::parse_sha256;
    #[test]
    fn stock_digest_output_requires_one_bounded_commitment() {
        let digest = "a".repeat(64);
        assert_eq!(
            parse_sha256(format!("{digest}  /owned/file\n").as_bytes()),
            Ok(digest.clone())
        );
        assert_eq!(
            parse_sha256(format!("SHA256:\r\n{digest}\r\nCertUtil: completed\r\n").as_bytes()),
            Ok(digest.clone())
        );
        assert!(parse_sha256(format!("{digest}\n{digest}\n").as_bytes()).is_err());
        assert!(parse_sha256(&vec![b'a'; 65_537]).is_err());
        assert!(parse_sha256(b"no digest\n").is_err());
    }
}

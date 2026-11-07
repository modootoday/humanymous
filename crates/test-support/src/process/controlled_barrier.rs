//! Cooperative controlled-fixture metadata, never native producer authority.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
    process::Child,
    time::{Duration, Instant},
};

const SCHEMA: &str = "humanymous.controlled-capture-grant/1";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Grant {
    schema: String,
    nonce: String,
    selector: String,
    pid: u32,
}

fn refused() -> io::Error {
    io::Error::other("controlled fixture grant refused")
}
fn selector_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.split("::").all(|part| {
            part.as_bytes()
                .first()
                .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
                && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
}
fn nonce_valid(value: &str) -> bool {
    value.len() == 32
        && value.bytes().any(|b| b != b'0')
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn encode_claim(nonce: &str, selector: &str, pid: u32) -> io::Result<Vec<u8>> {
    if !nonce_valid(nonce) || !selector_valid(selector) || pid == 0 {
        return Err(refused());
    }
    serde_json::to_vec(&Grant {
        schema: SCHEMA.into(),
        nonce: nonce.into(),
        selector: selector.into(),
        pid,
    })
    .map_err(|_| refused())
}

/// Encode UNVERIFIED synchronization metadata using the actual owned Child PID.
/// This does not prove Job assignment, source, OS supervision or producer identity.
/// Controller independently checks its fixed selector, pinned executable and real
/// native assignment before atomically publishing these bounded bytes.
pub fn encode_unverified_controlled_fixture_grant(
    child: &Child,
    expected_selector: &str,
    nonce: &str,
) -> io::Result<Vec<u8>> {
    encode_claim(nonce, expected_selector, child.id())
}

fn check_claim(bytes: &[u8], nonce: &str, selector: &str, pid: u32) -> io::Result<()> {
    if bytes.len() > 4096 || !nonce_valid(nonce) || !selector_valid(selector) || pid == 0 {
        return Err(refused());
    }
    let grant: Grant = serde_json::from_slice(bytes).map_err(|_| refused())?;
    if grant.schema != SCHEMA
        || grant.nonce != nonce
        || grant.selector != selector
        || grant.pid != pid
    {
        return Err(refused());
    }
    Ok(())
}

/// Wait up to one original five-second threshold BEFORE a controlled test body
/// starts fixture children. Expected selector must be its literal actual test name.
/// File/nonce/PID are cooperative metadata, NOT authenticated OS/Job authority.
/// Local filesystem syscalls may block; independent outer OS supervision and a
/// responsive-kernel premise remain required. No timeout or receipt can mint a
/// production capability. Errors expose no path, nonce or private JSON content.
pub fn await_controlled_fixture_grant(expected_selector: &str) -> io::Result<()> {
    if !selector_valid(expected_selector) {
        return Err(refused());
    }
    let path =
        PathBuf::from(std::env::var_os("HUMANYMOUS_CAPTURE_OUTER_GRANT").ok_or_else(refused)?);
    let nonce = std::env::var("HUMANYMOUS_CAPTURE_OUTER_NONCE").map_err(|_| refused())?;
    if !path.is_absolute() || !nonce_valid(&nonce) {
        return Err(refused());
    }
    let until = Instant::now() + Duration::from_secs(5);
    while Instant::now() < until {
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 4096
                {
                    return Err(refused());
                }
                let mut bytes = Vec::new();
                fs::File::open(&path)
                    .map_err(|_| refused())?
                    .take(4097)
                    .read_to_end(&mut bytes)
                    .map_err(|_| refused())?;
                check_claim(&bytes, &nonce, expected_selector, std::process::id())?;
                if Instant::now() >= until {
                    break;
                }
                return Ok(());
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(_) => return Err(refused()),
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "controlled outer assignment grant absent",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_claim_matches_only_exact_nonce_selector_and_pid() {
        let nonce = "a".repeat(32);
        let bytes = encode_claim(&nonce, "module::test_case", 7).unwrap();
        assert!(check_claim(&bytes, &nonce, "module::test_case", 7).is_ok());
        assert!(check_claim(&bytes, &nonce, "module::other", 7).is_err());
        assert!(check_claim(&bytes, &nonce, "module::test_case", 8).is_err());
        assert!(check_claim(&bytes, &"b".repeat(32), "module::test_case", 7).is_err());
        let text = String::from_utf8(bytes).unwrap();
        assert!(
            check_claim(
                text.replacen('{', "{\"extra\":0,", 1).as_bytes(),
                &nonce,
                "module::test_case",
                7
            )
            .is_err()
        );
        assert!(
            check_claim(
                text.replacen('{', "{\"pid\":7,", 1).as_bytes(),
                &nonce,
                "module::test_case",
                7
            )
            .is_err()
        );
    }
    #[test]
    fn bounded_metadata_has_no_path_or_zero_identity_escape() {
        for selector in ["", "module::::case", "a/b", "a\\b", "0case", "case\0"] {
            assert!(!selector_valid(selector));
        }
        assert!(!selector_valid(&"a".repeat(257)));
        assert!(!nonce_valid(&"0".repeat(32)));
        assert!(encode_claim(&"a".repeat(32), "case", 0).is_err());
        assert!(check_claim(&vec![b' '; 4097], &"a".repeat(32), "case", 7).is_err());
    }
}

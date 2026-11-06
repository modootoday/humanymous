use hmac::{Hmac, Mac};
use sha2::Sha256;

pub(super) fn mac(key: &[u8]) -> Hmac<Sha256> {
    Hmac::new_from_slice(key).expect("HMAC accepts keys of any length")
}

/// Derive the session secret as HMAC(master, UTF-8 session ID).
pub fn session_key(master_key: &[u8], session_id: &str) -> [u8; 32] {
    let mut value = mac(master_key);
    value.update(session_id.as_bytes());
    value.finalize().into_bytes().into()
}

/// Derive seed_n as HMAC(session key, `rit-seed` || big-endian n).
pub fn seed(session_key: &[u8], counter: u64) -> [u8; 32] {
    let mut value = mac(session_key);
    value.update(b"rit-seed");
    value.update(&counter.to_be_bytes());
    value.finalize().into_bytes().into()
}

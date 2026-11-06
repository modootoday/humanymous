//! Session-bound resource URL signatures. Serving policy belongs to enforcement.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

/// Sign session ID, NUL, asset, NUL and decimal expiry; encode the first 16 MAC bytes.
pub fn sign(master_key: &[u8], session_id: &str, asset: &str, expiry_unix: i64) -> String {
    let mut value =
        Hmac::<Sha256>::new_from_slice(master_key).expect("HMAC accepts keys of any length");
    value.update(session_id.as_bytes());
    value.update(&[0]);
    value.update(asset.as_bytes());
    value.update(&[0]);
    value.update(expiry_unix.to_string().as_bytes());
    URL_SAFE_NO_PAD.encode(&value.finalize().into_bytes()[..16])
}

/// Build the legacy resource URL without changing its historical escaping behavior.
/// Inputs must already be safe URL components at the caller boundary.
pub fn signed_url(master_key: &[u8], session_id: &str, asset: &str, expiry_unix: i64) -> String {
    format!(
        "/res/{}?sid={}&exp={}&sig={}",
        asset.strip_prefix('/').unwrap_or(asset),
        session_id,
        expiry_unix,
        sign(master_key, session_id, asset, expiry_unix)
    )
}

/// Verify exact encoded signature and reject only times strictly after expiry.
pub fn verify(
    master_key: &[u8],
    session_id: &str,
    asset: &str,
    expiry_unix: i64,
    now_unix: i64,
    signature: &str,
) -> bool {
    now_unix <= expiry_unix
        && bool::from(
            sign(master_key, session_id, asset, expiry_unix)
                .as_bytes()
                .ct_eq(signature.as_bytes()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_binds_session_asset_key_and_inclusive_expiry() {
        let token = sign(b"key", "sess", "/res/movie.mp4", 2000);
        assert!(verify(b"key", "sess", "/res/movie.mp4", 2000, 1500, &token));
        assert!(verify(b"key", "sess", "/res/movie.mp4", 2000, 2000, &token));
        assert!(!verify(
            b"key",
            "sess",
            "/res/movie.mp4",
            2000,
            2001,
            &token
        ));
        assert!(!verify(
            b"key",
            "other",
            "/res/movie.mp4",
            2000,
            1500,
            &token
        ));
        assert!(!verify(
            b"key",
            "sess",
            "/res/other.mp4",
            2000,
            1500,
            &token
        ));
        assert!(!verify(
            b"other",
            "sess",
            "/res/movie.mp4",
            2000,
            1500,
            &token
        ));
        assert!(!verify(
            b"key",
            "sess",
            "/res/movie.mp4",
            2000,
            1500,
            "tampered"
        ));
    }

    #[test]
    fn url_preserves_signed_asset_and_trims_one_slash_only() {
        assert_eq!(
            signed_url(b"key", "sess", "//asset", -1),
            format!(
                "/res//asset?sid=sess&exp=-1&sig={}",
                sign(b"key", "sess", "//asset", -1)
            )
        );
    }
}

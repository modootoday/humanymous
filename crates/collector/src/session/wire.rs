use super::{Engine, SessionError, URL_SAFE_NO_PAD};
use serde::Deserialize;
use zeroize::Zeroizing;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Bootstrap {
    pub protocol: String,
    pub session: Zeroizing<String>,
    pub key: Zeroizing<String>,
    pub counter: u64,
    pub expires: u64,
    pub installation: String,
    pub tenant: String,
    pub origin: String,
    pub revision: String,
    pub authority: String,
}
pub(super) fn key(value: &str) -> Result<Zeroizing<[u8; 32]>, SessionError> {
    if value.len() != 43 {
        return Err(SessionError::Bootstrap);
    }
    let bytes = Zeroizing::new(
        URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| SessionError::Bootstrap)?,
    );
    let encoded = Zeroizing::new(URL_SAFE_NO_PAD.encode(&bytes));
    if bytes.len() != 32 || *encoded != value {
        return Err(SessionError::Bootstrap);
    }
    let mut key = Zeroizing::new([0; 32]);
    key.copy_from_slice(&bytes);
    Ok(key)
}

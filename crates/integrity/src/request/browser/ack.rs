//! Consumption acknowledgement domain; never an ALLOW/body/collector attestation.
#[cfg(test)]
mod tests;
use super::{BindingError, MAX_SAFE_INTEGER};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// Bind consumed counter, request MAC, bucket and actual final HTTP status.
/// Gateways emit this only after consumption and replace all origin-supplied
/// reserved response headers. It does not attest the response body or ALLOW.
pub fn acknowledgement(
    key: &[u8],
    request_token: &str,
    counter: u64,
    bucket: u64,
    status: u16,
) -> Result<String, BindingError> {
    if key.len() != 32
        || counter == 0
        || counter > MAX_SAFE_INTEGER
        || bucket > MAX_SAFE_INTEGER
        || !(100..=599).contains(&status)
        || request_token.len() != 43
    {
        return Err(BindingError::Acknowledgement);
    }
    let token = URL_SAFE_NO_PAD
        .decode(request_token)
        .map_err(|_| BindingError::Acknowledgement)?;
    if token.len() != 32 || URL_SAFE_NO_PAD.encode(token) != request_token {
        return Err(BindingError::Acknowledgement);
    }
    let canonical = Zeroizing::new(format!(
        "humanymous.browser/0.1/ack\n{}:{}\n{}\n",
        request_token.len(),
        request_token,
        status
    ));
    let derived = Zeroizing::new(super::super::seed(key, counter));
    Ok(super::super::compute_token(
        derived.as_ref(),
        &canonical,
        counter,
        bucket,
    ))
}
/// Constant-time exact MAC check; a failed check cannot establish consumption.
pub fn verify_acknowledgement(
    key: &[u8],
    request_token: &str,
    counter: u64,
    bucket: u64,
    status: u16,
    presented: &str,
) -> bool {
    acknowledgement(key, request_token, counter, bucket, status)
        .is_ok_and(|expected| bool::from(expected.as_bytes().ct_eq(presented.as_bytes())))
}

use super::{Binding, BindingError, MAX_SAFE_INTEGER};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

pub(super) fn header(value: &str, limit: usize) -> Result<&str, BindingError> {
    if value.len() > limit {
        return Err(BindingError::Header);
    }
    let value = value.trim_matches([' ', '\t']);
    if !value.bytes().all(|b| (32..=126).contains(&b)) {
        return Err(BindingError::Header);
    }
    Ok(value)
}

pub(super) fn validate(b: &Binding<'_>) -> Result<(), BindingError> {
    if [b.installation, b.tenant, b.origin_identity, b.revision]
        .iter()
        .any(|s| {
            s.is_empty()
                || s.len() > 128
                || !s
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._:-".contains(&c))
        })
        || b.authority.is_empty()
        || b.authority.len() > 1024
        || !b
            .authority
            .bytes()
            .all(|c| (33..=126).contains(&c) && !b"@/#?\\".contains(&c))
    {
        return Err(BindingError::Scope);
    }
    if b.session.len() != 43 {
        return Err(BindingError::Session);
    }
    let session = URL_SAFE_NO_PAD
        .decode(b.session)
        .map_err(|_| BindingError::Session)?;
    if session.len() != 32 || URL_SAFE_NO_PAD.encode(&session) != b.session {
        return Err(BindingError::Session);
    }
    if !["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"].contains(&b.method)
        || !b.target.is_ascii()
        || crate::adapter::validate_target(b.target).is_err()
    {
        return Err(BindingError::Request);
    }
    if b.counter == 0
        || [b.counter, b.bucket, b.deadline]
            .iter()
            .any(|n| *n > MAX_SAFE_INTEGER)
    {
        return Err(BindingError::Integer);
    }
    if b.body.len() > 1_048_576 {
        return Err(BindingError::Body);
    }
    if ["GET", "HEAD"].contains(&b.method) && !b.body.is_empty() {
        return Err(BindingError::Request);
    }
    Ok(())
}

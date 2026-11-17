//! B02 certificate source: provisioned key pair first, automated issuance only
//! through an explicit domain-bound issuer contract, never a production
//! self-signed fallback. A self-signed development certificate (Go `cert.go`
//! parity) exists only in builds with the `dev-self-signed` feature and only
//! when the operator selects it explicitly.
use rustls::{
    ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    server::ResolvesServerCert,
};
use std::{fmt, fs::File, io::Read, path::PathBuf, sync::Arc};

/// Largest PEM file accepted for a certificate chain or private key.
pub const MAX_PEM_BYTES: u64 = 1 << 20;
/// Largest number of domains one issuer may be asked to cover.
pub const MAX_ISSUER_DOMAINS: usize = 100;
/// ALPN offered on the owned ingress: H2 first, then HTTP/1.1.
pub const INGRESS_ALPN: [&[u8]; 2] = [b"h2", b"http/1.1"];

/// Operator-declared certificate settings before selection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CertificateSettings {
    /// Provisioned PEM certificate chain.
    pub chain: Option<PathBuf>,
    /// Provisioned PEM private key paired with `chain`.
    pub key: Option<PathBuf>,
    /// Domains an automated issuer must cover.
    pub issuer_domains: Vec<String>,
    /// Explicit development-only self-signed certificate; never a fallback.
    #[cfg(feature = "dev-self-signed")]
    pub dev_self_signed: bool,
}

/// The one certificate source the ingress uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertificateSource {
    /// Operator-provided chain and key.
    Provisioned {
        /// PEM chain path.
        chain: PathBuf,
        /// PEM key path.
        key: PathBuf,
    },
    /// Certificates obtained by an accepted issuer for exactly these domains.
    Issuer {
        /// Normalised, deduplicated, sorted domains.
        domains: Vec<String>,
    },
    /// Explicitly selected development certificate for local hosts only.
    #[cfg(feature = "dev-self-signed")]
    DevSelfSigned,
}

/// Why no certificate source or server configuration was produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertificateError {
    /// Neither a provisioned pair nor issuer domains were declared.
    NoSource,
    /// A chain without a key, or a key without a chain.
    IncompletePair,
    /// Both a provisioned pair and issuer domains were declared.
    AmbiguousSource,
    /// An issuer domain is not a plain DNS host name.
    InvalidDomain,
    /// More issuer domains than [`MAX_ISSUER_DOMAINS`].
    TooManyDomains,
    /// A PEM file could not be read or exceeds [`MAX_PEM_BYTES`].
    Unreadable,
    /// The chain holds no certificate.
    EmptyChain,
    /// The key file holds no usable private key.
    MissingKey,
    /// rustls refused the pair, including a key that does not match the chain.
    Rejected,
}

impl fmt::Display for CertificateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoSource => {
                "no certificate source declared; self-signed is not a production fallback"
            }
            Self::IncompletePair => "certificate chain and private key must be provided together",
            Self::AmbiguousSource => {
                "provisioned certificate and issuer domains are mutually exclusive"
            }
            Self::InvalidDomain => "issuer domain is not a plain DNS host name",
            Self::TooManyDomains => "too many issuer domains",
            Self::Unreadable => "certificate or key file unreadable or too large",
            Self::EmptyChain => "certificate chain holds no certificate",
            Self::MissingKey => "private key file holds no private key",
            Self::Rejected => "certificate chain and private key rejected",
        })
    }
}

impl std::error::Error for CertificateError {}

/// Choose exactly one source, refusing incomplete, ambiguous or absent input.
pub fn select(settings: &CertificateSettings) -> Result<CertificateSource, CertificateError> {
    let provisioned = match (&settings.chain, &settings.key) {
        (Some(chain), Some(key)) => Some((chain.clone(), key.clone())),
        (None, None) => None,
        _ => return Err(CertificateError::IncompletePair),
    };
    #[cfg(feature = "dev-self-signed")]
    if settings.dev_self_signed {
        return if provisioned.is_none() && settings.issuer_domains.is_empty() {
            Ok(CertificateSource::DevSelfSigned)
        } else {
            Err(CertificateError::AmbiguousSource)
        };
    }
    match (provisioned, settings.issuer_domains.is_empty()) {
        (Some(_), false) => Err(CertificateError::AmbiguousSource),
        (Some((chain, key)), true) => Ok(CertificateSource::Provisioned { chain, key }),
        (None, true) => Err(CertificateError::NoSource),
        (None, false) => issuer_domains(&settings.issuer_domains)
            .map(|domains| CertificateSource::Issuer { domains }),
    }
}

/// Normalise issuer domains: lower-case DNS host names, no wildcard, IP literal
/// or trailing dot, deduplicated and sorted.
pub fn issuer_domains(raw: &[String]) -> Result<Vec<String>, CertificateError> {
    let mut domains = Vec::with_capacity(raw.len());
    for domain in raw {
        let domain = domain.trim().to_ascii_lowercase();
        if !dns_host(&domain) {
            return Err(CertificateError::InvalidDomain);
        }
        domains.push(domain);
    }
    domains.sort();
    domains.dedup();
    if domains.is_empty() {
        return Err(CertificateError::InvalidDomain);
    }
    if domains.len() > MAX_ISSUER_DOMAINS {
        return Err(CertificateError::TooManyDomains);
    }
    Ok(domains)
}

fn dns_host(domain: &str) -> bool {
    let labels: Vec<&str> = domain.split('.').collect();
    domain.len() <= 253
        && labels.len() >= 2
        && labels.iter().all(|label| {
            (1..=63).contains(&label.len())
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
        && labels
            .last()
            .is_some_and(|tld| !tld.bytes().all(|byte| byte.is_ascii_digit()))
}

/// Build the ingress configuration from a provisioned PEM chain and key.
pub fn provisioned_config(chain: &[u8], key: &[u8]) -> Result<ServerConfig, CertificateError> {
    let certificates = CertificateDer::pem_slice_iter(chain)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CertificateError::EmptyChain)?;
    if certificates.is_empty() {
        return Err(CertificateError::EmptyChain);
    }
    let key = PrivateKeyDer::from_pem_slice(key).map_err(|_| CertificateError::MissingKey)?;
    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certificates, key)
        .map_err(|_| CertificateError::Rejected)?;
    config.alpn_protocols = INGRESS_ALPN
        .iter()
        .map(|protocol| protocol.to_vec())
        .collect();
    Ok(config)
}

/// Read a provisioned pair from disk within [`MAX_PEM_BYTES`] and build the
/// ingress configuration.
pub fn load_provisioned(
    chain: &std::path::Path,
    key: &std::path::Path,
) -> Result<ServerConfig, CertificateError> {
    provisioned_config(&bounded_read(chain)?, &bounded_read(key)?)
}

fn bounded_read(path: &std::path::Path) -> Result<Vec<u8>, CertificateError> {
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(MAX_PEM_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|_| CertificateError::Unreadable)?;
    if bytes.len() as u64 > MAX_PEM_BYTES {
        return Err(CertificateError::Unreadable);
    }
    Ok(bytes)
}

/// Development host names of the self-signed certificate (Go `cert.go`).
#[cfg(feature = "dev-self-signed")]
pub const DEV_HOSTS: [&str; 4] = ["localhost", "humanymous.local", "127.0.0.1", "::1"];

/// Build the ingress configuration with a fresh in-memory ECDSA P-256
/// self-signed certificate, matching Go `selfSignedCert`: subject
/// `humanymous.local`, local host names only, server-auth usage, valid from one
/// hour ago for 365 days. Clients must trust it explicitly.
#[cfg(feature = "dev-self-signed")]
pub fn dev_self_signed_config() -> Result<ServerConfig, CertificateError> {
    let (certificate, key) = dev_self_signed_pair()?;
    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certificate], key)
        .map_err(|_| CertificateError::Rejected)?;
    config.alpn_protocols = INGRESS_ALPN
        .iter()
        .map(|protocol| protocol.to_vec())
        .collect();
    Ok(config)
}

#[cfg(feature = "dev-self-signed")]
fn dev_self_signed_pair()
-> Result<(CertificateDer<'static>, PrivateKeyDer<'static>), CertificateError> {
    let key = rcgen::KeyPair::generate().map_err(|_| CertificateError::Rejected)?;
    let mut params = rcgen::CertificateParams::new(DEV_HOSTS.map(str::to_owned).to_vec())
        .map_err(|_| CertificateError::Rejected)?;
    params.distinguished_name = rcgen::DistinguishedName::new();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "humanymous.local");
    let now = time::OffsetDateTime::from(std::time::SystemTime::now());
    params.not_before = now - time::Duration::hours(1);
    params.not_after = now + time::Duration::days(365);
    params.key_usages = vec![
        rcgen::KeyUsagePurpose::DigitalSignature,
        rcgen::KeyUsagePurpose::KeyEncipherment,
    ];
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
    let certificate = params
        .self_signed(&key)
        .map_err(|_| CertificateError::Rejected)?;
    Ok((
        certificate.der().clone(),
        PrivateKeyDer::Pkcs8(key.serialize_der().into()),
    ))
}

/// Contract for an accepted automated issuer: it owns issuance, renewal and the
/// protected cache, and answers handshakes only for its declared domains.
pub trait CertificateIssuer: Send + Sync {
    /// Domains this issuer was configured for; must equal the selected source.
    fn domains(&self) -> &[String];
    /// Resolver serving issued certificates during the handshake.
    fn resolver(&self) -> Arc<dyn ResolvesServerCert>;
    /// Extra ALPN identifiers the issuer's challenge needs on the same socket.
    fn challenge_alpn(&self) -> Vec<Vec<u8>>;
}

/// Build the ingress configuration from an issuer whose domains match the
/// selected source exactly.
pub fn issuer_config(
    domains: &[String],
    issuer: &dyn CertificateIssuer,
) -> Result<ServerConfig, CertificateError> {
    if issuer.domains() != domains {
        return Err(CertificateError::AmbiguousSource);
    }
    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(issuer.resolver());
    config.alpn_protocols = INGRESS_ALPN
        .iter()
        .map(|protocol| protocol.to_vec())
        .collect();
    config.alpn_protocols.extend(issuer.challenge_alpn());
    Ok(config)
}

#[cfg(test)]
#[path = "certificate_tests.rs"]
mod tests;

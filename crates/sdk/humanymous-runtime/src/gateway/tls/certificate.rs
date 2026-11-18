//! B02 certificate source: provisioned key pair first, automated issuance only
//! through an explicit domain-bound issuer contract, never a production
//! self-signed fallback.
use rustls::{
    ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    server::ResolvesServerCert,
};
use std::{
    fmt,
    fs::File,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex, Weak},
};

/// Largest PEM file accepted for a certificate chain or private key.
pub const MAX_PEM_BYTES: u64 = 1 << 20;
/// Largest number of domains one issuer may be asked to cover.
pub const MAX_ISSUER_DOMAINS: usize = 100;
/// ALPN offered on the owned ingress: H2 first, then HTTP/1.1.
pub const INGRESS_ALPN: [&[u8]; 2] = [b"h2", b"http/1.1"];
/// ALPN identifier of the ACME TLS-ALPN-01 challenge (RFC 8737). The ingress
/// offers it only in a configuration built by [`issuer_config`] for an issuer
/// that declares it, and never carries a request on a connection that
/// negotiates it.
pub const CHALLENGE_ALPN: &[u8] = b"acme-tls/1";

/// Operator-declared certificate settings before selection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CertificateSettings {
    /// Provisioned PEM certificate chain.
    pub chain: Option<PathBuf>,
    /// Provisioned PEM private key paired with `chain`.
    pub key: Option<PathBuf>,
    /// Domains an automated issuer must cover.
    pub issuer_domains: Vec<String>,
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
    /// The issuer asked for challenge ALPN other than exactly `acme-tls/1`.
    UnsupportedChallenge,
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
            Self::UnsupportedChallenge => "issuer challenge ALPN is not exactly acme-tls/1",
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
/// selected source exactly. An issuer may add only the `acme-tls/1` challenge
/// ALPN; its resolver is then recorded as a challenge resolver so the gateway
/// accepts that ALPN for this configuration and no other.
pub fn issuer_config(
    domains: &[String],
    issuer: &dyn CertificateIssuer,
) -> Result<ServerConfig, CertificateError> {
    if issuer.domains() != domains {
        return Err(CertificateError::AmbiguousSource);
    }
    let challenge = issuer.challenge_alpn();
    if !challenge.is_empty() && challenge != [CHALLENGE_ALPN] {
        return Err(CertificateError::UnsupportedChallenge);
    }
    let resolver = issuer.resolver();
    if !challenge.is_empty() {
        record_challenge_resolver(&resolver)?;
    }
    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(resolver);
    config.alpn_protocols = INGRESS_ALPN
        .iter()
        .map(|protocol| protocol.to_vec())
        .collect();
    config.alpn_protocols.extend(challenge);
    Ok(config)
}

/// Resolvers built by [`issuer_config`] for an issuer that answers
/// TLS-ALPN-01. `ServerConfig` carries no record of its certificate source,
/// so the gateway asks here whether a configuration offering `acme-tls/1`
/// came from an ACME issuer. Weak entries keep each allocation (and so its
/// address) reserved while listed; dead entries are pruned on every use.
static CHALLENGE_RESOLVERS: Mutex<Vec<Weak<dyn ResolvesServerCert>>> = Mutex::new(Vec::new());

fn record_challenge_resolver(
    resolver: &Arc<dyn ResolvesServerCert>,
) -> Result<(), CertificateError> {
    let mut resolvers = CHALLENGE_RESOLVERS
        .lock()
        .map_err(|_| CertificateError::Rejected)?;
    resolvers.retain(|entry| entry.strong_count() > 0);
    if !resolvers
        .iter()
        .any(|entry| std::ptr::addr_eq(entry.as_ptr(), Arc::as_ptr(resolver)))
    {
        resolvers.push(Arc::downgrade(resolver));
    }
    Ok(())
}

/// Whether `tls` was built by [`issuer_config`] for an issuer answering the
/// ACME TLS-ALPN-01 challenge. Fails closed when the record is unavailable.
pub(in crate::gateway) fn answers_acme_challenge(tls: &ServerConfig) -> bool {
    let Ok(mut resolvers) = CHALLENGE_RESOLVERS.lock() else {
        return false;
    };
    resolvers.retain(|entry| entry.strong_count() > 0);
    resolvers
        .iter()
        .any(|entry| std::ptr::addr_eq(entry.as_ptr(), Arc::as_ptr(&tls.cert_resolver)))
}

#[cfg(test)]
#[path = "certificate_tests.rs"]
mod tests;

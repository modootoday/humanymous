//! B02 automated issuer with the measured Go `autocert` behaviour: RFC 8555
//! ACME (Let's Encrypt production by default, any operator-chosen directory
//! such as staging or a private CA), TLS-ALPN-01 answered on the ingress socket,
//! an owner-only cache directory, ECDSA P-256 keys and renewal 30 days before
//! `NotAfter`. Handshakes are answered only for the declared domains.
use super::certificate::CertificateIssuer;
use instant_acme::{
    Account, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier, NewAccount,
    NewOrder, OrderStatus, RetryPolicy,
};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    server::{ClientHello, ResolvesServerCert},
    sign::CertifiedKey,
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// ALPN identifier of the TLS-ALPN-01 challenge (RFC 8737).
pub const CHALLENGE_ALPN: &[u8] = b"acme-tls/1";
/// Renew when fewer than this many seconds remain before `NotAfter` (Go
/// `autocert` default: 30 days).
pub const RENEW_BEFORE_SECONDS: i64 = 30 * 24 * 60 * 60;
/// Let's Encrypt production directory, the Go `autocert` default.
pub const LETS_ENCRYPT_PRODUCTION: &str = "https://acme-v02.api.letsencrypt.org/directory";
/// Let's Encrypt staging directory for rehearsals.
pub const LETS_ENCRYPT_STAGING: &str = "https://acme-staging-v02.api.letsencrypt.org/directory";
const MAX_CACHE_BYTES: u64 = 1 << 20;
const CHAIN_FILE: &str = "chain.pem";
const KEY_FILE: &str = "key.pem";
/// Cached PEM chain and key.
type CachedPair = (Vec<u8>, Vec<u8>);

/// Operator settings for the ACME issuer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcmeSettings {
    /// ACME directory URL; `https://` only.
    pub directory: String,
    /// Owner-only cache directory for the account and issued certificate.
    pub cache: PathBuf,
    /// Optional contact address registered with the account.
    pub email: Option<String>,
    /// Optional PEM trust root for a private ACME CA (for example step-ca).
    pub directory_root: Option<PathBuf>,
}

impl AcmeSettings {
    /// Let's Encrypt production with the given cache, like Go `autocert`.
    pub fn lets_encrypt(cache: PathBuf) -> Self {
        Self {
            directory: LETS_ENCRYPT_PRODUCTION.to_owned(),
            cache,
            email: None,
            directory_root: None,
        }
    }
}

/// Why issuance, renewal or the cache failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AcmeError {
    /// Directory URL, contact address or domains are unacceptable.
    Settings(&'static str),
    /// The cache is unreadable, not owner-only, or could not be written.
    Cache(&'static str),
    /// The ACME exchange failed; the detail is the server or client error.
    Protocol(String),
    /// The issued or cached certificate is unusable for the declared domains.
    Certificate(&'static str),
}

impl fmt::Display for AcmeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Settings(detail) => write!(formatter, "ACME settings refused: {detail}"),
            Self::Cache(detail) => write!(formatter, "ACME cache refused: {detail}"),
            Self::Protocol(detail) => write!(formatter, "ACME exchange failed: {detail}"),
            Self::Certificate(detail) => write!(formatter, "ACME certificate refused: {detail}"),
        }
    }
}

impl std::error::Error for AcmeError {}

impl From<instant_acme::Error> for AcmeError {
    fn from(error: instant_acme::Error) -> Self {
        Self::Protocol(error.to_string())
    }
}

/// Current certificate plus in-flight challenge certificates.
#[derive(Debug, Default)]
struct Keys {
    current: Option<Arc<CertifiedKey>>,
    not_after: Option<i64>,
    challenges: HashMap<String, Arc<CertifiedKey>>,
}

/// Handshake resolver shared between the ingress and the issuer.
#[derive(Debug)]
struct Resolver {
    domains: Vec<String>,
    keys: RwLock<Keys>,
}

impl Resolver {
    fn select(&self, server_name: Option<&str>, challenge: bool) -> Option<Arc<CertifiedKey>> {
        let name = server_name?.to_ascii_lowercase();
        if !self.domains.contains(&name) {
            return None;
        }
        let keys = self.keys.read().ok()?;
        if challenge {
            keys.challenges.get(&name).cloned()
        } else {
            keys.current.clone()
        }
    }
}

impl ResolvesServerCert for Resolver {
    fn resolve(&self, hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        // RFC 8737: a validation handshake offers exactly `acme-tls/1`.
        let challenge = hello.alpn().is_some_and(|mut protocols| {
            protocols.next() == Some(CHALLENGE_ALPN) && protocols.next().is_none()
        });
        self.select(hello.server_name(), challenge)
    }
}

/// ACME issuer bound to exactly the selected domains.
pub struct AcmeIssuer {
    settings: AcmeSettings,
    resolver: Arc<Resolver>,
}

impl AcmeIssuer {
    /// Validate settings, prepare the owner-only cache and load a cached
    /// certificate that still covers every domain.
    pub fn new(settings: AcmeSettings, domains: Vec<String>) -> Result<Self, AcmeError> {
        validate(&settings)?;
        if domains.is_empty() {
            return Err(AcmeError::Settings("no domains"));
        }
        private_directory(&settings.cache)?;
        let issuer = Self {
            settings,
            resolver: Arc::new(Resolver {
                domains,
                keys: RwLock::new(Keys::default()),
            }),
        };
        if let Some((chain, key)) = issuer.read_cached()? {
            // A cached pair for other domains is ignored and replaced on issue.
            if let Ok((certified, not_after)) = certified(&chain, &key, &issuer.resolver.domains) {
                issuer.install(certified, not_after)?;
            }
        }
        Ok(issuer)
    }

    /// `NotAfter` of the certificate currently served, as Unix seconds.
    pub fn not_after(&self) -> Option<i64> {
        self.resolver.keys.read().ok()?.not_after
    }

    /// Whether issuance or renewal is due at `now` (Unix seconds).
    pub fn due(&self, now: i64) -> bool {
        renewal_due(self.not_after(), now)
    }

    /// Obtain a certificate when none is served or renewal is due.
    pub async fn ensure(&self) -> Result<(), AcmeError> {
        if self.due(unix_now()) {
            self.issue().await?;
        }
        Ok(())
    }

    /// Keep the certificate current: check every `interval`, retrying failed
    /// renewals at the next check, like the Go `autocert` renewal loop.
    pub async fn maintain(self: Arc<Self>, interval: Duration) {
        loop {
            let _ = self.ensure().await;
            tokio::time::sleep(interval).await;
        }
    }

    /// Run one complete ACME order for every declared domain.
    pub async fn issue(&self) -> Result<(), AcmeError> {
        let account = self.account().await?;
        let identifiers: Vec<Identifier> = self
            .resolver
            .domains
            .iter()
            .map(|domain| Identifier::Dns(domain.clone()))
            .collect();
        let mut order = account.new_order(&NewOrder::new(&identifiers)).await?;
        let result = self.authorize_and_finalize(&mut order).await;
        if let Ok(mut keys) = self.resolver.keys.write() {
            keys.challenges.clear();
        }
        let (chain, key) = result?;
        let (certified, not_after) = certified(chain.as_bytes(), &key, &self.resolver.domains)?;
        self.write_cache(CHAIN_FILE, chain.as_bytes())?;
        self.write_cache(KEY_FILE, &key)?;
        self.install(certified, not_after)
    }

    async fn authorize_and_finalize(
        &self,
        order: &mut instant_acme::Order,
    ) -> Result<(String, Vec<u8>), AcmeError> {
        let mut authorizations = order.authorizations();
        while let Some(authorization) = authorizations.next().await {
            let mut authorization = authorization?;
            match authorization.status {
                AuthorizationStatus::Valid => continue,
                AuthorizationStatus::Pending => {}
                _ => return Err(AcmeError::Protocol("authorization not pending".into())),
            }
            let mut challenge = authorization
                .challenge(ChallengeType::TlsAlpn01)
                .ok_or(AcmeError::Protocol("tls-alpn-01 not offered".into()))?;
            let Identifier::Dns(domain) = challenge.identifier().identifier else {
                return Err(AcmeError::Protocol("non-DNS identifier".into()));
            };
            let domain = domain.clone();
            if !self.resolver.domains.contains(&domain) {
                return Err(AcmeError::Protocol(
                    "authorization for undeclared domain".into(),
                ));
            }
            let response =
                challenge_certificate(&domain, challenge.key_authorization().digest().as_ref())?;
            self.resolver
                .keys
                .write()
                .map_err(|_| AcmeError::Cache("resolver poisoned"))?
                .challenges
                .insert(domain, Arc::new(response));
            challenge.set_ready().await?;
        }
        let retry = RetryPolicy::new().timeout(Duration::from_secs(120));
        if order.poll_ready(&retry).await? != OrderStatus::Ready {
            return Err(AcmeError::Protocol("order not ready".into()));
        }
        let key = rcgen::KeyPair::generate()
            .map_err(|_| AcmeError::Certificate("key generation failed"))?;
        let mut params = rcgen::CertificateParams::new(self.resolver.domains.clone())
            .map_err(|_| AcmeError::Certificate("domains rejected"))?;
        params.distinguished_name = rcgen::DistinguishedName::new();
        let csr = params
            .serialize_request(&key)
            .map_err(|_| AcmeError::Certificate("CSR rejected"))?;
        order.finalize_csr(csr.der()).await?;
        let chain = order.poll_certificate(&retry).await?;
        Ok((chain, pem("PRIVATE KEY", &key.serialize_der()).into_bytes()))
    }

    async fn account(&self) -> Result<Account, AcmeError> {
        let path = self
            .settings
            .cache
            .join(account_file(&self.settings.directory));
        let builder = || match &self.settings.directory_root {
            Some(root) => Account::builder_with_root(root).map_err(AcmeError::from),
            None => Account::builder().map_err(AcmeError::from),
        };
        if let Some(bytes) = read_private(&path)? {
            let credentials: AccountCredentials = serde_json::from_slice(&bytes)
                .map_err(|_| AcmeError::Cache("account credentials unreadable"))?;
            return Ok(builder()?.from_credentials(credentials).await?);
        }
        let contact = self
            .settings
            .email
            .as_ref()
            .map(|email| format!("mailto:{email}"));
        let contacts: Vec<&str> = contact.iter().map(String::as_str).collect();
        // Go `autocert.AcceptTOS` parity: issuance requires accepting the CA terms.
        let (account, credentials) = builder()?
            .create(
                &NewAccount {
                    contact: &contacts,
                    terms_of_service_agreed: true,
                    only_return_existing: false,
                },
                self.settings.directory.clone(),
                None,
            )
            .await?;
        let bytes = serde_json::to_vec(&credentials)
            .map_err(|_| AcmeError::Cache("account credentials unwritable"))?;
        write_private(&self.settings.cache, &path, &bytes)?;
        Ok(account)
    }

    fn read_cached(&self) -> Result<Option<CachedPair>, AcmeError> {
        let chain = read_private(&self.settings.cache.join(CHAIN_FILE))?;
        let key = read_private(&self.settings.cache.join(KEY_FILE))?;
        Ok(chain.zip(key))
    }

    fn write_cache(&self, name: &str, bytes: &[u8]) -> Result<(), AcmeError> {
        write_private(&self.settings.cache, &self.settings.cache.join(name), bytes)
    }

    fn install(&self, certified: CertifiedKey, not_after: i64) -> Result<(), AcmeError> {
        let mut keys = self
            .resolver
            .keys
            .write()
            .map_err(|_| AcmeError::Cache("resolver poisoned"))?;
        keys.current = Some(Arc::new(certified));
        keys.not_after = Some(not_after);
        Ok(())
    }
}

impl CertificateIssuer for AcmeIssuer {
    fn domains(&self) -> &[String] {
        &self.resolver.domains
    }
    fn resolver(&self) -> Arc<dyn ResolvesServerCert> {
        self.resolver.clone()
    }
    fn challenge_alpn(&self) -> Vec<Vec<u8>> {
        vec![CHALLENGE_ALPN.to_vec()]
    }
}

fn validate(settings: &AcmeSettings) -> Result<(), AcmeError> {
    let rest = settings
        .directory
        .strip_prefix("https://")
        .ok_or(AcmeError::Settings("directory must be an https URL"))?;
    if rest.is_empty()
        || settings.directory.len() > 2048
        || settings.directory.chars().any(char::is_whitespace)
    {
        return Err(AcmeError::Settings("directory URL malformed"));
    }
    if let Some(email) = &settings.email {
        let mut parts = email.split('@');
        let valid = matches!(
            (parts.next(), parts.next(), parts.next()),
            (Some(local), Some(host), None) if !local.is_empty() && host.contains('.')
        ) && email.len() <= 254
            && !email.chars().any(|c| c.is_whitespace() || c.is_control());
        if !valid {
            return Err(AcmeError::Settings("contact address malformed"));
        }
    }
    Ok(())
}

/// Whether a certificate expiring at `not_after` must be (re)issued at `now`.
pub fn renewal_due(not_after: Option<i64>, now: i64) -> bool {
    not_after.is_none_or(|not_after| now >= not_after.saturating_sub(RENEW_BEFORE_SECONDS))
}

/// Self-signed TLS-ALPN-01 response certificate for `domain` carrying the
/// critical `id-pe-acmeIdentifier` extension over the key-authorization digest.
pub fn challenge_certificate(domain: &str, digest: &[u8]) -> Result<CertifiedKey, AcmeError> {
    let key =
        rcgen::KeyPair::generate().map_err(|_| AcmeError::Certificate("key generation failed"))?;
    let mut params = rcgen::CertificateParams::new(vec![domain.to_owned()])
        .map_err(|_| AcmeError::Certificate("domain rejected"))?;
    params.custom_extensions = vec![rcgen::CustomExtension::new_acme_identifier(digest)];
    let certificate = params
        .self_signed(&key)
        .map_err(|_| AcmeError::Certificate("challenge certificate rejected"))?;
    let signing = rustls::crypto::ring::sign::any_supported_type(&PrivateKeyDer::Pkcs8(
        key.serialize_der().into(),
    ))
    .map_err(|_| AcmeError::Certificate("challenge key rejected"))?;
    Ok(CertifiedKey::new(vec![certificate.der().clone()], signing))
}

/// Parse an issued chain and key, requiring the leaf to name every domain.
/// Returns the served key and the leaf `NotAfter` (Unix seconds).
pub fn certified(
    chain: &[u8],
    key: &[u8],
    domains: &[String],
) -> Result<(CertifiedKey, i64), AcmeError> {
    let certificates = CertificateDer::pem_slice_iter(chain)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AcmeError::Certificate("chain unreadable"))?;
    let leaf = certificates
        .first()
        .ok_or(AcmeError::Certificate("empty chain"))?;
    let (_, parsed) = x509_parser::parse_x509_certificate(leaf)
        .map_err(|_| AcmeError::Certificate("leaf unreadable"))?;
    let names: Vec<String> = parsed
        .subject_alternative_name()
        .ok()
        .flatten()
        .map(|extension| {
            extension
                .value
                .general_names
                .iter()
                .filter_map(|name| match name {
                    x509_parser::extensions::GeneralName::DNSName(name) => {
                        Some(name.to_ascii_lowercase())
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    if !domains.iter().all(|domain| names.contains(domain)) {
        return Err(AcmeError::Certificate("leaf does not cover every domain"));
    }
    let not_after = parsed.validity().not_after.timestamp();
    let key =
        PrivateKeyDer::from_pem_slice(key).map_err(|_| AcmeError::Certificate("key unreadable"))?;
    let signing = rustls::crypto::ring::sign::any_supported_type(&key)
        .map_err(|_| AcmeError::Certificate("key rejected"))?;
    let certified = CertifiedKey::new(certificates, signing);
    certified
        .keys_match()
        .map_err(|_| AcmeError::Certificate("key does not match leaf"))?;
    Ok((certified, not_after))
}

fn account_file(directory: &str) -> String {
    let digest = Sha256::digest(directory.as_bytes());
    format!("account-{}.json", hex::encode(&digest[..8]))
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

fn pem(label: &str, der: &[u8]) -> String {
    use base64::Engine;
    let body = base64::engine::general_purpose::STANDARD.encode(der);
    let mut out = format!("-----BEGIN {label}-----\n");
    for line in body.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(line).unwrap_or_default());
        out.push('\n');
    }
    out.push_str(&format!("-----END {label}-----\n"));
    out
}

/// Create the cache directory owner-only, or require an existing one to be a
/// real (non-symlink) directory that is owner-only on Unix.
fn private_directory(path: &Path) -> Result<(), AcmeError> {
    if fs::symlink_metadata(path).is_err() {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(path)
            .map_err(|_| AcmeError::Cache("cache directory not creatable"))?;
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| AcmeError::Cache("cache directory unreadable"))?;
    if !metadata.is_dir() {
        return Err(AcmeError::Cache("cache path is not a directory"));
    }
    owner_only(&metadata, 0o700)
}

fn owner_only(metadata: &fs::Metadata, mode: u32) -> Result<(), AcmeError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o777 != mode {
            return Err(AcmeError::Cache("cache entry is not owner-only"));
        }
    }
    #[cfg(not(unix))]
    let _ = (metadata, mode);
    Ok(())
}

fn read_private(path: &Path) -> Result<Option<Vec<u8>>, AcmeError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(AcmeError::Cache("cache entry unreadable")),
    };
    if !metadata.is_file() {
        return Err(AcmeError::Cache("cache entry is not a regular file"));
    }
    owner_only(&metadata, 0o600)?;
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(MAX_CACHE_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|_| AcmeError::Cache("cache entry unreadable"))?;
    if bytes.len() as u64 > MAX_CACHE_BYTES {
        return Err(AcmeError::Cache("cache entry too large"));
    }
    Ok(Some(bytes))
}

/// Write via an owner-only temporary file renamed over the target.
fn write_private(directory: &Path, path: &Path, bytes: &[u8]) -> Result<(), AcmeError> {
    let temporary = directory.join(format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .ok_or(AcmeError::Cache("cache entry name"))?
    ));
    let _ = fs::remove_file(&temporary);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let written = options
        .open(&temporary)
        .and_then(|mut file| file.write_all(bytes).and_then(|()| file.sync_all()));
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(AcmeError::Cache("cache entry unwritable"));
    }
    fs::rename(&temporary, path).map_err(|_| AcmeError::Cache("cache entry not replaced"))
}

#[cfg(test)]
#[path = "acme_tests.rs"]
mod tests;

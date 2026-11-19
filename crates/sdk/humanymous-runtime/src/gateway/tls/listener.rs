//! B02 runtime-binary certificate wiring. The Go `cmd/gate` edge flags
//! (`-tls-cert`/`-tls-key`, `-acme-domain`/`-acme-cache`/`-acme-email`/
//! `-acme-directory`) become the optional `tls` object of the private inherited
//! configuration, which is how the runtime binary takes all of its settings.
//! Its presence requests the TLS listener. Unlike Go, which fell back to a
//! self-signed certificate when no source was given, a request without exactly
//! one source fails closed before custody opens; the development certificate
//! needs both the explicit `dev_self_signed` setting and a build with the
//! `dev-self-signed` feature. ACME needs a build with the `acme` feature; its
//! renewal loop starts only through [`PreparedTls::start_maintenance`].
use super::certificate::{self, CertificateError, CertificateSettings, CertificateSource};
use rustls::ServerConfig;
use serde::Deserialize;
use std::{fmt, path::PathBuf, sync::Arc};

/// Default seconds between ACME renewal checks when none is configured.
pub const DEFAULT_ACME_CHECK_SECONDS: u64 = 60 * 60;

/// Requested TLS listener and its certificate source settings.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListenerTls {
    /// Provisioned PEM certificate chain (Go `-tls-cert`).
    #[serde(default)]
    pub certificate: Option<PathBuf>,
    /// Provisioned PEM private key (Go `-tls-key`).
    #[serde(default)]
    pub key: Option<PathBuf>,
    /// Automated issuance (Go `-acme-*`).
    #[serde(default)]
    pub acme: Option<AcmeListener>,
    /// Explicit development self-signed certificate; never a fallback.
    #[serde(default)]
    pub dev_self_signed: bool,
}

/// ACME issuer settings for the listener.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcmeListener {
    /// Domains to issue for (Go `-acme-domain`, here a list instead of CSV).
    pub domains: Vec<String>,
    /// Owner-only cache directory (Go `-acme-cache`; required here, no
    /// working-directory default).
    pub cache: PathBuf,
    /// Optional account contact (Go `-acme-email`).
    #[serde(default)]
    pub email: Option<String>,
    /// ACME directory URL (Go `-acme-directory`); absent means Let's Encrypt
    /// production, like Go.
    #[serde(default)]
    pub directory: Option<String>,
    /// Optional PEM trust root for a private ACME CA.
    #[serde(default)]
    pub directory_root: Option<PathBuf>,
    /// Seconds between renewal checks; absent means
    /// [`DEFAULT_ACME_CHECK_SECONDS`], zero is refused.
    #[serde(default)]
    pub check_interval_seconds: Option<u64>,
}

/// Why the requested TLS listener was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListenerTlsError {
    /// Source selection or certificate loading refused.
    Certificate(CertificateError),
    /// ACME settings without any domain.
    AcmeWithoutDomains,
    /// A zero ACME check interval.
    AcmeInterval,
    /// ACME selected in a build without the `acme` feature.
    AcmeUnavailable,
    /// The ACME issuer refused its settings or cache.
    AcmeIssuer(String),
}

impl ListenerTlsError {
    /// Fixed refusal text without paths or issuer detail.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Certificate(CertificateError::NoSource) => {
                "tls certificate source absent; self-signed is not a fallback"
            }
            Self::Certificate(CertificateError::IncompletePair) => {
                "tls certificate and key must be provided together"
            }
            Self::Certificate(CertificateError::AmbiguousSource) => {
                "tls certificate source ambiguous"
            }
            Self::Certificate(
                CertificateError::InvalidDomain | CertificateError::TooManyDomains,
            ) => "tls acme domains invalid",
            Self::Certificate(
                CertificateError::Unreadable
                | CertificateError::EmptyChain
                | CertificateError::MissingKey
                | CertificateError::Rejected,
            ) => "tls certificate or key unusable",
            Self::Certificate(CertificateError::UnsupportedChallenge) => {
                "tls issuer challenge unsupported"
            }
            Self::Certificate(CertificateError::DevSelfSignedUnavailable) => {
                "tls dev self-signed certificate unavailable in this build"
            }
            Self::AcmeWithoutDomains => "tls acme domains absent",
            Self::AcmeInterval => "tls acme check interval invalid",
            Self::AcmeUnavailable => "tls acme unavailable in this build",
            Self::AcmeIssuer(_) => "tls acme issuer refused",
        }
    }
}

impl fmt::Display for ListenerTlsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Certificate(error) => error.fmt(formatter),
            Self::AcmeIssuer(detail) => write!(formatter, "{}: {detail}", self.code()),
            _ => formatter.write_str(self.code()),
        }
    }
}

impl std::error::Error for ListenerTlsError {}

impl From<CertificateError> for ListenerTlsError {
    fn from(error: CertificateError) -> Self {
        Self::Certificate(error)
    }
}

/// Map the listener request onto the certificate settings [`certificate::select`] judges.
pub fn settings(tls: &ListenerTls) -> Result<CertificateSettings, ListenerTlsError> {
    let issuer_domains = match &tls.acme {
        Some(acme) if acme.domains.is_empty() => return Err(ListenerTlsError::AcmeWithoutDomains),
        Some(acme) => acme.domains.clone(),
        None => Vec::new(),
    };
    Ok(CertificateSettings {
        chain: tls.certificate.clone(),
        key: tls.key.clone(),
        issuer_domains,
        dev_self_signed: tls.dev_self_signed,
    })
}

/// Choose exactly one source for the listener request.
pub fn select(tls: &ListenerTls) -> Result<CertificateSource, ListenerTlsError> {
    Ok(certificate::select(&settings(tls)?)?)
}

#[cfg(feature = "acme")]
type Maintenance = (Arc<super::acme::AcmeIssuer>, std::time::Duration);
#[cfg(not(feature = "acme"))]
type Maintenance = std::convert::Infallible;

/// The selected source, its ingress configuration and, for ACME, the pending
/// renewal loop.
pub struct PreparedTls {
    source: CertificateSource,
    config: Arc<ServerConfig>,
    maintenance: Option<Maintenance>,
    #[cfg(feature = "acme")]
    issuer: Option<Arc<super::acme::AcmeIssuer>>,
}

impl fmt::Debug for PreparedTls {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedTls")
            .field("source", &self.source)
            .field("maintenance_pending", &self.maintenance.is_some())
            .finish_non_exhaustive()
    }
}

impl PreparedTls {
    /// The one selected source.
    pub fn source(&self) -> &CertificateSource {
        &self.source
    }

    /// Source name as Go logged it in `tls_mode` (`dev_self_signed` for the
    /// explicit development certificate that Go called `self-signed`).
    pub fn mode(&self) -> &'static str {
        match self.source {
            CertificateSource::Provisioned { .. } => "operator_certificate",
            CertificateSource::Issuer { .. } => "acme",
            CertificateSource::DevSelfSigned => "dev_self_signed",
        }
    }

    /// Ingress configuration for [`crate::gateway::service::ServiceHost::bind_tls`].
    pub fn server_config(&self) -> Arc<ServerConfig> {
        self.config.clone()
    }

    /// The ACME issuer serving this listener, if ACME was selected.
    #[cfg(feature = "acme")]
    pub fn issuer(&self) -> Option<&Arc<super::acme::AcmeIssuer>> {
        self.issuer.as_ref()
    }

    /// Spawn [`super::acme::AcmeIssuer::maintain`] on the current Tokio
    /// runtime when ACME was selected. It runs at most once; later calls and
    /// other sources return `None`. Must be called inside a Tokio runtime.
    pub fn start_maintenance(&mut self) -> Option<tokio::task::JoinHandle<()>> {
        let maintenance = self.maintenance.take()?;
        #[cfg(feature = "acme")]
        {
            let (issuer, interval) = maintenance;
            Some(tokio::spawn(issuer.maintain(interval)))
        }
        #[cfg(not(feature = "acme"))]
        match maintenance {}
    }
}

/// Select the source and build its ingress configuration: load a provisioned
/// pair, construct the ACME issuer (cache prepared, no network), or mint the
/// development certificate. Every refusal happens here, before custody.
pub fn prepare(tls: &ListenerTls) -> Result<PreparedTls, ListenerTlsError> {
    let source = select(tls)?;
    let (config, maintenance) = match &source {
        CertificateSource::Provisioned { chain, key } => {
            (certificate::load_provisioned(chain, key)?, None)
        }
        CertificateSource::Issuer { domains } => {
            let acme = tls
                .acme
                .as_ref()
                .ok_or(ListenerTlsError::AcmeWithoutDomains)?;
            let (config, pending) = acme_config(acme, domains)?;
            (config, Some(pending))
        }
        CertificateSource::DevSelfSigned => (dev_config()?, None),
    };
    Ok(PreparedTls {
        source,
        config: Arc::new(config),
        #[cfg(feature = "acme")]
        issuer: maintenance.as_ref().map(|(issuer, _)| issuer.clone()),
        maintenance,
    })
}

#[cfg(feature = "acme")]
fn check_interval(acme: &AcmeListener) -> Result<std::time::Duration, ListenerTlsError> {
    match acme
        .check_interval_seconds
        .unwrap_or(DEFAULT_ACME_CHECK_SECONDS)
    {
        0 => Err(ListenerTlsError::AcmeInterval),
        seconds => Ok(std::time::Duration::from_secs(seconds)),
    }
}

#[cfg(feature = "acme")]
fn acme_config(
    acme: &AcmeListener,
    domains: &[String],
) -> Result<(ServerConfig, Maintenance), ListenerTlsError> {
    use super::acme::{AcmeIssuer, AcmeSettings, LETS_ENCRYPT_PRODUCTION};
    let interval = check_interval(acme)?;
    let settings = AcmeSettings {
        directory: acme
            .directory
            .clone()
            .unwrap_or_else(|| LETS_ENCRYPT_PRODUCTION.to_owned()),
        cache: acme.cache.clone(),
        email: acme.email.clone(),
        directory_root: acme.directory_root.clone(),
    };
    let issuer = AcmeIssuer::new(settings, domains.to_vec())
        .map(Arc::new)
        .map_err(|error| ListenerTlsError::AcmeIssuer(error.to_string()))?;
    let config = certificate::issuer_config(domains, issuer.as_ref())?;
    Ok((config, (issuer, interval)))
}

#[cfg(not(feature = "acme"))]
fn acme_config(
    _: &AcmeListener,
    _: &[String],
) -> Result<(ServerConfig, Maintenance), ListenerTlsError> {
    Err(ListenerTlsError::AcmeUnavailable)
}

#[cfg(feature = "dev-self-signed")]
fn dev_config() -> Result<ServerConfig, ListenerTlsError> {
    Ok(certificate::dev_self_signed_config()?)
}

#[cfg(not(feature = "dev-self-signed"))]
fn dev_config() -> Result<ServerConfig, ListenerTlsError> {
    Err(CertificateError::DevSelfSignedUnavailable.into())
}

#[cfg(test)]
#[path = "listener_tests.rs"]
mod tests;

use super::*;
use std::path::Path;

fn pem(label: &str, der: &[u8]) -> String {
    use base64::Engine;
    let body = base64::engine::general_purpose::STANDARD.encode(der);
    let lines: Vec<&str> = body
        .as_bytes()
        .chunks(64)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect();
    format!(
        "-----BEGIN {label}-----\n{}\n-----END {label}-----\n",
        lines.join("\n")
    )
}

/// Fresh per-test scratch directory under the system temporary directory.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hmn-listener-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn provisioned(dir: &Path) -> ListenerTls {
    std::fs::create_dir_all(dir).unwrap();
    let issued = rcgen::generate_simple_self_signed(vec!["gate.example.com".to_owned()]).unwrap();
    let chain = dir.join("chain.pem");
    let key = dir.join("key.pem");
    std::fs::write(&chain, pem("CERTIFICATE", issued.cert.der())).unwrap();
    std::fs::write(
        &key,
        pem("PRIVATE KEY", &issued.signing_key.serialize_der()),
    )
    .unwrap();
    ListenerTls {
        certificate: Some(chain),
        key: Some(key),
        ..ListenerTls::default()
    }
}

fn acme(domains: &[&str], cache: PathBuf) -> ListenerTls {
    ListenerTls {
        acme: Some(AcmeListener {
            domains: domains.iter().map(|domain| (*domain).to_owned()).collect(),
            cache,
            email: None,
            directory: None,
            directory_root: None,
            check_interval_seconds: None,
        }),
        ..ListenerTls::default()
    }
}

fn alpn(prepared: &PreparedTls) -> Vec<Vec<u8>> {
    prepared.server_config().alpn_protocols.clone()
}

/// The `tls` object of the inherited configuration parses every Go edge flag
/// equivalent and refuses unknown fields; absent keeps plaintext.
#[test]
fn configuration_tls_object_parses_go_edge_settings_and_refuses_unknown_fields() {
    let parsed: ListenerTls = serde_json::from_value(serde_json::json!({
        "certificate": "/etc/humanymous/chain.pem",
        "key": "/etc/humanymous/key.pem",
        "acme": {
            "domains": ["gate.example.com"],
            "cache": "/var/lib/humanymous/acme",
            "email": "ops@example.com",
            "directory": "https://acme-staging-v02.api.letsencrypt.org/directory",
            "directory_root": "/etc/humanymous/ca.pem",
            "check_interval_seconds": 600
        },
        "dev_self_signed": false
    }))
    .unwrap();
    assert_eq!(
        parsed.certificate.as_deref(),
        Some(Path::new("/etc/humanymous/chain.pem"))
    );
    let acme = parsed.acme.unwrap();
    assert_eq!(acme.domains, ["gate.example.com"]);
    assert_eq!(acme.check_interval_seconds, Some(600));
    assert_eq!(
        serde_json::from_value::<ListenerTls>(serde_json::json!({})).unwrap(),
        ListenerTls::default()
    );
    assert!(
        serde_json::from_value::<ListenerTls>(serde_json::json!({"self_signed": true})).is_err()
    );
    assert!(
        serde_json::from_value::<ListenerTls>(
            serde_json::json!({"acme": {"domains": ["a.example"]}})
        )
        .is_err(),
        "the ACME cache has no working-directory default"
    );
    let mut config = crate::gateway::tests::config();
    assert!(config.tls.is_none());
    config.tls = Some(ListenerTls::default());
    assert!(config.validate().is_ok());
}

/// Go `TestBuildEdgeTLSSelfSigned`: Go served a self-signed certificate when
/// no source was set. Rust refuses instead; there is no production fallback.
#[test]
fn no_source_fails_closed_instead_of_self_signing() {
    let error = prepare(&ListenerTls::default()).unwrap_err();
    assert_eq!(
        error,
        ListenerTlsError::Certificate(CertificateError::NoSource)
    );
    assert_eq!(
        error.code(),
        "tls certificate source absent; self-signed is not a fallback"
    );
    let half = ListenerTls {
        certificate: Some(PathBuf::from("/no/such/chain.pem")),
        ..ListenerTls::default()
    };
    assert_eq!(
        prepare(&half).unwrap_err(),
        ListenerTlsError::Certificate(CertificateError::IncompletePair)
    );
}

/// A provisioned pair (Go `-tls-cert`/`-tls-key`) is selected and loaded with
/// a static certificate and no challenge ALPN.
#[test]
fn provisioned_pair_is_selected_and_served_without_challenge_alpn() {
    let dir = scratch("provisioned");
    let mut prepared = prepare(&provisioned(&dir)).unwrap();
    assert!(matches!(
        prepared.source(),
        CertificateSource::Provisioned { .. }
    ));
    assert_eq!(prepared.mode(), "operator_certificate");
    assert_eq!(alpn(&prepared), [b"h2".to_vec(), b"http/1.1".to_vec()]);
    assert!(
        crate::gateway::tls::validate_configuration(
            &crate::gateway::tests::config(),
            &prepared.server_config()
        )
        .is_ok()
    );
    assert!(prepared.start_maintenance().is_none());
    std::fs::remove_dir_all(dir).unwrap();
}

/// Go `TestBuildEdgeTLSBadCert`: a missing pair is a hard error, not a silent
/// self-signed certificate.
#[test]
fn missing_provisioned_files_are_a_hard_error() {
    let missing = ListenerTls {
        certificate: Some(PathBuf::from("/no/such/cert.pem")),
        key: Some(PathBuf::from("/no/such/key.pem")),
        ..ListenerTls::default()
    };
    let error = prepare(&missing).unwrap_err();
    assert_eq!(
        error,
        ListenerTlsError::Certificate(CertificateError::Unreadable)
    );
    assert_eq!(error.code(), "tls certificate or key unusable");
}

/// The development certificate needs the explicit flag and the
/// `dev-self-signed` build; it is exclusive with every other source.
#[test]
fn dev_flag_selects_the_dev_certificate_only_in_dev_builds() {
    let dev = ListenerTls {
        dev_self_signed: true,
        ..ListenerTls::default()
    };
    if cfg!(feature = "dev-self-signed") {
        let prepared = prepare(&dev).unwrap();
        assert_eq!(prepared.source(), &CertificateSource::DevSelfSigned);
        assert_eq!(prepared.mode(), "dev_self_signed");
        assert_eq!(alpn(&prepared), [b"h2".to_vec(), b"http/1.1".to_vec()]);
    } else {
        let error = prepare(&dev).unwrap_err();
        assert_eq!(
            error,
            ListenerTlsError::Certificate(CertificateError::DevSelfSignedUnavailable)
        );
        assert_eq!(
            error.code(),
            "tls dev self-signed certificate unavailable in this build"
        );
    }
    let dir = scratch("dev-mixed");
    let mut mixed = provisioned(&dir);
    mixed.dev_self_signed = true;
    assert_eq!(
        prepare(&mixed).unwrap_err(),
        ListenerTlsError::Certificate(CertificateError::AmbiguousSource)
    );
    let mut mixed = acme(&["gate.example.com"], dir.join("cache"));
    mixed.dev_self_signed = true;
    assert_eq!(
        prepare(&mixed).unwrap_err(),
        ListenerTlsError::Certificate(CertificateError::AmbiguousSource)
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// ACME settings without domains, beside a provisioned pair, or in a build
/// without the `acme` feature are refused.
#[test]
fn acme_without_domains_ambiguous_or_unavailable_is_refused() {
    let dir = scratch("acme-refused");
    assert_eq!(
        prepare(&acme(&[], dir.join("cache"))).unwrap_err(),
        ListenerTlsError::AcmeWithoutDomains
    );
    let mut both = provisioned(&dir);
    both.acme = acme(&["gate.example.com"], dir.join("cache")).acme;
    assert_eq!(
        prepare(&both).unwrap_err(),
        ListenerTlsError::Certificate(CertificateError::AmbiguousSource)
    );
    if !cfg!(feature = "acme") {
        assert_eq!(
            prepare(&acme(&["gate.example.com"], dir.join("cache"))).unwrap_err(),
            ListenerTlsError::AcmeUnavailable
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// Go `TestBuildEdgeTLSACME`: ACME selection serves through the issuer's
/// resolver and offers `acme-tls/1`, which the gateway accepts only here.
#[cfg(feature = "acme")]
#[test]
fn acme_selection_builds_the_challenge_resolver_configuration() {
    let cache = scratch("acme");
    let prepared = prepare(&acme(&["edge.example.com"], cache.clone())).unwrap();
    assert_eq!(
        prepared.source(),
        &CertificateSource::Issuer {
            domains: vec!["edge.example.com".to_owned()]
        }
    );
    assert_eq!(prepared.mode(), "acme");
    assert_eq!(
        alpn(&prepared),
        [
            b"h2".to_vec(),
            b"http/1.1".to_vec(),
            certificate::CHALLENGE_ALPN.to_vec()
        ]
    );
    assert!(
        crate::gateway::tls::validate_configuration(
            &crate::gateway::tests::config(),
            &prepared.server_config()
        )
        .is_ok()
    );
    let issuer = prepared.issuer().unwrap();
    assert_eq!(
        issuer.renewal_facts(),
        crate::gateway::tls::acme::RenewalFacts::default()
    );
    assert!(cache.is_dir(), "the issuer prepares its owner-only cache");
    std::fs::remove_dir_all(cache).unwrap();
}

/// Go `TestBuildEdgeTLSACMEStagingDirectory`: a custom directory (Let's
/// Encrypt staging) still yields the ACME configuration; a non-https directory
/// or a zero check interval is refused before any network use.
#[cfg(feature = "acme")]
#[test]
fn acme_staging_directory_is_honoured_and_bad_settings_are_refused() {
    use crate::gateway::tls::acme::LETS_ENCRYPT_STAGING;
    let cache = scratch("acme-staging");
    let mut staging = acme(&["edge.example.com"], cache.clone());
    if let Some(settings) = staging.acme.as_mut() {
        settings.directory = Some(LETS_ENCRYPT_STAGING.to_owned());
    }
    let prepared = prepare(&staging).unwrap();
    assert!(alpn(&prepared).contains(&certificate::CHALLENGE_ALPN.to_vec()));
    let mut plain = staging.clone();
    if let Some(settings) = plain.acme.as_mut() {
        settings.directory = Some("http://acme.example/directory".to_owned());
    }
    let error = prepare(&plain).unwrap_err();
    assert!(matches!(error, ListenerTlsError::AcmeIssuer(_)));
    assert_eq!(error.code(), "tls acme issuer refused");
    let mut zero = staging;
    if let Some(settings) = zero.acme.as_mut() {
        settings.check_interval_seconds = Some(0);
    }
    assert_eq!(prepare(&zero).unwrap_err(), ListenerTlsError::AcmeInterval);
    std::fs::remove_dir_all(cache).unwrap();
}

/// Go `TestSplitDomains`: Go split a CSV flag and skipped blank entries. The
/// JSON list is trimmed and normalised, but a blank entry is refused rather
/// than skipped, and a wildcard or IP literal never reaches the issuer.
#[test]
fn acme_domains_are_trimmed_and_blank_entries_refused() {
    let dir = scratch("acme-domains");
    let source = select(&acme(&[" A.example.com ", "b.example.com"], dir.clone())).unwrap();
    assert_eq!(
        source,
        CertificateSource::Issuer {
            domains: vec!["a.example.com".to_owned(), "b.example.com".to_owned()]
        }
    );
    for bad in [
        &["a.example.com", " "][..],
        &["*.example.com"],
        &["127.0.0.1"],
    ] {
        assert_eq!(
            select(&acme(bad, dir.clone())).unwrap_err(),
            ListenerTlsError::Certificate(CertificateError::InvalidDomain)
        );
    }
    assert!(!dir.exists(), "selection alone never creates the cache");
}

/// Selecting ACME starts the renewal loop exactly once beside the listener.
#[cfg(feature = "acme")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn acme_selection_starts_maintenance_once() {
    let cache = scratch("acme-maintain");
    let mut settings = acme(&["edge.example.com"], cache.clone());
    if let Some(acme) = settings.acme.as_mut() {
        // Nothing listens on port 1: the first renewal check fails fast.
        acme.directory = Some("https://127.0.0.1:1/directory".to_owned());
    }
    let mut prepared = prepare(&settings).unwrap();
    let issuer = prepared.issuer().unwrap().clone();
    assert_eq!(issuer.renewal_facts().attempts, 0, "prepare makes no order");
    let maintenance = prepared
        .start_maintenance()
        .expect("ACME starts maintenance");
    assert!(
        prepared.start_maintenance().is_none(),
        "started at most once"
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while issuer.renewal_facts().attempts == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "maintenance never ran"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    maintenance.abort();
    let facts = issuer.renewal_facts();
    assert_eq!(facts.generation, 0);
    assert!(facts.last_failure.is_some());
    std::fs::remove_dir_all(cache).unwrap();
}

/// A configuration that requests TLS never binds a plaintext listener: the
/// service refuses it before custody when no server configuration is given.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn service_refuses_a_tls_request_without_server_configuration() {
    let mut config = crate::gateway::tests::config();
    config.tls = Some(ListenerTls::default());
    let error = crate::gateway::service::ServiceHost::bind(config)
        .await
        .err()
        .expect("TLS request refused");
    assert_eq!(
        error.to_string(),
        "TLS listener requested without a certificate configuration"
    );
}

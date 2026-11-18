use super::*;
use rustls::{ClientConfig, ClientConnection, RootCertStore, ServerConfig, ServerConnection};

fn issued(names: &[&str]) -> (String, String, CertificateDer<'static>) {
    let issued = rcgen::generate_simple_self_signed(
        names
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    (
        pem("CERTIFICATE", issued.cert.der()),
        pem("PRIVATE KEY", &issued.signing_key.serialize_der()),
        issued.cert.der().clone(),
    )
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hmn-acme-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn domains(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn settings_accept_https_directories_and_plain_contacts_only() {
    let mut settings = AcmeSettings::lets_encrypt(PathBuf::from("cache"));
    assert_eq!(settings.directory, LETS_ENCRYPT_PRODUCTION);
    assert!(validate(&settings).is_ok());
    settings.directory = LETS_ENCRYPT_STAGING.into();
    settings.email = Some("ops@example.com".into());
    assert!(validate(&settings).is_ok());
    for bad in [
        "http://acme.example/directory",
        "https://",
        "https://a b",
        "",
    ] {
        settings.directory = bad.into();
        assert!(validate(&settings).is_err(), "{bad}");
    }
    settings.directory = LETS_ENCRYPT_STAGING.into();
    for bad in [
        "ops",
        "@example.com",
        "ops@localhost",
        "a@b@c.com",
        "o ps@example.com",
    ] {
        settings.email = Some(bad.into());
        assert!(validate(&settings).is_err(), "{bad}");
    }
}

#[test]
fn renewal_is_due_when_absent_or_within_thirty_days() {
    // A 90-day leaf keeps the Go autocert 30-day window.
    let not_after = 10_000_000;
    let ninety_days = Validity {
        not_before: not_after - 90 * 24 * 60 * 60,
        not_after,
    };
    assert_eq!(renewal_window(ninety_days), RENEW_BEFORE_SECONDS);
    assert!(renewal_due(None, 0));
    assert!(!renewal_due(
        Some(ninety_days),
        not_after - RENEW_BEFORE_SECONDS - 1
    ));
    assert!(renewal_due(
        Some(ninety_days),
        not_after - RENEW_BEFORE_SECONDS
    ));
    assert!(renewal_due(Some(ninety_days), not_after + 1));
    assert!(renewal_due(
        Some(Validity {
            not_before: i64::MIN,
            not_after: i64::MIN
        }),
        0
    ));
}

#[test]
fn short_lived_leaf_is_not_due_right_after_issuance() {
    // step-ca default: 24-hour leaves renew in their last third (8 hours).
    let issued_at = 1_000_000;
    let day = Validity {
        not_before: issued_at,
        not_after: issued_at + 24 * 60 * 60,
    };
    assert_eq!(renewal_window(day), 8 * 60 * 60);
    assert!(!renewal_due(Some(day), issued_at));
    assert!(!renewal_due(Some(day), issued_at + 16 * 60 * 60 - 1));
    assert!(renewal_due(Some(day), issued_at + 16 * 60 * 60));
    // Pebble's 6-day shortlived profile is likewise not due when issued.
    let six_days = Validity {
        not_before: issued_at,
        not_after: issued_at + 6 * 24 * 60 * 60,
    };
    assert!(!renewal_due(Some(six_days), issued_at));
    // A leaf cached on disk: the loaded validity drives the same window.
    let cache = scratch("short");
    let key = rcgen::KeyPair::generate().unwrap();
    let mut params = rcgen::CertificateParams::new(vec!["example.com".to_owned()]).unwrap();
    let now = time::OffsetDateTime::from(SystemTime::now());
    params.not_before = now - time::Duration::minutes(1);
    params.not_after = now + time::Duration::hours(24);
    let certificate = params.self_signed(&key).unwrap();
    let settings = AcmeSettings::lets_encrypt(cache.clone());
    let names = domains(&["example.com"]);
    let issuer = AcmeIssuer::new(settings.clone(), names.clone()).unwrap();
    issuer
        .write_cache(CHAIN_FILE, pem("CERTIFICATE", certificate.der()).as_bytes())
        .unwrap();
    issuer
        .write_cache(
            KEY_FILE,
            pem("PRIVATE KEY", &key.serialize_der()).as_bytes(),
        )
        .unwrap();
    let issuer = AcmeIssuer::new(settings, names).unwrap();
    assert!(issuer.validity().is_some());
    assert!(!issuer.due(unix_now()));
    fs::remove_dir_all(cache).unwrap();
}

#[test]
fn failed_attempts_back_off_exponentially() {
    let interval = Duration::from_secs(60);
    assert_eq!(retry_delay(interval, 0), interval);
    assert_eq!(retry_delay(interval, 1), Duration::from_secs(120));
    assert_eq!(retry_delay(interval, 2), Duration::from_secs(240));
    assert_eq!(retry_delay(interval, 3), Duration::from_secs(480));
    assert_eq!(retry_delay(interval, 20), MAX_RETRY_BACKOFF);
    assert_eq!(retry_delay(interval, u32::MAX), MAX_RETRY_BACKOFF);
    // An interval longer than the cap is never shortened.
    let daily = Duration::from_secs(24 * 60 * 60);
    assert_eq!(retry_delay(daily, 5), daily);
}

#[test]
fn challenge_certificate_names_the_domain_and_carries_the_critical_digest() {
    let digest = Sha256::digest(b"token.thumbprint");
    let response = challenge_certificate("example.com", &digest).unwrap();
    let (_, parsed) = x509_parser::parse_x509_certificate(&response.cert[0]).unwrap();
    let acme = parsed
        .extensions()
        .iter()
        .find(|extension| extension.oid.to_id_string() == "1.3.6.1.5.5.7.1.31")
        .expect("id-pe-acmeIdentifier");
    assert!(acme.critical);
    // DER OCTET STRING (0x04, length 32) wrapping the SHA-256 digest.
    assert_eq!(&acme.value[..2], &[0x04, 0x20]);
    assert_eq!(&acme.value[2..], digest.as_slice());
    let san = parsed.subject_alternative_name().unwrap().unwrap();
    assert_eq!(
        san.value.general_names,
        vec![x509_parser::extensions::GeneralName::DNSName("example.com")]
    );
}

#[test]
fn issued_pair_must_cover_every_domain_and_match_its_key() {
    let (chain, key, _) = issued(&["a.example.com", "b.example.com"]);
    let (_, other_key, _) = issued(&["a.example.com"]);
    let (_, validity) = certified(
        chain.as_bytes(),
        key.as_bytes(),
        &domains(&["a.example.com", "b.example.com"]),
    )
    .unwrap();
    assert!(validity.not_after > unix_now());
    assert!(validity.not_before <= unix_now());
    assert_eq!(
        certified(
            chain.as_bytes(),
            key.as_bytes(),
            &domains(&["c.example.com"])
        )
        .unwrap_err(),
        AcmeError::Certificate("leaf does not cover every domain")
    );
    assert_eq!(
        certified(
            chain.as_bytes(),
            other_key.as_bytes(),
            &domains(&["a.example.com"])
        )
        .unwrap_err(),
        AcmeError::Certificate("key does not match leaf")
    );
    assert_eq!(
        certified(b"", key.as_bytes(), &domains(&["a.example.com"])).unwrap_err(),
        AcmeError::Certificate("empty chain")
    );
}

#[test]
fn resolver_answers_only_declared_names_and_routes_challenges() {
    let digest = Sha256::digest(b"k");
    let resolver = Resolver {
        domains: domains(&["example.com"]),
        keys: RwLock::new(Keys::default()),
    };
    assert!(resolver.select(Some("example.com"), false).is_none());
    let (chain, key, _) = issued(&["example.com"]);
    let (served, _) = certified(chain.as_bytes(), key.as_bytes(), &resolver.domains).unwrap();
    let response = Arc::new(challenge_certificate("example.com", &digest).unwrap());
    {
        let mut keys = resolver.keys.write().unwrap();
        keys.current = Some(Arc::new(served));
        keys.challenges
            .insert("example.com".into(), response.clone());
    }
    assert!(resolver.select(Some("EXAMPLE.com"), false).is_some());
    assert!(Arc::ptr_eq(
        &resolver.select(Some("example.com"), true).unwrap(),
        &response
    ));
    assert!(resolver.select(Some("other.example.com"), false).is_none());
    assert!(resolver.select(Some("other.example.com"), true).is_none());
    assert!(resolver.select(None, false).is_none());
}

fn handshake(
    server: ServerConfig,
    roots: RootCertStore,
    name: &str,
    alpn: Vec<Vec<u8>>,
) -> Result<Option<Vec<u8>>, rustls::Error> {
    let mut client_config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client_config.alpn_protocols = alpn;
    let mut client =
        ClientConnection::new(Arc::new(client_config), name.to_owned().try_into().unwrap())
            .unwrap();
    let mut server = ServerConnection::new(Arc::new(server)).unwrap();
    for _ in 0..10 {
        let mut buffer = Vec::new();
        client.write_tls(&mut buffer).unwrap();
        server.read_tls(&mut buffer.as_slice()).unwrap();
        server.process_new_packets()?;
        let mut buffer = Vec::new();
        server.write_tls(&mut buffer).unwrap();
        client.read_tls(&mut buffer.as_slice()).unwrap();
        client.process_new_packets()?;
        if !client.is_handshaking() && !server.is_handshaking() {
            return Ok(client.alpn_protocol().map(<[u8]>::to_vec));
        }
    }
    Err(rustls::Error::General("handshake incomplete".into()))
}

#[test]
fn issuer_serves_cached_certificate_over_h2_and_refuses_other_names() {
    let cache = scratch("serve");
    let settings = AcmeSettings::lets_encrypt(cache.clone());
    let names = domains(&["example.com"]);
    let issuer = AcmeIssuer::new(settings.clone(), names.clone()).unwrap();
    assert!(issuer.due(unix_now()));
    let (chain, key, leaf) = issued(&["example.com"]);
    issuer.write_cache(CHAIN_FILE, chain.as_bytes()).unwrap();
    issuer.write_cache(KEY_FILE, key.as_bytes()).unwrap();
    let issuer = AcmeIssuer::new(settings, names.clone()).unwrap();
    assert!(!issuer.due(unix_now()));
    let config = super::super::certificate::issuer_config(&names, &issuer).unwrap();
    let mut roots = RootCertStore::empty();
    roots.add(leaf).unwrap();
    assert_eq!(
        handshake(
            config.clone(),
            roots.clone(),
            "example.com",
            vec![b"h2".to_vec()]
        )
        .unwrap(),
        Some(b"h2".to_vec())
    );
    assert!(handshake(config, roots, "other.example.com", vec![b"h2".to_vec()]).is_err());
    fs::remove_dir_all(cache).unwrap();
}

#[test]
fn cached_pair_for_other_domains_is_not_served() {
    let cache = scratch("stale");
    let settings = AcmeSettings::lets_encrypt(cache.clone());
    let issuer = AcmeIssuer::new(settings.clone(), domains(&["old.example.com"])).unwrap();
    let (chain, key, _) = issued(&["old.example.com"]);
    issuer.write_cache(CHAIN_FILE, chain.as_bytes()).unwrap();
    issuer.write_cache(KEY_FILE, key.as_bytes()).unwrap();
    let issuer = AcmeIssuer::new(settings, domains(&["new.example.com"])).unwrap();
    assert_eq!(issuer.not_after(), None);
    fs::remove_dir_all(cache).unwrap();
}

#[cfg(unix)]
#[test]
fn cache_is_owner_only_and_loose_entries_are_refused() {
    use std::os::unix::fs::PermissionsExt;
    let cache = scratch("perm");
    let settings = AcmeSettings::lets_encrypt(cache.clone());
    let issuer = AcmeIssuer::new(settings.clone(), domains(&["example.com"])).unwrap();
    assert_eq!(
        fs::metadata(&cache).unwrap().permissions().mode() & 0o777,
        0o700
    );
    issuer.write_cache(KEY_FILE, b"key").unwrap();
    let key_path = issuer.cache_path(KEY_FILE);
    assert_eq!(
        fs::metadata(&key_path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    issuer.write_cache(CHAIN_FILE, b"chain").unwrap();
    fs::set_permissions(&key_path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        AcmeIssuer::new(settings.clone(), domains(&["example.com"])).err(),
        Some(AcmeError::Cache("cache entry is not owner-only"))
    );
    fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(&cache, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        AcmeIssuer::new(settings, domains(&["example.com"])).err(),
        Some(AcmeError::Cache("cache entry is not owner-only"))
    );
    fs::set_permissions(&cache, fs::Permissions::from_mode(0o700)).unwrap();
    fs::remove_dir_all(cache).unwrap();
}

#[test]
fn account_files_are_keyed_by_directory() {
    assert_ne!(
        account_file(LETS_ENCRYPT_PRODUCTION),
        account_file(LETS_ENCRYPT_STAGING)
    );
    assert!(account_file(LETS_ENCRYPT_PRODUCTION).starts_with("account-"));
}

fn dated(name: &str, from: (i32, u8, u8), until: (i32, u8, u8)) -> (String, String) {
    let key = rcgen::KeyPair::generate().unwrap();
    let mut params = rcgen::CertificateParams::new(vec![name.to_owned()]).unwrap();
    params.not_before = rcgen::date_time_ymd(from.0, from.1, from.2);
    params.not_after = rcgen::date_time_ymd(until.0, until.1, until.2);
    let certificate = params.self_signed(&key).unwrap();
    (
        pem("CERTIFICATE", certificate.der()),
        pem("PRIVATE KEY", &key.serialize_der()),
    )
}

fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

/// Go `TestBuildEdgeTLSACME` and `TestBuildEdgeTLSACMEStagingDirectory`: the
/// issuer configuration (production or staging directory) carries a resolver,
/// advertises `acme-tls/1` and is accepted by the gateway; the same ALPN with
/// any other resolver, or in another shape, is refused.
#[test]
fn gateway_accepts_the_acme_issuer_configuration_and_only_it() {
    use crate::gateway::tls::{certificate::issuer_config, validate_configuration};
    let gateway = crate::gateway::tests::config();
    for (index, directory) in [LETS_ENCRYPT_PRODUCTION, LETS_ENCRYPT_STAGING]
        .into_iter()
        .enumerate()
    {
        let cache = scratch(&format!("edge-{index}"));
        let mut settings = AcmeSettings::lets_encrypt(cache.clone());
        settings.directory = directory.into();
        let names = domains(&["edge.example.com"]);
        let issuer = AcmeIssuer::new(settings, names.clone()).unwrap();
        let config = issuer_config(&names, &issuer).unwrap();
        assert_eq!(
            config.alpn_protocols,
            vec![
                b"h2".to_vec(),
                b"http/1.1".to_vec(),
                CHALLENGE_ALPN.to_vec()
            ]
        );
        validate_configuration(&gateway, &config).unwrap();

        let mut reordered = config.clone();
        reordered.alpn_protocols.rotate_right(1);
        assert!(validate_configuration(&gateway, &reordered).is_err());
        let mut challenge_only = config.clone();
        challenge_only.alpn_protocols = vec![CHALLENGE_ALPN.to_vec()];
        assert!(validate_configuration(&gateway, &challenge_only).is_err());
        let mut forged = ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(rustls::server::ResolvesServerCertUsingSni::new()));
        forged.alpn_protocols = config.alpn_protocols.clone();
        assert!(validate_configuration(&gateway, &forged).is_err());
        fs::remove_dir_all(cache).unwrap();
    }
}

/// Test-only verifier: a TLS-ALPN-01 response certificate is self-signed with
/// a critical ACME extension, so a validator inspects it instead of chaining.
#[derive(Debug)]
struct Validator;

impl rustls::client::danger::ServerCertVerifier for Validator {
    fn verify_server_cert(
        &self,
        _: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>,
        _: &[u8],
        _: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General(
            "validator offers TLS 1.3 only".into(),
        ))
    }
    // webpki refuses the critical ACME extension, so verify against the raw
    // SubjectPublicKeyInfo of the presented certificate.
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        let (_, parsed) = x509_parser::parse_x509_certificate(certificate)
            .map_err(|_| rustls::Error::General("certificate unreadable".into()))?;
        rustls::crypto::verify_tls13_signature_with_raw_key(
            message,
            &rustls::pki_types::SubjectPublicKeyInfoDer::from(parsed.public_key().raw),
            signature,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// A validator offering only `acme-tls/1` to the actual ingress receives the
/// challenge certificate, and the ingress then closes without reading a
/// request.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ingress_answers_the_challenge_handshake_and_reads_no_request() {
    let cache = scratch("ingress");
    let names = domains(&["edge.example.com"]);
    let issuer = AcmeIssuer::new(AcmeSettings::lets_encrypt(cache.clone()), names.clone()).unwrap();
    let digest = Sha256::digest(b"token.thumbprint");
    issuer.resolver.keys.write().unwrap().challenges.insert(
        "edge.example.com".into(),
        Arc::new(challenge_certificate("edge.example.com", &digest).unwrap()),
    );
    let config = Arc::new(super::super::certificate::issuer_config(&names, &issuer).unwrap());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let validator = tokio::task::spawn_blocking(move || {
        let mut client_config =
            ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(Validator))
                .with_no_client_auth();
        client_config.alpn_protocols = vec![CHALLENGE_ALPN.to_vec()];
        let mut client = ClientConnection::new(
            Arc::new(client_config),
            "edge.example.com".try_into().unwrap(),
        )
        .unwrap();
        let mut socket = std::net::TcpStream::connect(address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        while client.is_handshaking() {
            client.complete_io(&mut socket).unwrap();
        }
        (
            client.alpn_protocol().map(<[u8]>::to_vec),
            client.peer_certificates().unwrap()[0].clone().into_owned(),
        )
    });
    let (socket, _) = listener.accept().await.unwrap();
    let received = crate::gateway::tls::CapturedTls::receive(
        socket.into_std().unwrap(),
        config,
        Duration::from_secs(3),
    )
    .await;
    assert!(received.is_err());
    let (alpn, leaf) = validator.await.unwrap();
    assert_eq!(alpn.as_deref(), Some(CHALLENGE_ALPN));
    let (_, parsed) = x509_parser::parse_x509_certificate(&leaf).unwrap();
    let acme = parsed
        .extensions()
        .iter()
        .find(|extension| extension.oid.to_id_string() == "1.3.6.1.5.5.7.1.31")
        .expect("id-pe-acmeIdentifier");
    assert_eq!(&acme.value[2..], digest.as_slice());
    fs::remove_dir_all(cache).unwrap();
}

#[test]
fn certificate_cache_is_keyed_by_directory() {
    let cache = scratch("directory");
    let mut staging = AcmeSettings::lets_encrypt(cache.clone());
    staging.directory = LETS_ENCRYPT_STAGING.into();
    let production = AcmeSettings::lets_encrypt(cache.clone());
    let names = domains(&["example.com"]);
    let issuer = AcmeIssuer::new(staging.clone(), names.clone()).unwrap();
    let (chain, key, _) = issued(&["example.com"]);
    issuer.write_cache(CHAIN_FILE, chain.as_bytes()).unwrap();
    issuer.write_cache(KEY_FILE, key.as_bytes()).unwrap();
    let switched = AcmeIssuer::new(production, names.clone()).unwrap();
    assert_eq!(switched.not_after(), None);
    assert!(switched.due(unix_now()));
    let staged = AcmeIssuer::new(staging, names).unwrap();
    assert!(staged.not_after().is_some());
    // Not due: ensure starts no order.
    block_on(staged.ensure()).unwrap();
    assert_eq!(staged.renewal_facts(), RenewalFacts::default());
    fs::remove_dir_all(cache).unwrap();
}

#[test]
fn expired_or_not_yet_valid_pairs_are_refused_and_not_served() {
    let names = domains(&["example.com"]);
    let (expired_chain, expired_key) = dated("example.com", (2000, 1, 1), (2001, 1, 1));
    assert_eq!(
        certified(expired_chain.as_bytes(), expired_key.as_bytes(), &names).unwrap_err(),
        AcmeError::Certificate("leaf expired")
    );
    let (future_chain, future_key) = dated("example.com", (2999, 1, 1), (3000, 1, 1));
    assert_eq!(
        certified(future_chain.as_bytes(), future_key.as_bytes(), &names).unwrap_err(),
        AcmeError::Certificate("leaf not yet valid")
    );
    let cache = scratch("expired");
    let settings = AcmeSettings::lets_encrypt(cache.clone());
    let issuer = AcmeIssuer::new(settings.clone(), names.clone()).unwrap();
    issuer
        .write_cache(CHAIN_FILE, expired_chain.as_bytes())
        .unwrap();
    issuer
        .write_cache(KEY_FILE, expired_key.as_bytes())
        .unwrap();
    let reloaded = AcmeIssuer::new(settings, names).unwrap();
    assert_eq!(reloaded.not_after(), None);
    assert!(reloaded.due(unix_now()));
    fs::remove_dir_all(cache).unwrap();
}

#[test]
fn issuer_domains_are_normalised_and_wildcards_refused() {
    let cache = scratch("domains");
    let settings = AcmeSettings::lets_encrypt(cache.clone());
    let issuer = AcmeIssuer::new(
        settings.clone(),
        domains(&["Example.COM", "example.com", "b.example.com"]),
    )
    .unwrap();
    assert_eq!(
        issuer.domains(),
        domains(&["b.example.com", "example.com"]).as_slice()
    );
    for bad in [
        vec!["*.example.com"],
        vec![],
        vec!["127.0.0.1"],
        vec!["example.com."],
    ] {
        assert_eq!(
            AcmeIssuer::new(settings.clone(), domains(&bad)).err(),
            Some(AcmeError::Settings("domains must be plain DNS host names")),
            "{bad:?}"
        );
    }
    let many: Vec<String> = (0..=crate::gateway::tls::certificate::MAX_ISSUER_DOMAINS)
        .map(|index| format!("h{index}.example.com"))
        .collect();
    assert_eq!(
        AcmeIssuer::new(settings, many).err(),
        Some(AcmeError::Settings("too many domains"))
    );
    fs::remove_dir_all(cache).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn renewal_failures_are_recorded_with_kind_and_time() {
    let cache = scratch("facts");
    let mut settings = AcmeSettings::lets_encrypt(cache.clone());
    // Nothing listens on port 1: every order fails in the ACME exchange.
    settings.directory = "https://127.0.0.1:1/directory".into();
    let issuer = Arc::new(AcmeIssuer::new(settings, domains(&["example.com"])).unwrap());
    let before = unix_now();
    let error = issuer.issue().await.unwrap_err();
    assert_eq!(error.kind(), AcmeErrorKind::Protocol);
    let facts = issuer.renewal_facts();
    assert_eq!((facts.attempts, facts.generation), (1, 0));
    assert_eq!(facts.last_issued_at, None);
    let failure = facts.last_failure.expect("failure recorded");
    assert_eq!(failure.kind, AcmeErrorKind::Protocol);
    assert!(failure.at >= before && failure.at <= unix_now());
    assert_eq!(facts.consecutive_failures, 1);
    // The renewal loop records each retried failure instead of discarding it,
    // and backs off: with a 10 ms interval and no backoff it would try about
    // 50 times in 500 ms; doubling delays (40, 80, 160, 320 ms) allow at most
    // a handful.
    assert!(
        tokio::time::timeout(
            Duration::from_millis(500),
            issuer.clone().maintain(Duration::from_millis(10)),
        )
        .await
        .is_err()
    );
    let facts = issuer.renewal_facts();
    assert!((2..=6).contains(&facts.attempts), "{facts:?}");
    assert_eq!(u64::from(facts.consecutive_failures), facts.attempts);
    assert_eq!(facts.generation, 0);
    assert_eq!(
        facts.last_failure.map(|failure| failure.kind),
        Some(AcmeErrorKind::Protocol)
    );
    fs::remove_dir_all(cache).unwrap();
}

// Unix only: concurrent replacing renames are not exercised on Windows.
#[cfg(unix)]
#[test]
fn concurrent_cache_writes_never_share_a_temporary_file() {
    let cache = scratch("concurrent");
    private_directory(&cache).unwrap();
    let target = cache.join("entry.pem");
    let writers: Vec<_> = (0..8_u8)
        .map(|writer| {
            let (cache, target) = (cache.clone(), target.clone());
            std::thread::spawn(move || {
                for _ in 0..25 {
                    write_private(&cache, &target, &[writer; 64]).unwrap();
                }
            })
        })
        .collect();
    for writer in writers {
        writer.join().unwrap();
    }
    let written = fs::read(&target).unwrap();
    assert_eq!(written.len(), 64);
    assert!(written.iter().all(|byte| *byte == written[0]));
    let entries: Vec<_> = fs::read_dir(&cache)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries, vec![std::ffi::OsString::from("entry.pem")]);
    fs::remove_dir_all(cache).unwrap();
}

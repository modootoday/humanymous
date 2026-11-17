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
    let not_after = 10_000_000;
    assert!(renewal_due(None, 0));
    assert!(!renewal_due(
        Some(not_after),
        not_after - RENEW_BEFORE_SECONDS - 1
    ));
    assert!(renewal_due(
        Some(not_after),
        not_after - RENEW_BEFORE_SECONDS
    ));
    assert!(renewal_due(Some(not_after), not_after + 1));
    assert!(renewal_due(Some(i64::MIN), 0));
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
    let (_, not_after) = certified(
        chain.as_bytes(),
        key.as_bytes(),
        &domains(&["a.example.com", "b.example.com"]),
    )
    .unwrap();
    assert!(not_after > unix_now());
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
    let key_path = cache.join(KEY_FILE);
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

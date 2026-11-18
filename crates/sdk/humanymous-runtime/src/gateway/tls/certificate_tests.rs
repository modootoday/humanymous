use super::*;
use rustls::server::ClientHello;
use rustls::sign::CertifiedKey;

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

fn pair(name: &str) -> (String, String) {
    let issued = rcgen::generate_simple_self_signed(vec![name.to_owned()]).unwrap();
    (
        pem("CERTIFICATE", issued.cert.der()),
        pem("PRIVATE KEY", &issued.signing_key.serialize_der()),
    )
}

fn settings(chain: bool, key: bool, domains: &[&str]) -> CertificateSettings {
    CertificateSettings {
        chain: chain.then(|| PathBuf::from("chain.pem")),
        key: key.then(|| PathBuf::from("key.pem")),
        issuer_domains: domains.iter().map(|domain| (*domain).to_owned()).collect(),
    }
}

#[test]
fn selection_refuses_absent_incomplete_and_ambiguous_sources() {
    assert_eq!(
        select(&settings(false, false, &[])),
        Err(CertificateError::NoSource)
    );
    assert_eq!(
        select(&settings(true, false, &[])),
        Err(CertificateError::IncompletePair)
    );
    assert_eq!(
        select(&settings(false, true, &[])),
        Err(CertificateError::IncompletePair)
    );
    assert_eq!(
        select(&settings(true, true, &["example.com"])),
        Err(CertificateError::AmbiguousSource)
    );
    assert_eq!(
        select(&settings(true, true, &[])),
        Ok(CertificateSource::Provisioned {
            chain: "chain.pem".into(),
            key: "key.pem".into()
        })
    );
}

#[test]
fn issuer_domains_are_normalised_and_plain_hosts_only() {
    assert_eq!(
        select(&settings(
            false,
            false,
            &[" B.Example.com ", "a.example.com", "b.example.com"]
        )),
        Ok(CertificateSource::Issuer {
            domains: vec!["a.example.com".into(), "b.example.com".into()]
        })
    );
    for bad in [
        "*.example.com",
        "example.com.",
        "localhost",
        "192.0.2.1",
        "-bad.example.com",
        "bad-.example.com",
        "under_score.example.com",
        "",
    ] {
        assert_eq!(
            issuer_domains(&[bad.to_owned()]),
            Err(CertificateError::InvalidDomain),
            "{bad}"
        );
    }
    let many: Vec<String> = (0..=MAX_ISSUER_DOMAINS)
        .map(|n| format!("h{n}.example.com"))
        .collect();
    assert_eq!(issuer_domains(&many), Err(CertificateError::TooManyDomains));
}

#[test]
fn provisioned_pair_builds_h2_then_http11_ingress() {
    let (chain, key) = pair("example.com");
    let config = provisioned_config(chain.as_bytes(), key.as_bytes()).unwrap();
    assert_eq!(
        config.alpn_protocols,
        vec![b"h2".to_vec(), b"http/1.1".to_vec()]
    );
    assert_eq!(config.max_early_data_size, 0);
}

#[test]
fn provisioned_pair_refuses_empty_missing_and_mismatched_material() {
    let (chain, key) = pair("example.com");
    let (_, other_key) = pair("other.example.com");
    assert_eq!(
        provisioned_config(b"", key.as_bytes()).unwrap_err(),
        CertificateError::EmptyChain
    );
    assert_eq!(
        provisioned_config(chain.as_bytes(), b"").unwrap_err(),
        CertificateError::MissingKey
    );
    assert_eq!(
        provisioned_config(chain.as_bytes(), other_key.as_bytes()).unwrap_err(),
        CertificateError::Rejected
    );
}

#[test]
fn load_provisioned_reads_bounded_files() {
    let dir = std::env::temp_dir().join(format!("hmn-b02-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (chain, key) = pair("example.com");
    let (chain_path, key_path, big) = (
        dir.join("chain.pem"),
        dir.join("key.pem"),
        dir.join("big.pem"),
    );
    std::fs::write(&chain_path, chain).unwrap();
    std::fs::write(&key_path, key).unwrap();
    std::fs::write(&big, vec![b'a'; MAX_PEM_BYTES as usize + 1]).unwrap();
    assert!(load_provisioned(&chain_path, &key_path).is_ok());
    assert_eq!(
        load_provisioned(&big, &key_path).unwrap_err(),
        CertificateError::Unreadable
    );
    assert_eq!(
        load_provisioned(&dir.join("absent.pem"), &key_path).unwrap_err(),
        CertificateError::Unreadable
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[derive(Debug)]
struct NoCertificate;

impl ResolvesServerCert for NoCertificate {
    fn resolve(&self, _: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        None
    }
}

struct Issuer(Vec<String>);

impl CertificateIssuer for Issuer {
    fn domains(&self) -> &[String] {
        &self.0
    }
    fn resolver(&self) -> Arc<dyn ResolvesServerCert> {
        Arc::new(NoCertificate)
    }
    fn challenge_alpn(&self) -> Vec<Vec<u8>> {
        vec![b"acme-tls/1".to_vec()]
    }
}

#[test]
fn issuer_must_cover_exactly_the_selected_domains() {
    let domains = vec!["example.com".to_owned()];
    let config = issuer_config(&domains, &Issuer(domains.clone())).unwrap();
    assert_eq!(
        config.alpn_protocols,
        vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"acme-tls/1".to_vec()]
    );
    assert_eq!(
        issuer_config(&domains, &Issuer(vec!["other.example.com".into()])).unwrap_err(),
        CertificateError::AmbiguousSource
    );
}

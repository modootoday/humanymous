//! Per-run lab certificate authority for the TLS Gate topology.
//!
//! Minted inside the Gate container at start: a fresh CA key signs one server
//! certificate for the reserved `owned.test` name and is then dropped, so no
//! later certificate can be issued under this root. Only the leaf key reaches
//! disk, in an owner-only directory on the container tmpfs, and only until
//! the shipped gateway has loaded it. Nothing here is ever committed: every
//! run has a different root, and only the public root certificate (PEM and
//! SHA-256 of its DER) leaves the container.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose,
};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, os::unix::fs::OpenOptionsExt, path::Path};
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// The one server name the lab CA certifies (RFC 6761 reserved `.test`).
pub const SERVER_NAME: &str = "owned.test";

/// PEM armour for one DER object, 64 columns, LF line endings.
pub fn pem(label: &str, der: &[u8]) -> String {
    let body = STANDARD.encode(der);
    let mut text = format!("-----BEGIN {label}-----\n");
    for line in body.as_bytes().chunks(64) {
        text.push_str(std::str::from_utf8(line).expect("base64 is ASCII"));
        text.push('\n');
    }
    text.push_str(&format!("-----END {label}-----\n"));
    text
}

/// The issued server credential; the CA signing key no longer exists.
pub struct Issued {
    /// Public root certificate, PEM.
    pub root_pem: String,
    /// SHA-256 of the root certificate DER, lowercase hex.
    pub root_sha256: String,
    /// Leaf then root, PEM.
    pub chain_pem: String,
    /// Leaf PKCS#8 private key, PEM.
    pub key_pem: Zeroizing<String>,
}

/// Mint a fresh root and one `owned.test` server certificate under it.
pub fn issue() -> Result<Issued> {
    let mut root = CertificateParams::new(Vec::<String>::new())?;
    root.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    root.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    root.distinguished_name
        .push(DnType::CommonName, "humanymous product-e2e per-run lab CA");
    let root = CertifiedIssuer::self_signed(root, KeyPair::generate()?)?;
    let leaf_key = KeyPair::generate()?;
    let mut leaf = CertificateParams::new(vec![SERVER_NAME.to_owned()])?;
    leaf.is_ca = IsCa::ExplicitNoCa;
    leaf.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    leaf.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    leaf.use_authority_key_identifier_extension = true;
    leaf.distinguished_name
        .push(DnType::CommonName, SERVER_NAME);
    let leaf = leaf.signed_by(&leaf_key, &*root)?;
    let root_der = root.der().to_vec();
    let root_pem = pem("CERTIFICATE", &root_der);
    // The root's issuer (and its signing key) is dropped here.
    drop(root);
    Ok(Issued {
        root_sha256: hex::encode(Sha256::digest(&root_der)),
        chain_pem: pem("CERTIFICATE", leaf.der()) + &root_pem,
        root_pem,
        key_pem: Zeroizing::new(pem(
            "PRIVATE KEY",
            &Zeroizing::new(leaf_key.serialize_der()),
        )),
    })
}

/// Write one file that only its owner can read; refuses an existing path.
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::{
        RootCertStore,
        client::{WebPkiServerVerifier, danger::ServerCertVerifier},
        pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime, pem::PemObject},
    };
    use std::sync::Arc;

    fn chain(issued: &Issued) -> Vec<CertificateDer<'static>> {
        CertificateDer::pem_slice_iter(issued.chain_pem.as_bytes())
            .collect::<std::result::Result<_, _>>()
            .unwrap()
    }

    fn verifies(root_pem: &str, chain: &[CertificateDer<'static>], name: &str) -> bool {
        let mut roots = RootCertStore::empty();
        roots
            .add(CertificateDer::from_pem_slice(root_pem.as_bytes()).unwrap())
            .unwrap();
        let verifier = WebPkiServerVerifier::builder_with_provider(
            Arc::new(roots),
            Arc::new(rustls::crypto::ring::default_provider()),
        )
        .build()
        .unwrap();
        verifier
            .verify_server_cert(
                &chain[0],
                &chain[1..],
                &ServerName::try_from(name.to_owned()).unwrap(),
                &[],
                UnixTime::now(),
            )
            .is_ok()
    }

    #[test]
    fn each_run_mints_a_distinct_root_that_verifies_only_its_own_leaf() {
        let first = issue().unwrap();
        let second = issue().unwrap();
        assert_ne!(first.root_sha256, second.root_sha256);
        let leaf = chain(&first);
        assert_eq!(leaf.len(), 2);
        assert!(verifies(&first.root_pem, &leaf, SERVER_NAME));
        assert!(!verifies(&second.root_pem, &leaf, SERVER_NAME));
        assert!(!verifies(&first.root_pem, &leaf, "foreign.test"));
        assert!(PrivateKeyDer::from_pem_slice(first.key_pem.as_bytes()).is_ok());
        assert_eq!(
            hex::encode(Sha256::digest(
                CertificateDer::from_pem_slice(first.root_pem.as_bytes())
                    .unwrap()
                    .as_ref()
            )),
            first.root_sha256
        );
    }

    #[test]
    fn pem_lines_are_bounded_and_lf_terminated() {
        let text = pem("CERTIFICATE", &[7; 100]);
        assert!(text.starts_with("-----BEGIN CERTIFICATE-----\n"));
        assert!(text.ends_with("-----END CERTIFICATE-----\n"));
        assert!(
            text.lines()
                .all(|line| line.len() <= 64 || line.starts_with("-----"))
        );
        assert!(!text.contains('\r'));
    }

    #[test]
    fn private_files_are_owner_only_and_never_overwritten() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("lab-ca-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("key.pem");
        let _ = fs::remove_file(&path);
        write_private(&path, b"secret").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(write_private(&path, b"again").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}

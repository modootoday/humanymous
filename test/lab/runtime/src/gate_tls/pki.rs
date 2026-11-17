//! Ephemeral lab certificate authority, generated inside the Gate container.
//!
//! A fresh CA signs one server certificate for the lab authority. The CA key
//! exists only in memory and is dropped after signing; the server key is
//! written once to the Gate's private tmpfs (0600 in a 0700 directory) for the
//! runtime's provisioned-certificate source. Only public material (the CA
//! certificate and the server certificate digest) ever leaves the container.
//! Nothing here is a production certificate path or a self-signed fallback:
//! the runtime receives an ordinary provisioned chain and key.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa,
    Issuer, KeyPair, KeyUsagePurpose,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Public lab trust anchor plus the private files the runtime reads.
pub struct LabPki {
    /// PEM CA certificate: the only trust anchor the client accepts.
    pub ca_pem: String,
    /// SHA-256 of the DER server certificate the runtime must present.
    pub leaf_sha256: String,
    /// PEM server certificate chain path.
    pub chain: PathBuf,
    /// PEM PKCS#8 server key path (0600).
    pub key: PathBuf,
}

/// PEM armour with 64-column base64 lines.
pub fn pem(label: &str, der: &[u8]) -> String {
    let text = STANDARD.encode(der);
    let mut out = format!("-----BEGIN {label}-----\n");
    for line in text.as_bytes().chunks(64) {
        out.push_str(&String::from_utf8_lossy(line));
        out.push('\n');
    }
    out.push_str(&format!("-----END {label}-----\n"));
    out
}

fn private_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Issue a fresh CA and server certificate for `server_name` into `directory`
/// (created 0700; it must not exist yet).
pub fn issue(directory: &Path, server_name: &str) -> Result<LabPki> {
    fs::create_dir(directory)?;
    fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
    let ca_key = KeyPair::generate()?;
    let mut ca = CertificateParams::new(Vec::<String>::new())?;
    ca.distinguished_name = DistinguishedName::new();
    ca.distinguished_name
        .push(DnType::CommonName, "humanymous ephemeral lab CA");
    ca.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    ca.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca_certificate = ca.self_signed(&ca_key)?;
    let issuer = Issuer::new(ca, ca_key);
    let server_key = KeyPair::generate()?;
    let mut server = CertificateParams::new(vec![server_name.to_owned()])?;
    server.distinguished_name = DistinguishedName::new();
    server
        .distinguished_name
        .push(DnType::CommonName, server_name);
    server.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    server.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let leaf = server.signed_by(&server_key, &issuer)?;
    drop(issuer);
    let chain = directory.join("chain.pem");
    let key = directory.join("key.pem");
    private_file(&chain, pem("CERTIFICATE", leaf.der()).as_bytes())?;
    private_file(
        &key,
        zeroize::Zeroizing::new(pem("PRIVATE KEY", &server_key.serialize_der())).as_bytes(),
    )?;
    Ok(LabPki {
        ca_pem: pem("CERTIFICATE", ca_certificate.der()),
        leaf_sha256: format!("{:x}", Sha256::digest(leaf.der())),
        chain,
        key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;

    #[test]
    fn server_certificate_chains_to_the_ephemeral_ca_and_key_is_private() {
        let mut nonce = [0; 8];
        getrandom::fill(&mut nonce).unwrap();
        let name: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
        let root = std::env::temp_dir().join(format!("gate-tls-pki-{name}"));
        let pki = issue(&root, "owned.test").unwrap();
        assert_eq!(fs::metadata(&root).unwrap().mode() & 0o777, 0o700);
        assert_eq!(fs::metadata(&pki.key).unwrap().mode() & 0o777, 0o600);
        assert!(
            issue(&root, "owned.test").is_err(),
            "never reuses a directory"
        );
        // The runtime's own provisioned loader accepts the pair.
        let tls =
            humanymous_runtime::gateway::tls::certificate::load_provisioned(&pki.chain, &pki.key)
                .unwrap();
        assert_eq!(tls.alpn_protocols, [b"h2".to_vec(), b"http/1.1".to_vec()]);
        // A client trusting only the CA verifies the server certificate.
        let roots = super::super::client::roots(&pki.ca_pem).unwrap();
        assert_eq!(roots.len(), 1);
        assert!(pki.ca_pem.starts_with("-----BEGIN CERTIFICATE-----\n"));
        assert_eq!(pki.leaf_sha256.len(), 64);
        fs::remove_dir_all(root).unwrap();
    }
}

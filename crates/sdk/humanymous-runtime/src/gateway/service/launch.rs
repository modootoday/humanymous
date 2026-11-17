//! Private inherited launch document for the shipped runtime binary.
//!
//! The private-loopback v1 [`Configuration`] document is read exactly as
//! before. A separate closed, versioned envelope adds TLS termination at the
//! runtime's own loopback ingress with an operator-provisioned certificate
//! (B02 provisioned source). The envelope never relaxes the loopback listener
//! or origin, never selects an issuer and never falls back to a self-signed
//! certificate: an absent, incomplete or unreadable pair refuses startup.
use crate::gateway::{
    configuration::Configuration,
    tls::certificate::{self, CertificateSettings, CertificateSource},
};
use serde::Deserialize;
use std::{
    io::{self, Read},
    path::PathBuf,
    sync::Arc,
};
use zeroize::Zeroizing;

/// Schema of the closed loopback TLS ingress envelope.
pub const TLS_INGRESS_SCHEMA: &str = "humanymous.runtime-loopback-tls-ingress/1";

/// What the shipped binary was asked to serve.
pub enum Launch {
    /// Unchanged private-loopback v1 document over plain HTTP/1.1.
    Plain(Configuration),
    /// The same v1 document behind TLS terminated by the runtime itself.
    Tls {
        /// Validated v1 configuration (still loopback listen/origin only).
        configuration: Configuration,
        /// Server configuration from the provisioned pair, ALPN h2 then http/1.1.
        tls: Arc<rustls::ServerConfig>,
    },
}

/// Envelope probe: only the discriminating field is retained.
#[derive(Deserialize)]
struct Probe {
    #[serde(default)]
    schema: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema: String,
    configuration: Configuration,
    certificate: Provisioned,
}

/// Operator-provisioned PEM chain and private key paths.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Provisioned {
    chain: PathBuf,
    key: PathBuf,
}

/// Read one bounded private launch document.
pub fn read_launch(input: impl Read) -> io::Result<Launch> {
    let mut bytes = Zeroizing::new(Vec::new());
    input.take(65_537).read_to_end(&mut bytes)?;
    if bytes.len() > 65_536 {
        return Err(io::Error::other("configuration budget"));
    }
    let probe: Probe =
        serde_json::from_slice(&bytes).map_err(|_| io::Error::other("configuration JSON"))?;
    if probe.schema.is_none() {
        // A v1 document refuses unknown fields, so it never carries `schema`.
        return super::read_configuration(bytes.as_slice()).map(Launch::Plain);
    }
    if probe.schema.as_deref() != Some(TLS_INGRESS_SCHEMA) {
        return Err(io::Error::other("launch schema unsupported"));
    }
    let envelope: Envelope =
        serde_json::from_slice(&bytes).map_err(|_| io::Error::other("launch envelope JSON"))?;
    if envelope.schema != TLS_INGRESS_SCHEMA {
        return Err(io::Error::other("launch schema unsupported"));
    }
    envelope
        .configuration
        .validate()
        .map_err(|_| io::Error::other("configuration invalid"))?;
    let settings = CertificateSettings {
        chain: Some(envelope.certificate.chain),
        key: Some(envelope.certificate.key),
        ..Default::default()
    };
    let tls = match certificate::select(&settings)
        .map_err(|error| io::Error::other(error.to_string()))?
    {
        CertificateSource::Provisioned { chain, key } => {
            certificate::load_provisioned(&chain, &key)
                .map_err(|error| io::Error::other(error.to_string()))?
        }
        // The envelope declares a pair only; any other source is a refusal.
        _ => {
            return Err(io::Error::other(
                "only a provisioned certificate is accepted",
            ));
        }
    };
    Ok(Launch::Tls {
        configuration: envelope.configuration,
        tls: Arc::new(tls),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::path::Path;

    fn v1() -> Value {
        json!({"listen":"127.0.0.1:0","origin":"127.0.0.1:9",
            "scope":{"installation":"i","tenant":"t","origin":"o","revision":"r1","authority":"owned.test"},
            "routes":[{"method":"POST","path":"/effect","required":{"raw_tls":true,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}],
            "audit":{"directory":"a","node_id":"n","keys":{"directory":"k","passphrase":"p"},"allow_new":true,"trusted_head":null},
            "owner":{"kind":"unix","uid":1},"session_capacity":4,"session_ttl_ms":1000,"timeout_ms":1000,"rate_per_second":4})
    }

    fn pair(directory: &Path) -> (PathBuf, PathBuf) {
        let certified = rcgen::generate_simple_self_signed(vec!["owned.test".into()]).unwrap();
        let pem = |label: &str, der: &[u8]| {
            use base64::Engine as _;
            let text = base64::engine::general_purpose::STANDARD.encode(der);
            let mut out = format!("-----BEGIN {label}-----\n");
            for line in text.as_bytes().chunks(64) {
                out.push_str(std::str::from_utf8(line).unwrap());
                out.push('\n');
            }
            out.push_str(&format!("-----END {label}-----\n"));
            out
        };
        let chain = directory.join("chain.pem");
        let key = directory.join("key.pem");
        std::fs::write(&chain, pem("CERTIFICATE", certified.cert.der())).unwrap();
        std::fs::write(
            &key,
            pem("PRIVATE KEY", &certified.signing_key.serialize_der()),
        )
        .unwrap();
        (chain, key)
    }

    fn scratch(name: &str) -> PathBuf {
        let mut nonce = [0; 8];
        getrandom::fill(&mut nonce).unwrap();
        let directory = std::env::temp_dir().join(format!("launch-{name}-{}", hex::encode(nonce)));
        std::fs::create_dir(&directory).unwrap();
        directory
    }

    fn read(value: &Value) -> io::Result<Launch> {
        read_launch(serde_json::to_vec(value).unwrap().as_slice())
    }

    #[test]
    fn v1_document_is_read_unchanged_as_plain() {
        assert!(matches!(read(&v1()), Ok(Launch::Plain(_))));
        let mut unknown = v1();
        unknown["tls"] = json!(true);
        assert!(read(&unknown).is_err());
    }

    #[test]
    fn envelope_with_a_provisioned_pair_offers_h2_then_http1() {
        let directory = scratch("ok");
        let (chain, key) = pair(&directory);
        let launch = read(&json!({"schema":TLS_INGRESS_SCHEMA,"configuration":v1(),
            "certificate":{"chain":chain,"key":key}}))
        .unwrap();
        let Launch::Tls { configuration, tls } = launch else {
            panic!("TLS launch expected");
        };
        assert!(configuration.listen.ip().is_loopback());
        assert_eq!(tls.alpn_protocols, [b"h2".to_vec(), b"http/1.1".to_vec()]);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn envelope_refuses_unknown_schema_fields_missing_or_mismatched_pairs() {
        let directory = scratch("bad");
        let (chain, key) = pair(&directory);
        let other = scratch("other");
        let (other_chain, _) = pair(&other);
        let envelope = |certificate: Value| json!({"schema":TLS_INGRESS_SCHEMA,"configuration":v1(),"certificate":certificate});
        let mut wrong_schema = envelope(json!({"chain":chain,"key":key}));
        wrong_schema["schema"] = json!("humanymous.runtime-loopback-tls-ingress/2");
        let mut extra = envelope(json!({"chain":chain,"key":key}));
        extra["self_signed"] = json!(true);
        let mut public = envelope(json!({"chain":chain,"key":key}));
        public["configuration"]["listen"] = json!("0.0.0.0:8443");
        for refused in [
            wrong_schema,
            extra,
            public,
            envelope(json!({"chain":chain})),
            envelope(json!({"chain":chain,"key":directory.join("absent.pem")})),
            envelope(json!({"chain":key,"key":key})),
            envelope(json!({"chain":other_chain,"key":key})),
            envelope(json!({"chain":chain,"key":key,"issuer":"acme"})),
            json!({"schema":TLS_INGRESS_SCHEMA,"configuration":v1()}),
        ] {
            assert!(read(&refused).is_err(), "{refused}");
        }
        std::fs::remove_dir_all(directory).unwrap();
        std::fs::remove_dir_all(other).unwrap();
    }
}

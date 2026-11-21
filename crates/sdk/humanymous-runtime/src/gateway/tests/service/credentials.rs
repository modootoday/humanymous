//! Shipping row B25 in the shipped composition: the configured credential
//! trust-anchor files (Go `-agent-keys`, `-pat-issuers`) are read and their
//! gates installed before readiness; a refused file stops startup.
use super::*;
use crate::gateway::configuration::credentials::Credentials;
use crate::gateway::credential::privacy_pass::tests::ISSUER_A_SPKI;
use base64::Engine as _;

fn pem(spki_b64: &str) -> String {
    let lines: Vec<&str> = spki_b64
        .as_bytes()
        .chunks(64)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect();
    format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
        lines.join("\n")
    )
}

fn anchors(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let key = ed25519_dalek::SigningKey::from_bytes(&[3; 32]);
    let public =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes());
    let agent = dir.join("agent-keys.txt");
    std::fs::write(&agent, format!("# trusted agents\nagent-1 {public}\n")).unwrap();
    let pat = dir.join("pat-issuers.pem");
    std::fs::write(&pat, pem(ISSUER_A_SPKI)).unwrap();
    (agent, pat)
}

#[tokio::test]
async fn unix_configured_credential_anchors_are_installed_before_readiness() {
    let fixture = PrivateFixture::new();
    let (agent, pat) = anchors(&fixture.0);
    let mut config = fixture.configuration();
    config.credentials = Some(Credentials {
        agent_keys: Some(agent.clone()),
        pat_issuers: Some(pat),
        webauthn: None,
    });
    let host = ServiceHost::bind(config).await.unwrap();
    let ready = serde_json::to_value(host.readiness()).unwrap();
    let facts = &ready["profile"]["credentials"];
    assert_eq!(facts["web_bot_auth"], true, "{facts}");
    assert_eq!(facts["private_access_token"], true, "{facts}");
    assert_eq!(facts["webauthn"], false, "{facts}");
    assert!(
        !ready.to_string().contains("agent-keys.txt"),
        "path in readiness"
    );
    drop(host);

    // Only the agent allowlist.
    let fixture = PrivateFixture::new();
    let (agent, _) = anchors(&fixture.0);
    let mut config = fixture.configuration();
    config.credentials = Some(Credentials {
        agent_keys: Some(agent),
        pat_issuers: None,
        webauthn: None,
    });
    let host = ServiceHost::bind(config).await.unwrap();
    let ready = serde_json::to_value(host.readiness()).unwrap();
    assert_eq!(
        ready["profile"]["credentials"]["private_access_token"],
        false
    );
    drop(host);

    // Without the object the profile has no credentials facts.
    let fixture = PrivateFixture::new();
    let host = ServiceHost::bind(fixture.configuration()).await.unwrap();
    let ready = serde_json::to_value(host.readiness()).unwrap();
    assert!(ready["profile"].get("credentials").is_none());
}

#[tokio::test]
async fn unix_refused_credential_anchors_stop_startup() {
    let cases: [(&str, &str); 4] = [
        ("agent", "agent-1 not-a-key\n"),
        ("agent", "# only comments\n"),
        (
            "pat",
            "-----BEGIN PUBLIC KEY-----\nAAAA\n-----END PUBLIC KEY-----\n",
        ),
        ("pat", ""),
    ];
    for (kind, text) in cases {
        let fixture = PrivateFixture::new();
        let path = fixture.0.join("anchor");
        std::fs::write(&path, text).unwrap();
        let mut config = fixture.configuration();
        config.credentials = Some(match kind {
            "agent" => Credentials {
                agent_keys: Some(path),
                pat_issuers: None,
                webauthn: None,
            },
            _ => Credentials {
                agent_keys: None,
                pat_issuers: Some(path),
                webauthn: None,
            },
        });
        assert!(ServiceHost::bind(config).await.is_err(), "{kind}: {text:?}");
        assert!(Custody::acquire(&fixture.0.join("keys"), Owner::CurrentUser).is_ok());
    }
    // A missing file.
    let fixture = PrivateFixture::new();
    let mut config = fixture.configuration();
    config.credentials = Some(Credentials {
        agent_keys: Some(fixture.0.join("absent")),
        pat_issuers: None,
        webauthn: None,
    });
    assert!(ServiceHost::bind(config).await.is_err());
}

#[test]
fn credentials_object_names_at_least_one_nonempty_file() {
    let mut config = config();
    config.credentials = Some(Credentials::default());
    assert!(config.validate().is_err());
    config.credentials = Some(Credentials {
        agent_keys: Some("".into()),
        pat_issuers: None,
        webauthn: None,
    });
    assert!(config.validate().is_err());
    config.credentials = Some(Credentials {
        agent_keys: Some("/etc/hmn/agents".into()),
        pat_issuers: None,
        webauthn: None,
    });
    assert!(config.validate().is_ok());
    assert!(
        serde_json::from_value::<Credentials>(serde_json::json!({"webauthn_creds": "/x"})).is_err(),
        "unknown keys are refused"
    );
}

fn webauthn_config(
    dir: &std::path::Path,
    fanout: serde_json::Value,
) -> crate::gateway::configuration::credentials::WebAuthn {
    use rcgen::PublicKeyData as _;
    let key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).unwrap();
    let spki =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.subject_public_key_info());
    let path = dir.join("webauthn-creds.txt");
    std::fs::write(&path, format!("credential-1 {spki}\n")).unwrap();
    serde_json::from_value(serde_json::json!({
        "credentials": path,
        "origin": "https://owned.test",
        "rp_id": "owned.test",
        "fanout": fanout,
    }))
    .unwrap()
}

fn bounds() -> serde_json::Value {
    serde_json::json!({"max_credentials": 1024, "max_distinct": 64, "max_field_bytes": 512, "max_payload_bytes": 1_048_576})
}

/// WebAuthn needs an attributable client (its fan-out subnet): a loopback
/// listener without the PROXY v2 edge refuses startup; a public listener
/// installs it and readiness says so.
#[tokio::test]
async fn unix_webauthn_is_installed_only_with_a_client_address() {
    let fixture = PrivateFixture::new();
    let mut config = fixture.configuration();
    config.credentials = Some(Credentials {
        agent_keys: None,
        pat_issuers: None,
        webauthn: Some(webauthn_config(&fixture.0, bounds())),
    });
    config.validate().unwrap();
    assert!(ServiceHost::bind(config).await.is_err());
    assert!(Custody::acquire(&fixture.0.join("keys"), Owner::CurrentUser).is_ok());

    let fixture = PrivateFixture::new();
    let (tls, _) = provisioned_pair(&fixture.0.join("tls"), "owned.test");
    let mut config = fixture.configuration();
    config.listen = "0.0.0.0:0".parse().unwrap();
    config.public_listen = true;
    config.tls = Some(tls);
    config.credentials = Some(Credentials {
        agent_keys: None,
        pat_issuers: None,
        webauthn: Some(webauthn_config(&fixture.0, bounds())),
    });
    let prepared = crate::gateway::tls::listener::prepare(config.tls.as_ref().unwrap()).unwrap();
    let host = ServiceHost::bind_tls(config, None, None, None, None, prepared.server_config())
        .await
        .unwrap();
    let ready = serde_json::to_value(host.readiness()).unwrap();
    let facts = &ready["profile"]["credentials"];
    assert_eq!(facts["webauthn"], true, "{facts}");
    assert_eq!(facts["web_bot_auth"], false, "{facts}");
    assert!(
        !ready.to_string().contains("webauthn-creds.txt"),
        "path in readiness"
    );
}

#[tokio::test]
async fn unix_refused_webauthn_configuration() {
    // A zero bound, an empty credentials path and an invalid key file.
    let fixture = PrivateFixture::new();
    let mut zero = bounds();
    zero["max_distinct"] = serde_json::json!(0);
    let mut config = fixture.configuration();
    config.credentials = Some(Credentials {
        agent_keys: None,
        pat_issuers: None,
        webauthn: Some(webauthn_config(&fixture.0, zero)),
    });
    assert!(config.validate().is_err(), "zero fan-out bound");
    let mut webauthn = webauthn_config(&fixture.0, bounds());
    webauthn.credentials = "".into();
    config.credentials = Some(Credentials {
        agent_keys: None,
        pat_issuers: None,
        webauthn: Some(webauthn),
    });
    assert!(config.validate().is_err(), "empty path");

    let (tls, _) = provisioned_pair(&fixture.0.join("tls"), "owned.test");
    let mut config = fixture.configuration();
    config.listen = "0.0.0.0:0".parse().unwrap();
    config.public_listen = true;
    config.tls = Some(tls);
    let webauthn = webauthn_config(&fixture.0, bounds());
    std::fs::write(&webauthn.credentials, "credential-1 not-a-key\n").unwrap();
    config.credentials = Some(Credentials {
        agent_keys: None,
        pat_issuers: None,
        webauthn: Some(webauthn),
    });
    let prepared = crate::gateway::tls::listener::prepare(config.tls.as_ref().unwrap()).unwrap();
    assert!(
        ServiceHost::bind_tls(config, None, None, None, None, prepared.server_config())
            .await
            .is_err()
    );
    assert!(
        serde_json::from_value::<crate::gateway::configuration::credentials::FanoutBounds>(
            serde_json::json!({"max_credentials": 1, "max_distinct": 1, "max_field_bytes": 1})
        )
        .is_err(),
        "every bound is explicit"
    );
}

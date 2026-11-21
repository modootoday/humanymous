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
            },
            _ => Credentials {
                agent_keys: None,
                pat_issuers: Some(path),
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
    });
    assert!(config.validate().is_err());
    config.credentials = Some(Credentials {
        agent_keys: Some("/etc/hmn/agents".into()),
        pat_issuers: None,
    });
    assert!(config.validate().is_ok());
    assert!(
        serde_json::from_value::<Credentials>(serde_json::json!({"webauthn_creds": "/x"})).is_err(),
        "unknown keys are refused"
    );
}

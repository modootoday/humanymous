//! Load the configured credential trust anchors (`configuration::credentials`)
//! and install their gates. Every failure is a startup refusal before
//! readiness, with a static message (no paths or file bytes).
use crate::gateway::application::Application;
use crate::gateway::attest_floor::fanout::CredentialFanout;
use crate::gateway::configuration::credentials::Credentials;
use crate::gateway::credential::{MAX_CONFIG, privacy_pass, web_bot_auth, webauthn};
use crate::network::correlation::Registry;
use std::io::{self, Read};
use std::path::Path;

/// Read one trust-anchor file, bounded by the credential configuration cap.
fn read(path: &Path, what: &'static str) -> io::Result<String> {
    let file = std::fs::File::open(path).map_err(|_| io::Error::other(what))?;
    let mut text = String::new();
    file.take(MAX_CONFIG as u64 + 1)
        .read_to_string(&mut text)
        .map_err(|_| io::Error::other(what))?;
    if text.len() > MAX_CONFIG {
        return Err(io::Error::other(what));
    }
    Ok(text)
}

/// Parsed trust anchors, loaded before readiness and installed once the
/// application exists.
pub(super) struct Loaded {
    agent: Option<web_bot_auth::KeyDirectory>,
    pat: Option<privacy_pass::Verifier>,
    webauthn: Option<(webauthn::Registry, CredentialFanout)>,
}

/// Read and parse every configured file.
pub(super) fn load(credentials: &Credentials) -> io::Result<Loaded> {
    let agent = match &credentials.agent_keys {
        Some(path) => Some(
            web_bot_auth::KeyDirectory::parse(&read(path, "agent keys unreadable")?)
                .map_err(|_| io::Error::other("agent keys refused"))?,
        ),
        None => None,
    };
    let pat = match &credentials.pat_issuers {
        Some(path) => Some(
            privacy_pass::Verifier::from_pem(&read(path, "PAT issuers unreadable")?)
                .map_err(|_| io::Error::other("PAT issuers refused"))?,
        ),
        None => None,
    };
    let webauthn = match &credentials.webauthn {
        Some(config) => {
            let mut registry = webauthn::Registry::parse(&read(
                &config.credentials,
                "WebAuthn credentials unreadable",
            )?)
            .map_err(|_| io::Error::other("WebAuthn credentials refused"))?;
            registry.set_binding(&config.origin, &config.rp_id);
            let limits = config
                .fanout
                .limits()
                .ok_or_else(|| io::Error::other("WebAuthn fan-out bounds refused"))?;
            let fanout = CredentialFanout::new(Registry::new(limits), i64::from(config.fanout_cap));
            Some((registry, fanout))
        }
        None => None,
    };
    Ok(Loaded {
        agent,
        pat,
        webauthn,
    })
}

impl Loaded {
    /// Readiness facts: which gates are installed, never paths or keys.
    pub(super) fn facts(&self) -> serde_json::Value {
        serde_json::json!({
            "web_bot_auth": self.agent.is_some(),
            "private_access_token": self.pat.is_some(),
            "webauthn": self.webauthn.is_some(),
        })
    }

    /// Install the loaded gates. WebAuthn is refused unless served
    /// connections carry a client address, which its fan-out subnet needs.
    pub(super) fn install(self, application: &mut Application) -> io::Result<()> {
        if let Some(directory) = self.agent {
            application.install_agent_keys(directory);
        }
        if let Some(verifier) = self.pat {
            application.install_pat_issuers(verifier);
        }
        if let Some((registry, fanout)) = self.webauthn {
            application
                .install_webauthn(registry, fanout)
                .map_err(|_| io::Error::other("WebAuthn needs a client address source"))?;
        }
        Ok(())
    }
}

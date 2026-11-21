//! Load the configured credential trust anchors (`configuration::credentials`)
//! and install their gates. Every failure is a startup refusal before
//! readiness, with a static message (no paths or file bytes).
use crate::gateway::application::Application;
use crate::gateway::configuration::credentials::Credentials;
use crate::gateway::credential::{MAX_CONFIG, privacy_pass, web_bot_auth};
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
    Ok(Loaded { agent, pat })
}

impl Loaded {
    /// Readiness facts: which gates are installed, never paths or keys.
    pub(super) fn facts(&self) -> serde_json::Value {
        serde_json::json!({
            "web_bot_auth": self.agent.is_some(),
            "private_access_token": self.pat.is_some(),
            "webauthn": false,
        })
    }

    /// Install the loaded gates.
    pub(super) fn install(self, application: &mut Application) {
        if let Some(directory) = self.agent {
            application.install_agent_keys(directory);
        }
        if let Some(verifier) = self.pat {
            application.install_pat_issuers(verifier);
        }
    }
}

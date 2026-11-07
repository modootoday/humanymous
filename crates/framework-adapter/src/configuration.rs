use crate::Error;
use serde::Deserialize;
use zeroize::Zeroizing;

/// Private supervisor settings. No Debug/Serialize implementation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Configuration {
    /// Installation scope, never a public header.
    pub installation: String,
    /// Tenant binding.
    pub tenant: String,
    /// Protected origin binding.
    pub origin: String,
    /// Active policy revision.
    pub revision: String,
    /// Exact local authenticated transport endpoint.
    pub endpoint: String,
    credential: Zeroizing<String>,
    /// Expected actual authority OS identity/process.
    pub server: humanymous_authority::configuration::Peer,
    /// Complete exact protected pathname inventory.
    pub routes: Vec<String>,
    /// Complete input/decision/handoff budget, milliseconds.
    pub timeout_ms: u64,
    /// At most1MiB of transfer-decoded entity bytes.
    pub max_body: usize,
    /// Sync host input deadline must be separately enforced by the server.
    pub sync_input_deadline_enforced: bool,
}
impl Configuration {
    /// Parse required fixed schema; duplicate/unknown members reject.
    pub fn parse(json: &str) -> Result<Self, Error> {
        let c: Self =
            serde_json::from_str(json).map_err(|_| Error("invalid adapter configuration"))?;
        let identity = |s: &str| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        };
        if ![&c.installation, &c.tenant, &c.origin, &c.revision]
            .into_iter()
            .all(|s| identity(s))
            || !(1..=30_000).contains(&c.timeout_ms)
            || c.max_body > 1_048_576
            || !(32..=256).contains(&c.credential.len())
            || !c
                .credential
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || c.routes.is_empty()
            || c.routes.len() > 4096
            || c.routes.iter().any(|p| {
                !p.starts_with('/')
                    || p.starts_with("//")
                    || p.contains(['?', '#', '%', '\\'])
                    || reserved(p)
            })
        {
            return Err(Error("invalid adapter scope routes or limits"));
        }
        let endpoint = if cfg!(windows) {
            c.endpoint.strip_prefix(r"\\.\pipe\").is_some_and(|s| {
                !s.is_empty()
                    && s.len() <= 128
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            })
        } else {
            c.endpoint.starts_with('/') && !c.endpoint.contains('\0')
        };
        if !endpoint {
            return Err(Error("private local IPC endpoint required"));
        }
        Ok(c)
    }
    pub(crate) fn credential(&self) -> &str {
        &self.credential
    }
    /// Exact route check. This is an inventory, not a local authorization policy.
    pub fn permits_target(&self, target: &str) -> bool {
        let path = target.split('?').next().unwrap_or("");
        !reserved(path) && self.routes.iter().any(|p| p == path)
    }
}
fn reserved(p: &str) -> bool {
    p == "/_humanymous" || p.starts_with("/_humanymous/")
}

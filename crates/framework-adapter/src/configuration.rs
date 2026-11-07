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
    pub server: humanymous_runtime::authority::configuration::Peer,
    /// Complete exact protected pathname inventory.
    pub routes: Vec<String>,
    /// Complete input/decision/handoff budget, milliseconds.
    pub timeout_ms: u64,
    /// At most1MiB of transfer-decoded entity bytes.
    pub max_body: usize,
    /// Sync host input deadline must be separately enforced by the server.
    #[serde(default)]
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
    /// Borrow the zeroizing supervisor secret only while preparing IPC.
    pub fn credential(&self) -> &str {
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

#[cfg(test)]
pub(crate) fn fixture() -> serde_json::Value {
    let endpoint = if cfg!(windows) {
        r"\\.\pipe\humanymous-framework-unit"
    } else {
        "/tmp/humanymous-framework-unit.sock"
    };
    serde_json::json!({"installation":"i","tenant":"t","origin":"o","revision":"r","endpoint":endpoint,
        "credential":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","server":{"kind":"unix","uid":0},
        "routes":["/api"],"timeoutMs":1000,"maxBody":8})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_config_has_no_unknown_duplicate_or_remote_fallback() {
        let value = fixture();
        let text = value.to_string();
        assert!(Configuration::parse(&text).is_ok());
        assert!(Configuration::parse(&text.replacen('{', "{\"tenant\":\"other\",", 1)).is_err());
        let mut bad = value.clone();
        bad["unknown"] = true.into();
        assert!(Configuration::parse(&bad.to_string()).is_err());
        let mut bad = value.clone();
        bad["endpoint"] = "http://localhost:1234".into();
        assert!(Configuration::parse(&bad.to_string()).is_err());
        let mut bad = value.clone();
        bad["maxBody"] = 1_048_577.into();
        assert!(Configuration::parse(&bad.to_string()).is_err());
        let mut bad = value;
        bad.as_object_mut().unwrap().remove("credential");
        assert!(Configuration::parse(&bad.to_string()).is_err());
    }
    #[test]
    fn routes_are_exact_and_management_stays_unavailable() {
        let c = Configuration::parse(&fixture().to_string()).unwrap();
        assert!(c.permits_target("/api?q=1"));
        for target in ["/api/sub", "/unknown", "/_humanymous", "/_humanymous/admin"] {
            assert!(!c.permits_target(target));
        }
        assert!(!c.sync_input_deadline_enforced);
    }
}

//! Opt-in HTML loader injection (legacy internal/gate gate.go
//! `modifyResponse` with inject.go at baseline 981d25c, SoT-20, HR-27a; owner
//! decision #7, plan `plan:root/20261121201500`).
//!
//! Off by default: without an `inject` object no response is changed. Go
//! injected on every route its presets marked; here the operator lists the
//! exact GET routes. Each must be an inventoried GET route on the
//! buffered response profile, never a reserved route. The snippet is Go's:
//! the idempotence marker and one `<script src="..." defer>` element whose
//! `src` is `loader_src`. Go served `loader.js` from its control plane; the
//! gateway serves no script (sot:root/client-collector forbids a
//! hand-authored loader), so `loader_src` names a same-origin path the
//! operator serves, such as the generated collector bundle. The mode is the
//! boot mode of `gate.inject`; an active settings overlay that names the gate
//! replaces it per request (owner decision #3).
use super::{ResponseProfile, Route};
use crate::gateway::ban::gate::Mode;
use serde::Deserialize;

/// Most listed inject routes.
pub const MAX_ROUTES: usize = 64;
/// Longest `loader_src`.
pub const MAX_LOADER_SRC: usize = 256;

/// `inject` section of the private startup document.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inject {
    /// Boot mode of `gate.inject`; default `enforce` (Go's default).
    #[serde(default = "enforce")]
    pub mode: Mode,
    /// Exact GET route paths whose HTML responses carry the loader.
    pub routes: Vec<String>,
    /// Same-origin absolute path of the loader script the operator serves.
    pub loader_src: String,
}

fn enforce() -> Mode {
    Mode::Enforce
}

fn loader_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"/._~-".contains(&byte)
}

impl Inject {
    /// Whether the section is usable with the configured route inventory.
    pub fn valid(&self, inventory: &[Route]) -> bool {
        let src = self.loader_src.as_bytes();
        let loader = src.len() <= MAX_LOADER_SRC
            && src.first() == Some(&b'/')
            && src.get(1) != Some(&b'/')
            && src.iter().copied().all(loader_byte)
            && !crate::authority::configuration::reserved(&self.loader_src);
        let mut seen = std::collections::BTreeSet::new();
        loader
            && !self.routes.is_empty()
            && self.routes.len() <= MAX_ROUTES
            && self.routes.iter().all(|path| {
                seen.insert(path.as_str())
                    && !crate::authority::configuration::reserved(path)
                    && inventory.iter().any(|route| {
                        route.method == "GET"
                            && &route.path == path
                            && route.response_profile == ResponseProfile::Buffered
                    })
            })
    }

    /// Whether a request for `method` and `target` is on an inject route.
    pub fn covers(&self, method: &str, target: &str) -> bool {
        let path = target.split('?').next().unwrap_or("");
        method == "GET" && self.routes.iter().any(|route| route == path)
    }

    /// Go's snippet: the marker, then one deferred external script element.
    pub fn snippet(&self) -> Vec<u8> {
        format!(
            "{}<script src=\"{}\" defer></script>",
            crate::gateway::inject::MARKER,
            self.loader_src
        )
        .into_bytes()
    }

    /// Nonsecret readiness view.
    pub fn readiness(&self) -> serde_json::Value {
        serde_json::json!({"mode":super::edge::mode_name(self.mode),"routes":self.routes.len(),
            "loader_src":self.loader_src})
    }
}

#[cfg(test)]
mod tests;

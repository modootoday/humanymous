//! Embed/iframe context inspection (SoT-10 §3.1): the embedding origin must be
//! allow-listed and the RIT token must survive into the embed context.
use super::classify::{go_lower, is_embed_destination};
use crate::signals::observation::{Signal, Source, Verdict};
use serde_json::json;

/// Caller-supplied evidence for one embed/iframe request.
#[derive(Debug, Clone, Copy, Default)]
pub struct EmbedContext<'a> {
    /// `Sec-Fetch-Dest` header value; empty when absent.
    pub sec_fetch_dest: &'a str,
    /// `Origin` header (or Referer host); empty when absent.
    pub origin: &'a str,
    /// Embedding allow-list; empty permits every origin.
    pub allowed_hosts: &'a [&'a str],
    /// Whether a RIT token accompanied the request.
    pub rit_present: bool,
}

impl EmbedContext<'_> {
    /// Whether the fetch destination is an embed context (exact, case-sensitive).
    pub fn is_embed(&self) -> bool {
        is_embed_destination(self.sec_fetch_dest)
    }
}

/// Return the frozen L5 embed signals for a request; non-embed requests yield none.
///
/// Order: `l5.embed.origin_disallowed` (non-empty origin outside a non-empty
/// allow-list), then `l5.embed.token_stripped` (RIT absent).
pub fn inspect(context: &EmbedContext<'_>) -> Vec<Signal> {
    let mut result = Vec::new();
    if !context.is_embed() {
        return result;
    }
    if !context.origin.is_empty() && !host_allowed(context.origin, context.allowed_hosts) {
        result.push(Signal::new(
            "l5.embed.origin_disallowed",
            json!(context.origin),
            Verdict::Suspicious,
            1.0,
            Source::Server,
            "embed from non-allowlisted origin",
        ));
    }
    if !context.rit_present {
        result.push(Signal::new(
            "l5.embed.token_stripped",
            json!(true),
            Verdict::Bot,
            1.0,
            Source::Server,
            "RIT token absent in embed context",
        ));
    }
    result
}

/// Empty allow-list permits every origin (legacy demo default).
fn host_allowed(origin: &str, allowed: &[&str]) -> bool {
    let origin = normalize_host(origin);
    allowed.is_empty() || allowed.iter().any(|host| normalize_host(host) == origin)
}

/// Trim, lowercase, drop one `https://` then one `http://` prefix, cut at the first `/`.
fn normalize_host(value: &str) -> String {
    let lowered = go_lower(value.trim());
    let host = lowered.strip_prefix("https://").unwrap_or(&lowered);
    let host = host.strip_prefix("http://").unwrap_or(host);
    host.split('/').next().unwrap_or_default().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(signals: &[Signal]) -> Vec<&str> {
        signals.iter().map(|signal| signal.id.as_str()).collect()
    }

    #[test]
    fn non_embed_destinations_yield_nothing() {
        for dest in ["", "document", "image", "IFRAME"] {
            let context = EmbedContext {
                sec_fetch_dest: dest,
                origin: "https://evil.com",
                allowed_hosts: &["myapp.local"],
                rit_present: false,
            };
            assert!(!context.is_embed());
            assert!(inspect(&context).is_empty());
        }
    }

    #[test]
    fn origin_normalization_and_empty_allow_list() {
        let allowed = ["MyApp.Local/path"];
        for (origin, disallowed) in [
            ("https://myapp.local", false),
            ("  HTTP://MYAPP.LOCAL/x ", false),
            ("https://http://myapp.local", false),
            ("http://https://myapp.local", true),
            ("https://myapp.local:443", true),
            ("https://evil.com", true),
            ("", false),
        ] {
            let context = EmbedContext {
                sec_fetch_dest: "embed",
                origin,
                allowed_hosts: &allowed,
                rit_present: true,
            };
            assert_eq!(
                ids(&inspect(&context)) == ["l5.embed.origin_disallowed"],
                disallowed,
                "{origin:?}"
            );
        }
        let open = EmbedContext {
            sec_fetch_dest: "object",
            origin: "https://evil.com",
            allowed_hosts: &[],
            rit_present: true,
        };
        assert!(inspect(&open).is_empty());
    }

    #[test]
    fn emitted_signals_use_registry_definitions() {
        let signals = inspect(&EmbedContext {
            sec_fetch_dest: "frame",
            origin: "https://evil.com",
            allowed_hosts: &["myapp.local"],
            rit_present: false,
        });
        assert_eq!(
            ids(&signals),
            ["l5.embed.origin_disallowed", "l5.embed.token_stripped"]
        );
        assert_eq!(signals[0].value, json!("https://evil.com"));
        assert_eq!(signals[0].verdict, Verdict::Suspicious);
        assert_eq!(signals[1].value, json!(true));
        assert_eq!(signals[1].verdict, Verdict::Bot);
        for signal in &signals {
            assert!(crate::signals::observation::lookup(&signal.id).is_some());
            assert_eq!(signal.collected, Source::Server);
            assert_eq!(signal.confidence, 1.0);
        }
    }
}

//! Node/Fetch observations projected onto the existing integrity-owned schema.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub(crate) fn error(reason: &str) -> napi::Error {
    napi::Error::from_reason(reason.to_owned())
}
pub(crate) fn now() -> napi::Result<u64> {
    humanymous_authority::now_ms().map_err(|_| error("clock unavailable"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Configuration {
    pub installation: String,
    pub tenant: String,
    pub origin: String,
    pub revision: String,
    pub endpoint: String,
    pub credential: String,
    pub server: humanymous_authority::configuration::Peer,
    pub routes: Vec<String>,
    pub timeout_ms: u32,
    pub max_body: u32,
}
impl Configuration {
    pub fn parse(value: &str) -> napi::Result<Self> {
        let c: Self =
            serde_json::from_str(value).map_err(|_| error("invalid configuration schema"))?;
        if c.routes.is_empty()
            || c.routes.len() > 4096
            || c.routes.iter().any(|p| {
                !p.starts_with('/')
                    || p.starts_with("//")
                    || p.contains(['?', '#', '%', '\\'])
                    || p == "/_humanymous"
                    || p.starts_with("/_humanymous/")
            })
            || c.timeout_ms == 0
            || c.timeout_ms > 30_000
            || c.max_body > 1_048_576
            || c.credential.len() < 32
            || c.credential.len() > 256
            || !c
                .credential
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || ![&c.installation, &c.tenant, &c.origin, &c.revision]
                .iter()
                .all(|s| {
                    !s.is_empty()
                        && s.len() <= 128
                        && s.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
                })
        {
            return Err(error("invalid scope, routes or limits"));
        }
        let local = if cfg!(windows) {
            c.endpoint.strip_prefix(r"\\.\pipe\").is_some_and(|s| {
                !s.is_empty()
                    && s.len() <= 128
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            })
        } else {
            c.endpoint.starts_with('/') && !c.endpoint.contains('\0')
        };
        if !local {
            return Err(error("local IPC endpoint required"));
        }
        Ok(c)
    }
}
pub(crate) struct Observation {
    pub method: String,
    pub target: String,
    pub authority: String,
    pub headers: Vec<[String; 2]>,
    pub peer: Option<String>,
    pub connection: Option<String>,
    pub raw: bool,
}
pub(crate) fn internal(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.starts_with("x-humanymous-")
        || n.starts_with("x-forwarded-")
        || [
            "forwarded",
            "cf-connecting-ip",
            "true-client-ip",
            "x-real-ip",
        ]
        .contains(&n.as_str())
}
pub(crate) fn uuid() -> napi::Result<String> {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).map_err(|_| error("entropy unavailable"))?;
    b[6] = (b[6] & 15) | 64;
    b[8] = (b[8] & 63) | 128;
    let h = hex::encode(b);
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    ))
}
pub(crate) fn encode(
    c: &Configuration,
    o: &mut Observation,
    body: &[u8],
    deadline: u64,
) -> napi::Result<Vec<u8>> {
    let path = o.target.split('?').next().unwrap_or("");
    if !c.routes.iter().any(|p| p == path) || body.len() > c.max_body as usize {
        return Err(error("route or body unavailable"));
    }
    o.headers.retain(|[name, _]| !internal(name));
    let value = serde_json::json!({"protocol":"humanymous.adapter/0.1","id":uuid()?,"tenant":c.tenant,"origin":c.origin,"revision":c.revision,"deadline":deadline,"method":o.method,"target":o.target,"authority":o.authority,"peer":o.peer,"headers":o.headers,"headerMode":if o.raw {"node-raw"} else {"fetch-normalized"},"body":{"layer":"transfer-decoded-content-encoded","complete":true,"length":body.len(),"sha256":hex::encode(Sha256::digest(body)),"base64":STANDARD.encode(body)},"capabilities":{"rawTls":false,"originalH2":false,"rawHeaderOrder":o.raw,"completeBody":true},"stream":null,"capture":null,"connectionId":o.connection,"adapter":{"id":"humanymous.node","version":"0.1.0-experimental.1","installation":c.installation}});
    let wire = serde_json::to_vec(&value).map_err(|_| error("envelope encoding"))?;
    humanymous_integrity::adapter::parse(&wire, now()?).map_err(error)?;
    Ok(wire)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Configuration {
        Configuration {
            installation: "i".into(),
            tenant: "t".into(),
            origin: "o".into(),
            revision: "r".into(),
            endpoint: String::new(),
            credential: String::new(),
            server: humanymous_authority::configuration::Peer::Unix { uid: 0 },
            routes: vec!["/api".into()],
            timeout_ms: 1000,
            max_body: 4,
        }
    }
    fn observation() -> Observation {
        Observation {
            method: "POST".into(),
            target: "/api?q=1".into(),
            authority: "example.test".into(),
            headers: vec![
                ["X-Forwarded-For".into(), "127.0.0.1".into()],
                ["Authorization".into(), "app".into()],
            ],
            peer: None,
            connection: None,
            raw: false,
        }
    }
    #[test]
    fn exact_body_and_sanitization() {
        let mut o = observation();
        let wire = encode(&config(), &mut o, &[0, 255], now().unwrap() + 1000).unwrap();
        let parsed = humanymous_integrity::adapter::parse(&wire, now().unwrap()).unwrap();
        assert_eq!(parsed.body(), [0, 255]);
        assert_eq!(
            parsed.headers(),
            [["Authorization".to_owned(), "app".to_owned()]]
        );
    }
    #[test]
    fn route_limit_protocol_and_provenance_reject() {
        let mut o = observation();
        assert!(encode(&config(), &mut o, b"12345", now().unwrap() + 1000).is_err());
        o.target = "/_humanymous/admin".into();
        assert!(encode(&config(), &mut o, b"", now().unwrap() + 1000).is_err());
        o = observation();
        o.peer = Some("127.0.0.1".into());
        assert!(encode(&config(), &mut o, b"", now().unwrap() + 1000).is_err());
        o = observation();
        o.headers.push(["Upgrade".into(), "websocket".into()]);
        assert!(encode(&config(), &mut o, b"", now().unwrap() + 1000).is_err());
    }
    #[test]
    fn entropy_ids_are_distinct() {
        let a = uuid().unwrap();
        assert_ne!(a, uuid().unwrap());
        assert_eq!(a.len(), 36);
    }
}

//! NAPI error translation; shared context owns config/envelope/admission.
pub(crate) use humanymous_framework_adapter::internal_header as internal;
pub(crate) use humanymous_framework_adapter::{Configuration, Observation};
pub(crate) fn error(reason: &str) -> napi::Error {
    napi::Error::from_reason(reason.to_owned())
}
fn boundary(e: humanymous_framework_adapter::Error) -> napi::Error {
    error(e.0)
}
pub(crate) fn parse(value: &str) -> napi::Result<Configuration> {
    Configuration::parse(value).map_err(boundary)
}
#[cfg(test)]
pub(crate) fn now() -> napi::Result<u64> {
    humanymous_authority::now_ms().map_err(|_| error("clock unavailable"))
}
pub(crate) fn uuid() -> napi::Result<String> {
    humanymous_framework_adapter::uuid().map_err(boundary)
}
#[cfg(test)]
pub(crate) fn encode(
    c: &Configuration,
    o: &mut Observation,
    body: &[u8],
    deadline: u64,
) -> napi::Result<Vec<u8>> {
    o.headers.retain(|[name, _]| !internal(name));
    humanymous_framework_adapter::encode(
        c,
        o,
        body,
        deadline,
        humanymous_core::integrity::adapter::AdapterProfile::Node,
    )
    .map_err(boundary)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Configuration {
        let endpoint = if cfg!(windows) {
            r"\\.\pipe\humanymous-unit"
        } else {
            "/tmp/humanymous-unit.sock"
        };
        parse(&serde_json::json!({"installation":"i","tenant":"t","origin":"o","revision":"r",
            "endpoint":endpoint,"credential":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","server":{"kind":"unix","uid":0},
            "routes":["/api"],"timeoutMs":1000,"maxBody":4}).to_string()).unwrap()
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
        let parsed = humanymous_core::integrity::adapter::parse(&wire, now().unwrap()).unwrap();
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

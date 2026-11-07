use crate::{Configuration, Error};
use base64::{Engine, engine::general_purpose::STANDARD};
use humanymous_authority::client::{self, BoundResponse, ClientOptions, ExpectedBinding};
use humanymous_integrity::adapter::{AdapterProfile, parse_with_profile};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

/// Host-observed normalized request; never original network capture.
pub struct Observation {
    /// Uppercase HTTP method.
    pub method: String,
    /// Actual raw origin-form target, including query.
    pub target: String,
    /// Actual host/authority.
    pub authority: String,
    /// Sanitized received header view; CGI maps have lost original order/duplicates.
    pub headers: Vec<[String; 2]>,
    /// Node HTTP1 peer only; normalized profiles must leave absent.
    pub peer: Option<String>,
    /// Node HTTP1 connection ID only; never original capture authority.
    pub connection: Option<String>,
    /// Original Node rawHeaders evidence, false for CGI/ASGI/Fetch.
    pub raw: bool,
}
/// Names not trusted or forwarded to the application as capture authority.
pub fn internal_header(name: &str) -> bool {
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
/// A single body/decision lifecycle, started before host input consumption.
pub struct Pending {
    deadline: u64,
    until: Instant,
}
impl Pending {
    /// Start one bounded wall-clock and monotonic budget.
    pub fn new(c: &Configuration) -> Result<Self, Error> {
        Ok(Self {
            deadline: humanymous_authority::now_ms()
                .map_err(|_| Error("clock unavailable"))?
                .checked_add(c.timeout_ms)
                .ok_or(Error("deadline overflow"))?,
            until: Instant::now() + Duration::from_millis(c.timeout_ms),
        })
    }
    /// Remaining input/IPC duration; a caller cannot reset the original budget.
    pub fn remaining(&self) -> Result<Duration, Error> {
        let now = humanymous_authority::now_ms().map_err(|_| Error("clock unavailable"))?;
        self.remaining_at(now, Instant::now())
    }
    fn remaining_at(&self, now: u64, monotonic: Instant) -> Result<Duration, Error> {
        let remaining = self
            .until
            .checked_duration_since(monotonic)
            .filter(|d| !d.is_zero());
        if now >= self.deadline || remaining.is_none() {
            return Err(Error("adapter deadline expired"));
        }
        Ok(remaining
            .unwrap()
            .min(Duration::from_millis(self.deadline - now)))
    }
    /// Encode observations and delegate validation/peer authentication/decision.
    pub async fn decide(
        &self,
        c: &Configuration,
        o: &Observation,
        body: &[u8],
        profile: AdapterProfile,
    ) -> Result<BoundResponse, Error> {
        let wire = encode(c, o, body, self.deadline, profile)?;
        let parsed = parse_with_profile(
            &wire,
            humanymous_authority::now_ms().map_err(|_| Error("clock unavailable"))?,
            profile,
        )
        .map_err(|_| Error("invalid framework request"))?;
        let timeout_ms = self.remaining()?.as_millis().min(c.timeout_ms as u128) as u64;
        if timeout_ms == 0 {
            return Err(Error("adapter deadline expired"));
        }
        client::decide(
            &ClientOptions {
                endpoint: &c.endpoint,
                credential: c.credential(),
                server: &c.server,
                timeout_ms,
                adapter: profile,
            },
            &wire,
            ExpectedBinding {
                id: parsed.id(),
                tenant: &c.tenant,
                origin: &c.origin,
                revision: &c.revision,
                deadline: self.deadline,
            },
        )
        .await
        .map_err(|_| Error("authority unavailable"))
    }
    /// Final host handoff liveness check, in addition to BoundResponse::consume.
    pub fn check_handoff(&self, expires: u64) -> Result<(), Error> {
        self.remaining()?;
        if humanymous_authority::now_ms().map_err(|_| Error("clock unavailable"))? >= expires {
            return Err(Error("authority result expired"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;

    #[test]
    fn repeated_stage_reads_preserve_original_monotonic_deadline() {
        let start = Instant::now();
        let pending = Pending {
            deadline: 5000,
            until: start + Duration::from_millis(100),
        };
        assert_eq!(
            pending.remaining_at(4900, start).unwrap(),
            Duration::from_millis(100)
        );
        assert_eq!(
            pending
                .remaining_at(4910, start + Duration::from_millis(10))
                .unwrap(),
            Duration::from_millis(90)
        );
        assert_eq!(
            pending
                .remaining_at(4990, start + Duration::from_millis(90))
                .unwrap(),
            Duration::from_millis(10)
        );
    }

    #[test]
    fn wall_rollback_cannot_extend_the_original_monotonic_budget() {
        let start = Instant::now();
        let pending = Pending {
            deadline: 5000,
            until: start + Duration::from_millis(100),
        };
        assert_eq!(
            pending
                .remaining_at(1, start + Duration::from_millis(99))
                .unwrap(),
            Duration::from_millis(1)
        );
        assert!(
            pending
                .remaining_at(1, start + Duration::from_millis(100))
                .is_err()
        );
        assert!(
            pending
                .remaining_at(1, start + Duration::from_millis(101))
                .is_err()
        );
    }

    #[test]
    fn either_original_clock_expiry_refuses_even_if_the_other_has_time() {
        let start = Instant::now();
        let pending = Pending {
            deadline: 5000,
            until: start + Duration::from_millis(100),
        };
        assert!(pending.remaining_at(5000, start).is_err());
        assert!(
            pending
                .remaining_at(4900, start + Duration::from_millis(100))
                .is_err()
        );
        assert_eq!(
            pending.remaining_at(4999, start).unwrap(),
            Duration::from_millis(1)
        );
    }
}

/// Native entropy-generated request identity; no caller-chosen replay nonce.
pub fn uuid() -> Result<String, Error> {
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce).map_err(|_| Error("entropy unavailable"))?;
    nonce[6] = (nonce[6] & 15) | 64;
    nonce[8] = (nonce[8] & 63) | 128;
    let h = hex::encode(nonce);
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    ))
}
/// Project the exact selected producer profile, then use the integrity parser.
pub fn encode(
    c: &Configuration,
    o: &Observation,
    body: &[u8],
    deadline: u64,
    profile: AdapterProfile,
) -> Result<Vec<u8>, Error> {
    let mode = match profile {
        AdapterProfile::Node if o.raw => "node-raw",
        AdapterProfile::Node => "fetch-normalized",
        AdapterProfile::PythonAsgi => "asgi-normalized",
        AdapterProfile::PythonWsgi => "wsgi-normalized",
        AdapterProfile::RubyRack => "rack-normalized",
        AdapterProfile::Apache => "apache-normalized",
        AdapterProfile::DotnetAspNetCore => "aspnetcore-normalized",
        _ => return Err(Error("unsupported framework profile")),
    };
    if (profile != AdapterProfile::Node && o.raw)
        || !c.permits_target(&o.target)
        || body.len() > c.max_body
    {
        return Err(Error("route body or provenance unavailable"));
    }
    let headers: Vec<_> = o
        .headers
        .iter()
        .filter(|[n, _]| !internal_header(n))
        .collect();
    let expected = profile.expected();
    let wire=serde_json::to_vec(&serde_json::json!({"protocol":"humanymous.adapter/0.1",
        "id":uuid()?,"tenant":c.tenant,"origin":c.origin,"revision":c.revision,"deadline":deadline,
        "method":o.method,"target":o.target,"authority":o.authority,"peer":o.peer,"headers":headers,
        "headerMode":mode,"body":{"layer":"transfer-decoded-content-encoded","complete":true,
        "length":body.len(),"sha256":hex::encode(Sha256::digest(body)),"base64":STANDARD.encode(body)},
        "capabilities":{"rawTls":false,"originalH2":false,"rawHeaderOrder":o.raw,"completeBody":true},
        "stream":null,"capture":null,"connectionId":o.connection,
        "adapter":{"id":expected.id,"version":expected.version,"installation":c.installation}}))
        .map_err(|_|Error("envelope encoding unavailable"))?;
    parse_with_profile(
        &wire,
        humanymous_authority::now_ms().map_err(|_| Error("clock unavailable"))?,
        profile,
    )
    .map_err(|_| Error("invalid framework request"))?;
    Ok(wire)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observation() -> Observation {
        Observation {
            method: "POST".into(),
            target: "/api?q=1".into(),
            authority: "example.test".into(),
            headers: vec![
                ["host".into(), "example.test".into()],
                ["content-length".into(), "2".into()],
                ["x-humanymous-capture".into(), "forged".into()],
            ],
            peer: None,
            connection: None,
            raw: false,
        }
    }
    #[test]
    fn normalized_profiles_bind_binary_bytes_without_raw_claims() {
        let c = Configuration::parse(&crate::configuration::fixture().to_string()).unwrap();
        for profile in [
            AdapterProfile::PythonAsgi,
            AdapterProfile::PythonWsgi,
            AdapterProfile::RubyRack,
            AdapterProfile::Apache,
            AdapterProfile::DotnetAspNetCore,
            AdapterProfile::Node,
        ] {
            let now = humanymous_authority::now_ms().unwrap();
            let wire = encode(&c, &observation(), &[0, 255], now + 1000, profile).unwrap();
            let r = parse_with_profile(&wire, now, profile).unwrap();
            assert_eq!(r.body(), [0, 255]);
            assert!(!r.headers().iter().any(|[n, _]| internal_header(n)));
            for other in [
                AdapterProfile::PythonAsgi,
                AdapterProfile::PythonWsgi,
                AdapterProfile::RubyRack,
                AdapterProfile::Apache,
                AdapterProfile::DotnetAspNetCore,
                AdapterProfile::Node,
            ] {
                if profile != other {
                    assert!(parse_with_profile(&wire, now, other).is_err());
                }
            }
        }
    }
    #[test]
    fn forged_raw_peer_and_expired_or_incomplete_body_reject() {
        let c = Configuration::parse(&crate::configuration::fixture().to_string()).unwrap();
        let now = humanymous_authority::now_ms().unwrap();
        let mut o = observation();
        o.raw = true;
        assert!(encode(&c, &o, &[0, 255], now + 1000, AdapterProfile::PythonAsgi).is_err());
        o.raw = false;
        o.peer = Some("127.0.0.1".into());
        assert!(encode(&c, &o, &[0, 255], now + 1000, AdapterProfile::RubyRack).is_err());
        o.peer = None;
        assert!(encode(&c, &o, &[0], now + 1000, AdapterProfile::PythonWsgi).is_err());
        assert!(encode(&c, &o, &[0, 255], now, AdapterProfile::PythonWsgi).is_err());
    }
}

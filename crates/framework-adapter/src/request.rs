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
        if now >= self.deadline || Instant::now() >= self.until {
            return Err(Error("adapter deadline expired"));
        }
        Ok((self.until - Instant::now()).min(Duration::from_millis(self.deadline - now)))
    }
    /// Encode observations and delegate validation/peer authentication/decision.
    pub async fn decide(
        &self,
        c: &Configuration,
        o: &Observation,
        body: &[u8],
        profile: AdapterProfile,
    ) -> Result<BoundResponse, Error> {
        let mode = match profile {
            AdapterProfile::PythonAsgi => "asgi-normalized",
            AdapterProfile::PythonWsgi => "wsgi-normalized",
            AdapterProfile::RubyRack => "rack-normalized",
            _ => return Err(Error("unsupported framework profile")),
        };
        if !c.permits_target(&o.target) || body.len() > c.max_body {
            return Err(Error("route or body unavailable"));
        }
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).map_err(|_| Error("entropy unavailable"))?;
        nonce[6] = (nonce[6] & 15) | 64;
        nonce[8] = (nonce[8] & 63) | 128;
        let h = hex::encode(nonce);
        let id = format!(
            "{}-{}-{}-{}-{}",
            &h[..8],
            &h[8..12],
            &h[12..16],
            &h[16..20],
            &h[20..]
        );
        let headers: Vec<_> = o
            .headers
            .iter()
            .filter(|[n, _]| !internal_header(n))
            .collect();
        let expected = profile.expected();
        let wire=serde_json::to_vec(&serde_json::json!({"protocol":"humanymous.adapter/0.1",
            "id":id,"tenant":c.tenant,"origin":c.origin,"revision":c.revision,"deadline":self.deadline,
            "method":o.method,"target":o.target,"authority":o.authority,"peer":null,"headers":headers,
            "headerMode":mode,"body":{"layer":"transfer-decoded-content-encoded","complete":true,
            "length":body.len(),"sha256":hex::encode(Sha256::digest(body)),"base64":STANDARD.encode(body)},
            "capabilities":{"rawTls":false,"originalH2":false,"rawHeaderOrder":false,"completeBody":true},
            "stream":null,"capture":null,"connectionId":null,
            "adapter":{"id":expected.id,"version":expected.version,"installation":c.installation}}))
            .map_err(|_| Error("envelope encoding unavailable"))?;
        parse_with_profile(
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
                id: &id,
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

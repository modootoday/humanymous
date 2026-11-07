//! Shared local decision client. It grants no origin or management authority.
use crate::{Rejection, configuration::Peer, protocol};
use humanymous_integrity::adapter::{AdapterProfile, parse_with_profile};
use serde::Deserialize;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

/// Private supervisor configuration; intentionally not Debug or Serialize.
pub struct ClientOptions<'a> {
    /// Absolute private filesystem socket or exact local pipe name.
    pub endpoint: &'a str,
    /// Scoped process-epoch bearer delivered privately by the supervisor.
    pub credential: &'a str,
    /// Actual server UID or SID/authority PID, never envelope peer metadata.
    pub server: &'a Peer,
    /// Entire connect/write/read budget in milliseconds.
    pub timeout_ms: u64,
    /// Provisioned producer profile, never inferred from the envelope.
    pub adapter: AdapterProfile,
}
/// Expected outstanding request binding, independent from response contents.
pub struct ExpectedBinding<'a> {
    /// Canonical request nonce.
    pub id: &'a str,
    /// Configured tenant.
    pub tenant: &'a str,
    /// Configured origin.
    pub origin: &'a str,
    /// Current configured revision.
    pub revision: &'a str,
    /// Exact outstanding request deadline.
    pub deadline: u64,
}
/// Frozen shared engine decision vocabulary, not a new scoring implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    /// May invoke the already bound origin once after final liveness checks.
    Allow,
    /// Refuse origin effects.
    Deny,
    /// Refuse effects; browser recovery is a separate capability.
    Challenge,
}
/// Single-owner bound decision. No serialization or Clone can mint tickets.
pub struct BoundResponse {
    verdict: Verdict,
    expires: u64,
    received: u64,
    until: Instant,
}
impl BoundResponse {
    /// Inspect the result; callers must consume and check liveness before origin.
    pub fn verdict(&self) -> Verdict {
        self.verdict
    }
    /// Actual authority expiry, which can be earlier than request.deadline.
    pub fn expires(&self) -> u64 {
        self.expires
    }
    /// Consume once, rejecting wall rollback and monotonic or authority expiry.
    pub fn consume(self) -> Result<Verdict, Rejection> {
        self.consume_at(crate::now_ms()?, Instant::now())
    }
    fn consume_at(self, now: u64, monotonic: Instant) -> Result<Verdict, Rejection> {
        if now < self.received || now >= self.expires || monotonic >= self.until {
            return Err(Rejection::new(503, "authority_expired"));
        }
        Ok(self.verdict)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    protocol: String,
    id: String,
    tenant: String,
    origin: String,
    revision: String,
    expires: u64,
    verdict: Verdict,
}
fn bound(
    bytes: &[u8],
    expected: &ExpectedBinding<'_>,
    started: u64,
    now: u64,
    until: Instant,
) -> Result<BoundResponse, Rejection> {
    let reply: Reply =
        serde_json::from_slice(bytes).map_err(|_| Rejection::new(503, "authority_response"))?;
    if reply.protocol != "humanymous.adapter/0.1"
        || reply.id != expected.id
        || reply.tenant != expected.tenant
        || reply.origin != expected.origin
        || reply.revision != expected.revision
    {
        return Err(Rejection::new(503, "authority_binding"));
    }
    if now < started || reply.expires <= now || reply.expires > expected.deadline {
        return Err(Rejection::new(503, "authority_expired"));
    }
    Ok(BoundResponse {
        verdict: reply.verdict,
        expires: reply.expires,
        received: now,
        until: until.min(Instant::now() + Duration::from_millis(reply.expires - now)),
    })
}
/// Verify the exact request and native server, perform one bounded exchange and
/// validate the exact reply. Dropping the future closes its IPC; never retry.
pub async fn decide(
    options: &ClientOptions<'_>,
    envelope: &[u8],
    expected: ExpectedBinding<'_>,
) -> Result<BoundResponse, Rejection> {
    let now = crate::now_ms()?;
    if !(1..=30_000).contains(&options.timeout_ms)
        || !(32..=256).contains(&options.credential.len())
        || !options
            .credential
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err(Rejection::new(503, "client_configuration"));
    }
    let request = parse_with_profile(envelope, now, options.adapter)
        .map_err(|_| Rejection::new(503, "client_envelope"))?;
    let (origin, revision, _) = request.origin_binding();
    if request.id() != expected.id
        || request.tenant() != expected.tenant
        || origin != expected.origin
        || revision != expected.revision
        || request.deadline() != expected.deadline
    {
        return Err(Rejection::new(503, "client_binding"));
    }
    let budget = Duration::from_millis(options.timeout_ms.min(expected.deadline - now));
    let until = Instant::now() + budget;
    tokio::time::timeout(budget,async{
        let mut stream=crate::transport::connect(options.endpoint,options.server).await.map_err(|_|Rejection::new(503,"client_peer_or_transport"))?;
        let head=format!("POST /adapter/v0.1/decision HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",options.credential,envelope.len());
        stream.write_all(head.as_bytes()).await.map_err(|_|Rejection::new(503,"client_write"))?;
        stream.write_all(envelope).await.map_err(|_|Rejection::new(503,"client_write"))?;
        let response=protocol::read_response(&mut stream).await.map_err(|_|Rejection::new(503,"authority_framing"))?;
        if response.status!=200{return Err(Rejection::new(503,"authority_status"));}
        bound(&response.body,&expected,now,crate::now_ms()?,until)
    }).await.map_err(|_|Rejection::new(503,"authority_timeout"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn expected() -> ExpectedBinding<'static> {
        ExpectedBinding {
            id: "01234567-89ab-4cde-8fab-0123456789ab",
            tenant: "t",
            origin: "o",
            revision: "r",
            deadline: 6000,
        }
    }
    fn reply() -> serde_json::Value {
        json!({"protocol":"humanymous.adapter/0.1","id":expected().id,"tenant":"t","origin":"o","revision":"r","expires":5000,"verdict":"ALLOW"})
    }
    fn check(bytes: &[u8]) -> Result<BoundResponse, Rejection> {
        bound(
            bytes,
            &expected(),
            1000,
            1000,
            Instant::now() + Duration::from_secs(3),
        )
    }
    #[test]
    fn rejects_omission_duplicate_unknown_mismatch_and_expiry() {
        let original = reply();
        let bytes = serde_json::to_vec(&original).unwrap();
        assert_eq!(check(&bytes).unwrap().verdict(), Verdict::Allow);
        for key in original.as_object().unwrap().keys() {
            let mut v = original.clone();
            v.as_object_mut().unwrap().remove(key);
            assert!(
                check(&serde_json::to_vec(&v).unwrap()).is_err(),
                "missing {key}"
            );
        }
        for key in ["id", "tenant", "origin", "revision", "protocol"] {
            let mut v = original.clone();
            v[key] = "wrong".into();
            assert!(check(&serde_json::to_vec(&v).unwrap()).is_err());
        }
        for expires in [0, 1000, 6001, u64::MAX] {
            let mut v = original.clone();
            v["expires"] = expires.into();
            assert!(check(&serde_json::to_vec(&v).unwrap()).is_err());
        }
        for verdict in ["unknown", "allow", ""] {
            let mut v = original.clone();
            v["verdict"] = verdict.into();
            assert!(check(&serde_json::to_vec(&v).unwrap()).is_err());
        }
        for prefix in [
            "\"tenant\":\"t\",",
            "\"ten\\u0061nt\":\"t\",",
            "\"unknown\":true,",
        ] {
            let s = format!(
                "{{{prefix}{}",
                &String::from_utf8(bytes.clone()).unwrap()[1..]
            );
            assert!(check(s.as_bytes()).is_err());
        }
        assert!(check(&[0xff]).is_err());
        assert!(
            bound(
                &bytes,
                &expected(),
                1001,
                1000,
                Instant::now() + Duration::from_secs(3)
            )
            .is_err()
        );
    }
    #[test]
    fn authority_expiry_is_independent_of_request_and_monotonic_budgets() {
        let monotonic = Instant::now();
        let bytes = serde_json::to_vec(&reply()).unwrap();
        // The request permits until6000 and the local transport budget remains
        // live; neither can extend the authority's earlier5000 expiry.
        for (wall, accepted) in [(4999, true), (5000, false), (5001, false)] {
            let response = bound(
                &bytes,
                &expected(),
                1000,
                1000,
                monotonic + Duration::from_secs(60),
            )
            .unwrap();
            assert_eq!(response.expires(), 5000);
            let result = response.consume_at(wall, monotonic);
            if accepted {
                assert_eq!(result.unwrap(), Verdict::Allow);
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.status, 503);
                assert_eq!(error.code, "authority_expired");
            }
        }
    }
    #[test]
    fn consuming_rechecks_authority_and_monotonic_expiry() {
        let now = crate::now_ms().unwrap();
        let expired = BoundResponse {
            verdict: Verdict::Allow,
            expires: now + 10_000,
            received: now,
            until: Instant::now() - Duration::from_millis(1),
        };
        assert!(expired.consume().is_err());
        let rollback = BoundResponse {
            verdict: Verdict::Allow,
            expires: now + 10_000,
            received: now + 1000,
            until: Instant::now() + Duration::from_secs(1),
        };
        assert!(rollback.consume().is_err());
        let valid = BoundResponse {
            verdict: Verdict::Challenge,
            expires: now + 10_000,
            received: now,
            until: Instant::now() + Duration::from_secs(1),
        };
        assert_eq!(valid.consume().unwrap(), Verdict::Challenge);
    }
    #[tokio::test]
    async fn response_framing_rejects_truncation_smuggling_and_extra_bytes() {
        let good=b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}";
        assert_eq!(
            protocol::read_response(&mut good.as_slice())
                .await
                .unwrap()
                .status,
            200
        );
        for text in [
            String::from_utf8(good.to_vec()).unwrap().replace(
                "Content-Length: 2",
                "Content-Length: 2\r\ncontent-length: 2",
            ),
            String::from_utf8(good.to_vec()).unwrap().replace(
                "Connection: close",
                "Transfer-Encoding: chunked\r\nConnection: close",
            ),
            String::from_utf8(good.to_vec())
                .unwrap()
                .replace("close", "keep-alive"),
            format!("{}extra", String::from_utf8(good.to_vec()).unwrap()),
            String::from_utf8(good[..good.len() - 1].to_vec()).unwrap(),
        ] {
            assert!(protocol::read_response(&mut text.as_bytes()).await.is_err());
        }
    }
}

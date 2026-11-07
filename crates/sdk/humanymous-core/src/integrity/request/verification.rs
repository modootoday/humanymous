use super::{seed, verify_token};
use crate::signals::observation::{Signal, Source, Verdict};
use serde_json::json;

/// Caller-supplied request evidence. No clocks, storage or HTTP abstractions.
#[derive(Debug, Default)]
pub struct VerifyParams<'a> {
    /// Expected per-session secret.
    pub session_key: &'a [u8],
    /// Last seed counter issued by the server.
    pub last_n: u64,
    /// Counter presented by the client.
    pub presented_n: u64,
    /// Bucket presented by the client.
    pub presented_tb: u64,
    /// Bucket observed by the server.
    pub current_tb: u64,
    /// Exact encoded token; empty means absent.
    pub token: &'a str,
    /// Canonical string rebuilt from observed request bytes.
    pub canonical: &'a str,
    /// Bootstrap grace before any token has been issued.
    pub first_req: bool,
}

/// Return the frozen L5 RIT signal in the original failure-precedence order.
pub fn verify(params: VerifyParams<'_>) -> Signal {
    let signal =
        |id, value, verdict, notes| Signal::new(id, value, verdict, 1.0, Source::Server, notes);
    if params.first_req {
        return signal(
            "l5.rit.ok",
            json!("bootstrap"),
            Verdict::Ok,
            "first request, token grace",
        );
    }
    if params.token.is_empty() {
        return signal(
            "l5.rit.absent",
            json!(true),
            Verdict::Bot,
            "no RIT token on API call",
        );
    }
    if params.presented_n != params.last_n.wrapping_add(1) {
        return signal(
            "l5.rit.stale_replay",
            json!(params.presented_n),
            Verdict::Suspicious,
            "RIT counter out of sequence (replay)",
        );
    }
    if params.presented_tb.abs_diff(params.current_tb) > 1 {
        return signal(
            "l5.rit.stale_replay",
            json!(params.presented_tb),
            Verdict::Suspicious,
            "RIT time bucket outside window (replay)",
        );
    }
    let seed = seed(params.session_key, params.last_n);
    if !verify_token(
        &seed,
        params.canonical,
        params.presented_n,
        params.presented_tb,
        params.token,
    ) {
        return signal(
            "l5.rit.header_tampered",
            json!(true),
            Verdict::Bot,
            "signed request != observed (header/body tampered)",
        );
    }
    signal("l5.rit.ok", json!(true), Verdict::Ok, "RIT valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::request::{compute_token, session_key};

    #[test]
    fn valid_roundtrip_tamper_replay_and_bootstrap() {
        let key = session_key(b"master", "session");
        let seed = seed(&key, 0);
        let token = compute_token(&seed, "observed", 1, 1000);
        let params = || VerifyParams {
            session_key: &key,
            last_n: 0,
            presented_n: 1,
            presented_tb: 1000,
            current_tb: 1000,
            token: &token,
            canonical: "observed",
            first_req: false,
        };
        assert_eq!(verify(params()).id, "l5.rit.ok");
        assert_eq!(
            verify(VerifyParams {
                canonical: "tampered",
                ..params()
            })
            .id,
            "l5.rit.header_tampered"
        );
        assert_eq!(
            verify(VerifyParams {
                last_n: 5,
                ..params()
            })
            .id,
            "l5.rit.stale_replay"
        );
        assert_eq!(
            verify(VerifyParams {
                current_tb: 1005,
                ..params()
            })
            .id,
            "l5.rit.stale_replay"
        );
        assert_eq!(
            verify(VerifyParams {
                token: "",
                ..params()
            })
            .id,
            "l5.rit.absent"
        );
        assert_eq!(
            verify(VerifyParams {
                first_req: true,
                ..Default::default()
            })
            .value,
            json!("bootstrap")
        );
    }

    #[test]
    fn missing_token_precedes_sequence_errors() {
        assert_eq!(
            verify(VerifyParams {
                last_n: 5,
                presented_n: 9,
                ..Default::default()
            })
            .id,
            "l5.rit.absent"
        );
    }
}

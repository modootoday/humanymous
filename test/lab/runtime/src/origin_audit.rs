//! Roadmap 2c cross-namespace origin/audit effect binding.
//!
//! Four actors share one runtime image and run in separate containers:
//!
//! - `origin`: an owned recording origin on the backend network only. It logs
//!   every TCP contact (with the peer address) to stdout before reading it.
//! - `gate`: a supervisor that starts the shipped `humanymous-runtime` binary.
//!   The gateway accepts loopback listen/origin addresses and loopback peers
//!   only, so the supervisor composes two plain TCP relays in the Gate
//!   namespace: ingress `0.0.0.0:8080` to the runtime listener, and the
//!   runtime's loopback origin address to `oa-origin:18080` on the backend
//!   network. The Gate container joins both networks; its sealed custody and
//!   WAL live on named volumes so they survive a container restart.
//! - `client`: a Rust HTTP/1.1 client on the ingress network only.
//! - `tamper`: a network-none actor that mounts only the WAL volume (never the
//!   custody volume), copies the live history and mutates the copy.
//!
//! Go parity (981d25c): `deployments/compose/defenders.yaml` puts the origin on
//! `web` only, the Gate on `web` plus the lab subnets, and the bots on a lab
//! subnet only (`bots.yaml`); `internal/gate/gate.go` proxies to the single
//! configured upstream. `internal/audit/wal_test.go` replays the WAL across a
//! restart with contiguous sequence, recovers a torn tail and refuses framed
//! corruption.
pub mod client;
pub mod gate;
pub mod origin;
pub mod relay;
pub mod tamper;

use serde_json::{Value, json};

/// Runtime listener inside the Gate namespace (loopback only).
pub const GATE_LISTEN: &str = "127.0.0.1:18443";
/// Runtime origin address inside the Gate namespace: the backend relay.
pub const ORIGIN_LOOPBACK: &str = "127.0.0.1:18080";
/// Origin service name and port on the backend network.
pub const ORIGIN_UPSTREAM: &str = "oa-origin:18080";
/// Origin port, also probed directly by the client.
pub const ORIGIN_PORT: u16 = 18080;
/// Gate ingress relay port on the ingress network.
pub const FRONTEND_PORT: u16 = 8080;
/// Runtime loopback port, probed on the Gate ingress address by the client.
pub const RUNTIME_PORT: u16 = 18443;
/// Sealed custody volume mount.
pub const KEYS: &str = "/writer/keys";
/// WAL volume mount; histories live in subdirectories.
pub const WAL_ROOT: &str = "/writer/wal";
/// The live history directory under `WAL_ROOT`.
pub const LIVE: &str = "live";
/// Audit node identity.
pub const NODE: &str = "origin-audit-gate";
/// Protected route.
pub const EFFECT: &str = "/effect";
/// Client report schema.
pub const CLIENT_SCHEMA: &str = "humanymous.origin-audit-client/1";
/// Client rows, in execution order, identical for every phase.
pub const CLIENT_ROWS: &[&str] = &[
    "origin-direct-unreachable",
    "gate-loopback-ports-unreachable",
    "session-issued-via-gate",
    "unsigned-refused",
    "absolute-form-refused",
    "signed-effect-forwarded",
    "replay-refused",
];
/// Tamper modes. `copy` and `torn` must still start; the rest must refuse.
pub const TAMPER_ACCEPTED: &[&str] = &["copy", "torn"];
/// Tamper modes the runtime must refuse at startup.
pub const TAMPER_REFUSED: &[&str] = &["forge", "truncate", "rollback"];

/// The Gate's private configuration document. The client builds the same
/// document with placeholder custody fields, because request signing binds
/// only the trusted scope and routes.
pub fn document(
    audit: &str,
    keys: &str,
    passphrase: &str,
    allow_new: bool,
    trusted_head: Value,
    uid: u32,
) -> Value {
    json!({"listen":GATE_LISTEN,"origin":ORIGIN_LOOPBACK,
      "scope":{"installation":"origin-audit","tenant":"lab","origin":"origin","revision":"r1","authority":"owned.test"},
      "routes":[{"method":"POST","path":EFFECT,"required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,"collector":false,"rit":true}}],
      "audit":{"directory":audit,"node_id":NODE,"keys":{"directory":keys,"passphrase":passphrase},"allow_new":allow_new,"trusted_head":trusted_head},
      "owner":{"kind":"unix","uid":uid},"session_capacity":16,"session_ttl_ms":60000,"timeout_ms":5000,"rate_per_second":1000})
}

/// A history directory name: `live` or `case-<lowercase mode>`. Anything else
/// (separators, dots, other names) is refused before any path is formed.
pub fn history_name(name: &str) -> bool {
    name == LIVE
        || name.strip_prefix("case-").is_some_and(|mode| {
            !mode.is_empty() && mode.len() <= 16 && mode.bytes().all(|b| b.is_ascii_lowercase())
        })
}

/// Write one JSON line to stdout and flush it.
pub fn emit(value: &Value) -> std::io::Result<()> {
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, value)?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

/// Read one bounded JSON line from a reader.
pub fn read_line(reader: &mut impl std::io::BufRead, limit: u64) -> std::io::Result<Value> {
    use std::io::{BufRead, Read};
    let mut line = Vec::new();
    Read::take(&mut *reader, limit + 1).read_until(b'\n', &mut line)?;
    if line.is_empty() || line.len() as u64 > limit || line.last() != Some(&b'\n') {
        return Err(std::io::Error::other("missing or oversized control line"));
    }
    serde_json::from_slice(&line).map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use humanymous_runtime::gateway::configuration::Configuration;

    #[test]
    fn document_is_a_valid_loopback_gateway_configuration() {
        let parsed: Configuration =
            serde_json::from_value(document("/a", "/k", "p", true, Value::Null, 65532)).unwrap();
        assert!(parsed.validate().is_ok());
        assert_eq!(parsed.listen.to_string(), GATE_LISTEN);
        assert_eq!(parsed.origin.to_string(), ORIGIN_LOOPBACK);
        assert_eq!(parsed.routes.len(), 1);
        assert!(ORIGIN_UPSTREAM.ends_with(&format!(":{ORIGIN_PORT}")));
        assert!(GATE_LISTEN.ends_with(&format!(":{RUNTIME_PORT}")));
    }

    #[test]
    fn history_names_cannot_escape_the_wal_root() {
        for good in ["live", "case-copy", "case-rollback"] {
            assert!(history_name(good), "{good}");
        }
        for bad in [
            "",
            "case-",
            "case-../x",
            "case-a/b",
            "../live",
            "live/",
            "case-Forge",
            "keys",
        ] {
            assert!(!history_name(bad), "{bad}");
        }
        for mode in TAMPER_ACCEPTED.iter().chain(TAMPER_REFUSED) {
            assert!(history_name(&format!("case-{mode}")));
        }
    }

    #[test]
    fn control_lines_are_bounded_and_newline_terminated() {
        let mut ok = std::io::Cursor::new(b"{\"a\":1}\n".to_vec());
        assert_eq!(read_line(&mut ok, 64).unwrap()["a"], 1);
        let mut torn = std::io::Cursor::new(b"{\"a\":1}".to_vec());
        assert!(read_line(&mut torn, 64).is_err());
        let mut long = std::io::Cursor::new(b"{\"a\":\"xxxxxxxxxx\"}\n".to_vec());
        assert!(read_line(&mut long, 8).is_err());
    }
}

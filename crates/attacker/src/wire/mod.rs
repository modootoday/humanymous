//! Owned-lab red-team wire clients ported from the Go `cmd/redteam` /
//! `cmd/tlsparrot` oracle (repo 981d25c).
//!
//! Defense-only (AGENTS.md hard constraint 4): every client here targets only
//! this deployment's owned lab runtime, and no stealth capability exists beyond
//! the Go oracle's own test knobs. The guards are concrete:
//! - [`client::LoopbackClient`] refuses any non-loopback socket address, at
//!   construction and again on every send;
//! - the ClientHello builder's SNI is fixed to the reserved name `owned.test`
//!   (RFC 6761 `.test`) and has no setter;
//! - the raw h2 `:authority` is caller-supplied, but [`h2::H2Request::new`]
//!   refuses anything other than a loopback IP, `localhost` or a `.test` name.
//!
//! The ClientHello and h2 builders construct bytes that are structurally
//! modelled on the oracle presets (utls v1.8.2 Chrome_Auto, Chrome_100,
//! Firefox_Auto; Go stock `crypto/tls`; the raw h2 framer and Go's
//! `x/net/http2` Transport) and are accepted by the runtime's own observers
//! (TLS ClientHello capture, HTTP/2 capture, JA3/JA4). They are not byte-for-
//! byte copies: key shares, ECH payloads, random and session id are filler, and
//! GREASE values and the Chrome_Auto shuffle are deterministic. Completing a
//! live TLS/h2 handshake is a recorded gap: the only in-tree TLS/h2 stack is
//! `rustls`, which builds its own ClientHello and cannot present a third-party
//! parrot, so handshake-level execution against the live detector is not
//! reproducible without a new dependency. See the FM11 ledger for the gaps.
pub mod attacks;
pub mod client;
pub mod clienthello;
pub mod h2;
pub mod rit;

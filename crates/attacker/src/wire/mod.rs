//! Owned-lab red-team wire clients ported from the Go `cmd/redteam` /
//! `cmd/tlsparrot` oracle (repo 981d25c).
//!
//! Defense-only (AGENTS.md hard constraint 4): every client here targets only
//! this deployment's owned lab runtime. The loopback client refuses routable
//! targets, the ClientHello/H2 builders carry only a loopback SNI/authority,
//! and no stealth capability exists beyond the Go oracle's own test knobs.
//!
//! The modules construct exactly the bytes the Go clients put on the wire and
//! verify them through the runtime's own observers (TLS ClientHello capture,
//! HTTP/2 capture, JA3). Completing a live TLS/h2 handshake is a recorded gap:
//! the only in-tree TLS/h2 stack is `rustls`, which builds its own ClientHello
//! and cannot present a byte-exact third-party parrot, so handshake-level
//! execution against the live detector is not reproducible without a new
//! dependency. See the FM11 ledger for the measured reason.
pub mod attacks;
pub mod client;
pub mod clienthello;
pub mod h2;
pub mod rit;

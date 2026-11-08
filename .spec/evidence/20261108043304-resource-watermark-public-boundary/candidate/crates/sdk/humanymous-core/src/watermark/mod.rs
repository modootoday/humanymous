//! Resource watermark issuance primitives and bounded volatile custody.
//!
//! Legacy-v1 carriers identify an issuance, not a human or a policy verdict.
//! Low-level carrier transforms do not validate source grammar or authorize HTTP
//! issuance. PNG and standards-validated WASM codecs remain unimplemented.
//!
//! # Public consumer and private custody boundary
//!
//! Framing is corruption detection, not successful session authentication. Public
//! payloads and syntax choices are caller input, not identity/grammar proofs.
//! Actual issuance/trace composition must authenticate the caller, validate input
//! semantics, commit issuance before response and verify HMAC against live custody.
//!
//! The public context remains usable without exporting its bounded scan internals:
//! ```
//! use humanymous_core::watermark::frame::{self, Limits};
//! use humanymous_core::watermark::ledger::{Record, Reservation};
//! use humanymous_core::watermark::payload::WatermarkKey;
//! let _ = std::mem::size_of::<Record>();
//! let _ = std::mem::size_of::<Reservation>();
//! let _key = WatermarkKey::new(vec![1; 32]).unwrap();
//! let limits = Limits::new(1024, 2048, 64).unwrap();
//! let encoded = frame::encode(b"public consumer", limits).unwrap();
//! assert_eq!(frame::recover(&encoded, limits).unwrap(), b"public consumer");
//! ```
//!
//! External consumers cannot create recovery candidates outside admitted scans:
//! ```compile_fail
//! use humanymous_core::watermark::frame::Candidates;
//! ```
//! ```compile_fail
//! use humanymous_core::watermark::frame::Budget;
//! ```
//! They cannot forge unchecked admission limits:
//! ```compile_fail
//! use humanymous_core::watermark::frame::Limits;
//! let _unchecked = Limits { input: usize::MAX, output: usize::MAX, candidates: usize::MAX };
//! ```
//! Ledger custody cannot be read through the public handle:
//! ```compile_fail
//! use std::time::Duration;
//! use humanymous_core::watermark::ledger::{Configuration, Ledger};
//! let ledger = Ledger::new(Configuration {
//!     record_ttl: Duration::from_secs(10), reservation_ttl: Duration::from_secs(2), capacity: 1,
//! }, Duration::ZERO).unwrap();
//! let _custody = &ledger.inner;
//! ```
//! Reservations must be allocated by their owning ledger:
//! ```compile_fail
//! use humanymous_core::watermark::ledger::Reservation;
//! let _constructor = Reservation::new;
//! ```
//! Public input data cannot be promoted into a committed record:
//! ```compile_fail
//! use std::time::Duration;
//! use humanymous_core::watermark::{ledger::Record, payload::Payload};
//! let _record = Record {
//!     payload: Payload { tag_id: 1, session_id: "caller".into(), asset_id: "/asset".into(), bucket: 1 },
//!     mime: "application/octet-stream".into(), channels: vec!["bin-trailer".into()],
//!     ip_pseudonym: None, issued_at: Duration::ZERO,
//! };
//! ```
//! Owned secret bytes are not exposed by the public key wrapper:
//! ```compile_fail
//! use humanymous_core::watermark::payload::WatermarkKey;
//! let key = WatermarkKey::new(vec![1; 32]).unwrap();
//! let _secret = &key.0;
//! ```
//! Private encoding machinery is not an SDK compatibility surface:
//! ```compile_fail
//! use humanymous_core::watermark::codec::base32;
//! ```

pub mod codec;
mod error;
pub mod frame;
pub mod ledger;
pub mod payload;

pub use error::Error;

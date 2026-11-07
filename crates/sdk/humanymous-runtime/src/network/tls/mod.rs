//! Raw TLS ClientHello observation and protocol fingerprint derivation.
mod fingerprint;
mod hello;

pub use fingerprint::{engine_from_client_hello, extension_order_hash, ja3, ja4};
pub use hello::{ClientHello, ParseError, parse_client_hello, parse_record};

pub(in crate::network) fn is_grease(value: u16) -> bool {
    value & 0x0f0f == 0x0a0a && value >> 8 == value & 0xff
}

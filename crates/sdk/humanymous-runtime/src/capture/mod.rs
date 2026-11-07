//! Bounded original-hop TLS and HTTP/2 capture before HTTP normalization.
//!
//! Capture is transient transport evidence, never a human attestation. Runtime
//! consumers must bind it to their request and retain required-plane refusal
//! until actual ingress integration has been qualified.
pub mod http2;
pub mod tls;

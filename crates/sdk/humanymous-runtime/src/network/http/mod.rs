//! Header observations distinguish raw wire evidence from normalized maps.
mod client;
mod headers;
mod peek;
mod projection;
pub use client::{
    client_ip, client_subnet, direct_peer, forwarded_ip, is_datacenter_ip, is_trusted_proxy,
    parse_ip,
};
pub use headers::HeaderInfo;
pub use peek::{
    PEEK_DEADLINE, PEEK_LIMIT, Peek, PeekError, WireOrder, peek_header_order, peek_tcp,
};
pub use projection::{canonical_name, first_value, project_request};

#[cfg(test)]
mod tests;

//! PROXY protocol v2 peer recovery behind an explicit trusted-balancer allowlist.
//!
//! Shipping-plan row B26: a transport-level peer allowlist owned by Network. The
//! declared source only replaces the socket peer for address-keyed state when the
//! actual TCP peer is an allowlisted balancer. It never creates TLS or original-
//! client provenance (network-capture N01) and never reads HTTP forwarding fields.
//! An untrusted peer's bytes are not inspected at all, so a spoofed header stays
//! ordinary payload for the next layer.
use crate::network::intel::prefix_set::{Floor, Prefix};
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// The 12-byte PROXY protocol v2 signature.
pub const SIGNATURE: [u8; 12] = [
    0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A,
];
/// Narrowest accepted prefix length; anything broader lets clients spoof sources.
pub const MIN_PREFIX: u8 = 8;
/// Bound on the number of trusted entries in one allowlist.
pub const MAX_ENTRIES: usize = 256;

/// Static refusal codes; no peer or header bytes are interpolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// An allowlist entry is not an IP address or CIDR.
    Cidr,
    /// An allowlist entry is broader than `MIN_PREFIX`.
    TooBroad,
    /// The allowlist is empty or exceeds `MAX_ENTRIES`.
    Allowlist,
    /// Signature or version is not PROXY v2.
    NotProxyV2,
    /// Command is neither LOCAL (0x0) nor PROXY (0x1); 0x2-0xF are reserved.
    Command,
    /// The address block is shorter than its declared family needs.
    ShortAddress,
    /// The stream ended or failed before the complete header.
    Truncated,
}

/// One trusted network. Parsing and membership reuse the network context's
/// single CIDR primitive; this type only adds the allowlist breadth limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cidr(Prefix);

impl Cidr {
    /// Parse `a.b.c.d/n`, `v6/n` or a bare address (host route).
    pub fn parse(text: &str) -> Result<Self, Refusal> {
        let prefix = Prefix::parse_masked(text).map_err(|_| Refusal::Cidr)?;
        Floor {
            v4: MIN_PREFIX,
            v6: MIN_PREFIX,
        }
        .check(&prefix)
        .map_err(|_| Refusal::TooBroad)?;
        Ok(Self(prefix))
    }

    /// Membership; an IPv4-mapped IPv6 peer matches IPv4 networks.
    pub fn contains(&self, ip: IpAddr) -> bool {
        self.0.contains(ip)
    }
}

/// Non-empty bounded set of trusted balancer networks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowlist(Vec<Cidr>);

impl Allowlist {
    /// Parse a comma/space separated list. Empty input is a refusal: with no
    /// trusted balancer the caller must not enable PROXY v2 at all.
    pub fn parse(list: &str) -> Result<Self, Refusal> {
        let entries = list
            .split([',', ' '])
            .filter(|f| !f.is_empty())
            .map(Cidr::parse)
            .collect::<Result<Vec<_>, _>>()?;
        if entries.is_empty() || entries.len() > MAX_ENTRIES {
            return Err(Refusal::Allowlist);
        }
        Ok(Self(entries))
    }

    /// Whether the actual TCP peer is a trusted balancer.
    pub fn trusts(&self, peer: IpAddr) -> bool {
        self.0.iter().any(|cidr| cidr.contains(peer))
    }

    /// Parsed entries in input order.
    pub fn entries(&self) -> &[Cidr] {
        &self.0
    }
}

/// Decoded header meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Header {
    /// LOCAL command (balancer health check): keep the socket peer.
    Local,
    /// UNSPEC, UDP or UNIX family: keep the socket peer.
    Unspecified,
    /// PROXY over TCP with the declared source address.
    Source(SocketAddr),
}

/// Read exactly one PROXY v2 header and nothing after it, so the following
/// TLS ClientHello stays on the stream. Callers bound this with a deadline.
pub fn read_header(stream: &mut impl Read) -> Result<Header, Refusal> {
    let mut fixed = [0u8; 16];
    stream
        .read_exact(&mut fixed)
        .map_err(|_| Refusal::Truncated)?;
    if fixed[..12] != SIGNATURE || fixed[12] & 0xF0 != 0x20 {
        return Err(Refusal::NotProxyV2);
    }
    // Only LOCAL (0x0) and PROXY (0x1) are defined; a reserved command is
    // refused before its address block is read, never treated as PROXY.
    let command = fixed[12] & 0x0F;
    if command > 0x01 {
        return Err(Refusal::Command);
    }
    let mut block = vec![0u8; usize::from(u16::from_be_bytes([fixed[14], fixed[15]]))];
    stream
        .read_exact(&mut block)
        .map_err(|_| Refusal::Truncated)?;
    if command == 0x00 {
        return Ok(Header::Local);
    }
    let port = |at: usize| u16::from_be_bytes([block[at], block[at + 1]]);
    match fixed[13] {
        0x11 => {
            if block.len() < 12 {
                return Err(Refusal::ShortAddress);
            }
            let ip = Ipv4Addr::new(block[0], block[1], block[2], block[3]);
            Ok(Header::Source(SocketAddr::new(ip.into(), port(8))))
        }
        0x21 => {
            if block.len() < 36 {
                return Err(Refusal::ShortAddress);
            }
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&block[..16]);
            let ip = Ipv6Addr::from(octets);
            Ok(Header::Source(SocketAddr::new(ip.into(), port(32))))
        }
        _ => Ok(Header::Unspecified),
    }
}

/// Outcome of the trust gate for one accepted connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Peer {
    /// Untrusted peer: no bytes were read; the socket peer stands.
    Socket(SocketAddr),
    /// Trusted balancer declared this source.
    Declared(SocketAddr),
    /// Trusted balancer sent LOCAL/UNSPEC: the socket peer stands.
    Balancer(SocketAddr),
}

impl Peer {
    /// Address for address-keyed state (bans, rates, correlation).
    pub fn address(&self) -> SocketAddr {
        match *self {
            Self::Socket(a) | Self::Declared(a) | Self::Balancer(a) => a,
        }
    }
}

/// Apply the trust gate. A malformed header from a trusted balancer is a
/// refusal; the caller drops that connection and keeps serving others.
pub fn resolve(
    allowlist: &Allowlist,
    socket_peer: SocketAddr,
    stream: &mut impl Read,
) -> Result<Peer, Refusal> {
    if !allowlist.trusts(socket_peer.ip()) {
        return Ok(Peer::Socket(socket_peer));
    }
    Ok(match read_header(stream)? {
        Header::Source(source) => Peer::Declared(source),
        Header::Local | Header::Unspecified => Peer::Balancer(socket_peer),
    })
}

#[cfg(test)]
mod tests;

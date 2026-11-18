//! PROXY protocol v2 peer recovery behind an explicit trusted-balancer allowlist.
//!
//! Shipping-plan row B26: a transport-level peer allowlist owned by Network. The
//! declared source only replaces the socket peer for address-keyed state when the
//! actual TCP peer is an allowlisted balancer. It never creates TLS or original-
//! client provenance (network-capture N01) and never reads HTTP forwarding fields.
//! An untrusted peer's bytes are not inspected at all, so a spoofed header stays
//! ordinary payload for the next layer.
//!
//! Port of legacy internal/gate proxyproto.go at baseline 981d25c, checked
//! against `tests/fixtures/network/proxyproto-go-golden.json`. Only PROXY v2 is
//! accepted; a v1 text header is refused like any other non-v2 prefix, as in
//! Go. The accepting listener is `gateway::proxy_listener`.
//!
//! Deliberate fail-closed differences from Go, pinned by the golden test:
//! - An empty allowlist is a refusal (Go returned no networks and no error).
//! - A prefix length with a leading zero (`/08`) is refused (Go accepted it).
//! - An IPv4-mapped IPv6 entry is an IPv4 network whose breadth is checked
//!   after mapping: Go accepted `::ffff:0:0/96`, which trusts every IPv4 peer,
//!   and turned a bare `::ffff:a.b.c.d` into `::/32`.
//! - More than [`MAX_ENTRIES`] entries is a refusal.
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// The 12-byte PROXY protocol v2 signature.
pub const SIGNATURE: [u8; 12] = [
    0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A,
];
/// Length of the fixed v2 header: signature, version/command, family, length.
pub const FIXED_LEN: usize = 16;
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
    /// The address block is shorter than its declared family needs.
    ShortAddress,
    /// The stream ended or failed before the complete header.
    Truncated,
}

/// One trusted network: masked address plus prefix length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cidr {
    network: IpAddr,
    prefix: u8,
}

impl Cidr {
    /// Parse `a.b.c.d/n`, `v6/n` or a bare address (host route).
    pub fn parse(text: &str) -> Result<Self, Refusal> {
        let (address, prefix) = match text.split_once('/') {
            Some((address, prefix)) => (address, Some(prefix)),
            None => (text, None),
        };
        let address: IpAddr = address.parse().map_err(|_| Refusal::Cidr)?;
        let width = if address.is_ipv4() { 32 } else { 128 };
        let prefix = match prefix {
            None => width,
            Some(p)
                if !p.is_empty()
                    && p.len() <= 3
                    && p.bytes().all(|b| b.is_ascii_digit())
                    && (p.len() == 1 || !p.starts_with('0')) =>
            {
                p.parse::<u8>().map_err(|_| Refusal::Cidr)?
            }
            Some(_) => return Err(Refusal::Cidr),
        };
        if prefix > width {
            return Err(Refusal::Cidr);
        }
        // An IPv4-mapped entry names IPv4 space, as Go matched it; its breadth
        // is the IPv4 prefix length that remains after the 96-bit mapping.
        let (address, prefix) = match address {
            IpAddr::V6(v6) if prefix >= 96 => match v6.to_ipv4_mapped() {
                Some(v4) => (IpAddr::V4(v4), prefix - 96),
                None => (address, prefix),
            },
            _ => (address, prefix),
        };
        if prefix < MIN_PREFIX {
            return Err(Refusal::TooBroad);
        }
        Ok(Self {
            network: mask(address, prefix),
            prefix,
        })
    }

    /// Membership. An IPv4-mapped IPv6 peer is the IPv4 peer it maps, so it
    /// matches IPv4 networks only, as Go's `IPNet.Contains` did.
    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = ip.to_canonical();
        ip.is_ipv4() == self.network.is_ipv4() && mask(ip, self.prefix) == self.network
    }

    /// Masked network address.
    pub fn network(&self) -> IpAddr {
        self.network
    }

    /// Prefix length in the family of [`network`](Self::network).
    pub fn prefix(&self) -> u8 {
        self.prefix
    }
}

fn mask(ip: IpAddr, prefix: u8) -> IpAddr {
    match ip {
        IpAddr::V4(v4) => {
            let bits = u32::from(v4) & u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0);
            IpAddr::V4(Ipv4Addr::from(bits))
        }
        IpAddr::V6(v6) => {
            let bits = u128::from(v6) & u128::MAX.checked_shl(128 - u32::from(prefix)).unwrap_or(0);
            IpAddr::V6(Ipv6Addr::from(bits))
        }
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

    /// Whether the actual TCP peer address is a trusted balancer.
    pub fn trusts(&self, peer: IpAddr) -> bool {
        self.0.iter().any(|cidr| cidr.contains(peer))
    }

    /// Whether the actual TCP peer socket is a trusted balancer. A scoped
    /// (zoned) IPv6 peer is never trusted: Go could not parse its zoned
    /// address text and kept the socket peer.
    pub fn trusts_peer(&self, peer: SocketAddr) -> bool {
        match peer {
            SocketAddr::V6(v6) if v6.scope_id() != 0 => false,
            _ => self.trusts(peer.ip()),
        }
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

/// Validate the fixed 16-byte header and return the address block length
/// that follows it.
pub fn block_len(fixed: &[u8; FIXED_LEN]) -> Result<usize, Refusal> {
    if fixed[..12] != SIGNATURE || fixed[12] & 0xF0 != 0x20 {
        return Err(Refusal::NotProxyV2);
    }
    Ok(usize::from(u16::from_be_bytes([fixed[14], fixed[15]])))
}

/// Decode a complete header: the fixed part (already accepted by
/// [`block_len`]) and its whole address block. An IPv4-mapped TCP6 source is
/// the IPv4 source it maps, as Go rendered and keyed it.
pub fn decode(fixed: &[u8; FIXED_LEN], block: &[u8]) -> Result<Header, Refusal> {
    if fixed[12] & 0x0F == 0x00 {
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
            let ip = IpAddr::V6(Ipv6Addr::from(octets)).to_canonical();
            Ok(Header::Source(SocketAddr::new(ip, port(32))))
        }
        _ => Ok(Header::Unspecified),
    }
}

/// Read exactly one PROXY v2 header and nothing after it, so the following
/// TLS ClientHello stays on the stream. Callers bound this with a deadline.
pub fn read_header(stream: &mut impl Read) -> Result<Header, Refusal> {
    let mut fixed = [0u8; FIXED_LEN];
    stream
        .read_exact(&mut fixed)
        .map_err(|_| Refusal::Truncated)?;
    let mut block = vec![0u8; block_len(&fixed)?];
    stream
        .read_exact(&mut block)
        .map_err(|_| Refusal::Truncated)?;
    decode(&fixed, &block)
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
    if !allowlist.trusts_peer(socket_peer) {
        return Ok(Peer::Socket(socket_peer));
    }
    Ok(match read_header(stream)? {
        Header::Source(source) => Peer::Declared(source),
        Header::Local | Header::Unspecified => Peer::Balancer(socket_peer),
    })
}

#[cfg(test)]
mod tests;

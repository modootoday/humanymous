//! Legacy Core client-address model (`clientIP` family in headerinfo.go and
//! `isDatacenterIP` in server.go, Go oracle 981d25c).
//!
//! These are pure string transforms for the legacy scoring inputs. They never
//! authenticate a transport peer: the result is not provenance, management
//! locality or a PROXY v2 recovered source (shipping-plan row B26 keeps the
//! transport-level allowlist in `network::proxy`). A caller must not treat a
//! forwarded value accepted here as an authenticated client identity.
//!
//! Deliberate difference from Go (owner decision C1, issue #75, 2026-11-18:
//! trusted-proxy XFF only from an explicit CIDR allowlist, default off). Go
//! trusted every loopback, private and link-local peer implicitly. Here the
//! trust set is the caller-supplied `trusted` allowlist; an empty allowlist,
//! the default, trusts nothing and [`client_ip`] always returns the direct
//! peer. For a peer inside the allowlist, header choice, the leftmost
//! nonblank X-Forwarded-For rule, the X-Real-IP fallback and the subnet
//! derivation stay Go-identical.
use super::{headers::trim_space, projection::first_value};
use crate::network::proxy::Cidr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Go `net.ParseIP`: textual IPv4 or IPv6 without a zone. An IPv4-mapped IPv6
/// address is returned as IPv4, as Go `IP.To4` treats it.
pub fn parse_ip(text: &str) -> Option<IpAddr> {
    match text.parse::<IpAddr>().ok()? {
        IpAddr::V6(v6) => Some(v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4)),
        v4 => Some(v4),
    }
}

fn local_v4(ip: Ipv4Addr) -> bool {
    ip.is_loopback() || ip.is_private() || ip.is_link_local()
}

fn local_v6(ip: Ipv6Addr) -> bool {
    let head = ip.segments()[0];
    ip.is_loopback() || head & 0xfe00 == 0xfc00 || head & 0xffc0 == 0xfe80
}

// Go IsLoopback || IsPrivate || IsLinkLocalUnicast.
fn local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => local_v4(v4),
        IpAddr::V6(v6) => local_v6(v6),
    }
}

/// Legacy `isTrustedProxy` against an explicit allowlist (C1): `ip` is
/// trusted only when it parses and a `trusted` entry contains it. Go's
/// implicit loopback, private and link-local trust is removed, so an empty
/// allowlist trusts nothing. Unparseable text is never trusted.
pub fn is_trusted_proxy(ip: &str, trusted: &[Cidr]) -> bool {
    parse_ip(ip).is_some_and(|ip| trusted.iter().any(|net| net.contains(ip)))
}

/// Legacy `isDatacenterIP`. `member` is the operator-supplied datacenter range
/// set; an empty set (`|_| false`) fails open. Unparseable, loopback, private,
/// link-local and unspecified addresses are never datacenter.
pub fn is_datacenter_ip(ip: &str, member: impl Fn(IpAddr) -> bool) -> bool {
    match parse_ip(ip) {
        Some(ip) if !local(ip) && !ip.is_unspecified() => member(ip),
        _ => false,
    }
}

// Go net.SplitHostPort host part, or None where Go reports an AddrError.
fn split_host(hostport: &str) -> Option<&str> {
    let last = hostport.rfind(':')?;
    let (host, from_open, from_close) = if hostport.starts_with('[') {
        let end = hostport.find(']')?;
        if end + 1 != last {
            return None;
        }
        (&hostport[1..end], 1, end + 1)
    } else {
        let host = &hostport[..last];
        if host.contains(':') {
            return None;
        }
        (host, 0, 0)
    };
    if hostport[from_open..].contains('[') || hostport[from_close..].contains(']') {
        return None;
    }
    Some(host)
}

/// Legacy `directPeer`: the host of `host:port`, else the text with every
/// leading and trailing bracket removed.
pub fn direct_peer(remote_addr: &str) -> String {
    split_host(remote_addr)
        .unwrap_or_else(|| remote_addr.trim_matches(['[', ']']))
        .to_owned()
}

/// Legacy `clientIP`: from a peer inside the `trusted` allowlist, the first
/// nonblank X-Forwarded-For member, else a nonblank X-Real-IP, else the peer.
/// From any other peer, and always with an empty allowlist, the peer. The
/// value is unvalidated text.
pub fn client_ip(remote_addr: &str, fields: &[(String, String)], trusted: &[Cidr]) -> String {
    let peer = direct_peer(remote_addr);
    if !is_trusted_proxy(&peer, trusted) {
        return peer;
    }
    let chain = first_value(fields, "X-Forwarded-For");
    if let Some(first) = chain
        .split(',')
        .map(trim_space)
        .find(|part| !part.is_empty())
    {
        return first.to_owned();
    }
    match trim_space(first_value(fields, "X-Real-IP")) {
        "" => peer,
        real => real.to_owned(),
    }
}

/// Legacy `forwardedIP`: the client-asserted address with no trust decision,
/// i.e. the first X-Forwarded-For member (possibly blank) or X-Real-IP.
pub fn forwarded_ip(fields: &[(String, String)]) -> String {
    let chain = first_value(fields, "X-Forwarded-For");
    let claimed = if chain.is_empty() {
        first_value(fields, "X-Real-IP")
    } else {
        chain.split_once(',').map_or(chain, |(first, _)| first)
    };
    trim_space(claimed).to_owned()
}

/// Legacy `clientSubnet`: the first three dot-separated labels of
/// [`client_ip`] (same `trusted` allowlist) when it has exactly three dots,
/// else the whole value.
pub fn client_subnet(remote_addr: &str, fields: &[(String, String)], trusted: &[Cidr]) -> String {
    let ip = client_ip(remote_addr, fields, trusted);
    if ip.matches('.').count() == 3 {
        return ip.splitn(4, '.').take(3).collect::<Vec<_>>().join(".");
    }
    ip
}

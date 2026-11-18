//! Pure immutable IPv4/IPv6 prefix set with sorted-range lookup.
//!
//! [`Prefix`] is the single CIDR primitive of the network context; the PROXY v2
//! balancer allowlist reuses it. [`PrefixSet`] merges prefixes into sorted,
//! non-overlapping integer ranges, so a lookup is one binary search per family.
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Static refusal codes; no input text is interpolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixError {
    /// Not an IP address, or the length is malformed or wider than the family.
    Syntax,
    /// Canonical form was required but host bits are set.
    HostBits,
    /// Shorter than the caller's [`Floor`] for its family.
    TooBroad,
}

impl fmt::Display for PrefixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Syntax => "prefix syntax",
            Self::HostBits => "prefix host bits set",
            Self::TooBroad => "prefix too broad",
        })
    }
}
impl std::error::Error for PrefixError {}

/// One network: masked address plus prefix length. Serialized as canonical
/// `address/len` text; deserialization refuses non-canonical text.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct Prefix {
    network: IpAddr,
    len: u8,
}

impl Prefix {
    /// Parse `a.b.c.d/n`, `v6/n` or a bare address (host route) and mask host
    /// bits. Use this for third-party list input that may not be canonical.
    pub fn parse_masked(text: &str) -> Result<Self, PrefixError> {
        let (address, len) = split(text)?;
        Ok(Self {
            network: mask(address, len),
            len,
        })
    }

    /// Parse the canonical form only: host bits must already be zero. Use this
    /// for our own snapshot format, where non-canonical text means corruption.
    pub fn parse_canonical(text: &str) -> Result<Self, PrefixError> {
        let (address, len) = split(text)?;
        if mask(address, len) != address {
            return Err(PrefixError::HostBits);
        }
        Ok(Self {
            network: address,
            len,
        })
    }

    /// Masked network address.
    pub fn network(&self) -> IpAddr {
        self.network
    }

    /// Prefix length in bits.
    pub fn len(&self) -> u8 {
        self.len
    }

    /// Whether this is a zero-length (default-route) prefix.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Membership; an IPv4-mapped IPv6 address matches IPv4 networks.
    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = match ip {
            IpAddr::V6(v6) if self.network.is_ipv4() => match v6.to_ipv4_mapped() {
                Some(v4) => IpAddr::V4(v4),
                None => return false,
            },
            other => other,
        };
        ip.is_ipv4() == self.network.is_ipv4() && mask(ip, self.len) == self.network
    }

    fn range(&self) -> Range {
        match self.network {
            IpAddr::V4(v4) => {
                let start = u32::from(v4);
                let host = u32::MAX.checked_shr(u32::from(self.len)).unwrap_or(0);
                Range::V4(start, start | host)
            }
            IpAddr::V6(v6) => {
                let start = u128::from(v6);
                let host = u128::MAX.checked_shr(u32::from(self.len)).unwrap_or(0);
                Range::V6(start, start | host)
            }
        }
    }
}

impl fmt::Display for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.network, self.len)
    }
}

impl TryFrom<String> for Prefix {
    type Error = PrefixError;
    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse_canonical(&text)
    }
}
impl From<Prefix> for String {
    fn from(prefix: Prefix) -> Self {
        prefix.to_string()
    }
}

/// Shortest accepted prefix length per address family. The single breadth
/// check of the network context: a caller states its floor, and a prefix
/// shorter than it is refused as [`PrefixError::TooBroad`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Floor {
    /// Minimum IPv4 prefix length.
    pub v4: u8,
    /// Minimum IPv6 prefix length.
    pub v6: u8,
}

impl Floor {
    /// Refuse `prefix` when it is shorter than this floor for its family.
    pub fn check(&self, prefix: &Prefix) -> Result<(), PrefixError> {
        let floor = if prefix.network.is_ipv4() {
            self.v4
        } else {
            self.v6
        };
        if prefix.len < floor {
            return Err(PrefixError::TooBroad);
        }
        Ok(())
    }
}

fn split(text: &str) -> Result<(IpAddr, u8), PrefixError> {
    let (address, len) = match text.split_once('/') {
        Some((address, len)) => (address, Some(len)),
        None => (text, None),
    };
    let address: IpAddr = address.parse().map_err(|_| PrefixError::Syntax)?;
    let width = if address.is_ipv4() { 32 } else { 128 };
    let len = match len {
        None => width,
        Some(p)
            if !p.is_empty()
                && p.len() <= 3
                && p.bytes().all(|b| b.is_ascii_digit())
                && (p.len() == 1 || !p.starts_with('0')) =>
        {
            p.parse::<u8>().map_err(|_| PrefixError::Syntax)?
        }
        Some(_) => return Err(PrefixError::Syntax),
    };
    if len > width {
        return Err(PrefixError::Syntax);
    }
    Ok((address, len))
}

fn mask(ip: IpAddr, len: u8) -> IpAddr {
    match ip {
        IpAddr::V4(v4) => {
            let bits = u32::from(v4) & u32::MAX.checked_shl(32 - u32::from(len)).unwrap_or(0);
            IpAddr::V4(Ipv4Addr::from(bits))
        }
        IpAddr::V6(v6) => {
            let bits = u128::from(v6) & u128::MAX.checked_shl(128 - u32::from(len)).unwrap_or(0);
            IpAddr::V6(Ipv6Addr::from(bits))
        }
    }
}

enum Range {
    V4(u32, u32),
    V6(u128, u128),
}

/// Immutable set of prefixes as sorted, merged, inclusive integer ranges.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrefixSet {
    v4: Vec<(u32, u32)>,
    v6: Vec<(u128, u128)>,
}

impl PrefixSet {
    /// Build from any prefixes; overlapping and adjacent prefixes are merged.
    /// Memory is bounded by the input, which the snapshot format caps.
    pub fn new(prefixes: impl IntoIterator<Item = Prefix>) -> Self {
        let (mut v4, mut v6) = (Vec::new(), Vec::new());
        for prefix in prefixes {
            match prefix.range() {
                Range::V4(start, end) => v4.push((start, end)),
                Range::V6(start, end) => v6.push((start, end)),
            }
        }
        Self {
            v4: merge(v4),
            v6: merge(v6),
        }
    }

    /// Membership; an IPv4-mapped IPv6 address is looked up as IPv4.
    pub fn contains(&self, ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(v4) => lookup(&self.v4, u32::from(v4)),
            IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
                Some(v4) => lookup(&self.v4, u32::from(v4)),
                None => lookup(&self.v6, u128::from(v6)),
            },
        }
    }

    /// Number of merged ranges (IPv4, IPv6); a size gauge, not a prefix count.
    pub fn ranges(&self) -> (usize, usize) {
        (self.v4.len(), self.v6.len())
    }

    /// Whether no address is covered.
    pub fn is_empty(&self) -> bool {
        self.v4.is_empty() && self.v6.is_empty()
    }

    /// Number of distinct addresses covered (IPv4, IPv6). Ranges are merged,
    /// so overlap is counted once. The IPv6 total saturates at `u128::MAX`,
    /// which only the whole family (`::/0`) reaches.
    pub fn coverage(&self) -> (u64, u128) {
        let v4 = self
            .v4
            .iter()
            .map(|&(start, end)| u64::from(end - start) + 1)
            .sum();
        let v6 = self.v6.iter().fold(0u128, |total, &(start, end)| {
            total.saturating_add((end - start).saturating_add(1))
        });
        (v4, v6)
    }
}

fn merge<T: Ord + Copy + Successor>(mut ranges: Vec<(T, T)>) -> Vec<(T, T)> {
    ranges.sort_unstable();
    let mut merged: Vec<(T, T)> = Vec::with_capacity(ranges.len());
    for (start, end) in ranges {
        if let Some(last) = merged.last_mut()
            && (start <= last.1 || last.1.successor() == Some(start))
        {
            last.1 = last.1.max(end);
            continue;
        }
        merged.push((start, end));
    }
    merged.shrink_to_fit();
    merged
}

fn lookup<T: Ord + Copy>(ranges: &[(T, T)], value: T) -> bool {
    let after = ranges.partition_point(|&(start, _)| start <= value);
    after > 0 && ranges[after - 1].1 >= value
}

/// Checked increment shared by both address widths, for adjacency merging.
trait Successor: Sized {
    fn successor(self) -> Option<Self>;
}
impl Successor for u32 {
    fn successor(self) -> Option<Self> {
        self.checked_add(1)
    }
}
impl Successor for u128 {
    fn successor(self) -> Option<Self> {
        self.checked_add(1)
    }
}

#[cfg(test)]
mod tests;

//! Pure source contract: one registry of public sources as data, and one
//! [`normalize`] entry point that turns upstream bytes into canonical prefixes.
//!
//! No network or filesystem access happens here. The operator updater (the
//! `humanymous ip-intel` CLI composition root) acquires bytes from pinned git
//! submodule checkouts or public HTTP endpoints and passes them in.
use super::{IntelCategory, prefix_set::Prefix};
use std::fmt;

mod format;

/// Directory holding one git submodule per [`SourceOrigin::Git`] source, named
/// exactly like the registry entry.
pub const SUBMODULE_ROOT: &str = "third_party/ip-intel";
/// Largest accepted upstream body per URL or file.
pub const MAX_SOURCE_BYTES: usize = 32 << 20;
/// Largest number of prefixes one source may contribute.
pub const MAX_SOURCE_PREFIXES: usize = 1_000_000;

/// Upstream wire format; parsers are grouped by format, not by provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// AWS `ip-ranges.json`.
    AwsJson,
    /// Google Cloud `cloud.json`.
    GcpJson,
    /// Azure Service Tags `ServiceTags_Public_*.json`.
    AzureServiceTags,
    /// Oracle Cloud `public_ip_ranges.json`.
    OracleJson,
    /// RFC 8805 geofeed CSV (first column is the prefix).
    Geofeed,
    /// One address or CIDR per line, `#`/`;` comments (netset, ipset, Tor list).
    Netset,
    /// Spamhaus DROP JSON lines with a trailing metadata record.
    SpamhausJsonl,
}

/// Where the operator updater obtains a source's bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceOrigin {
    /// Files inside the pinned submodule `third_party/ip-intel/<name>`. The
    /// gitlink SHA is the source of truth for the data version.
    Git {
        /// Upstream repository, recorded in `.gitmodules`.
        repository: &'static str,
        /// Files relative to the submodule root, concatenated.
        files: &'static [&'static str],
    },
    /// Public HTTP endpoints fetched at update time (never vendored).
    Http {
        /// URLs whose bodies are normalized and combined.
        urls: &'static [&'static str],
    },
    /// The endpoint rotates; the operator passes the current URL or a file.
    OperatorUrl {
        /// Stable page where the operator finds the current URL.
        discovery: &'static str,
    },
}

/// One registry entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpec {
    /// Stable name; also the submodule directory name for git sources.
    pub name: &'static str,
    /// Category of every prefix from this source.
    pub category: IntelCategory,
    /// Licence or terms note recorded in each snapshot.
    pub licence: &'static str,
    /// Upstream wire format.
    pub format: Format,
    /// Acquisition path.
    pub origin: SourceOrigin,
}

impl SourceSpec {
    /// Repository-relative submodule path for git sources.
    pub fn submodule_path(&self) -> Option<String> {
        matches!(self.origin, SourceOrigin::Git { .. })
            .then(|| format!("{SUBMODULE_ROOT}/{}", self.name))
    }
}

/// The complete source registry. Only free, public sources that need no paid
/// service or owner-owned account. See `sot:root/ip-reputation` for terms.
pub const REGISTRY: &[SourceSpec] = &[
    SourceSpec {
        name: "aws-ip-ranges",
        category: IntelCategory::Datacenter,
        licence: "AWS published endpoint; no redistribution grant: runtime fetch only",
        format: Format::AwsJson,
        origin: SourceOrigin::Http {
            urls: &["https://ip-ranges.amazonaws.com/ip-ranges.json"],
        },
    },
    SourceSpec {
        name: "gcp-cloud",
        category: IntelCategory::Datacenter,
        licence: "Google published endpoint; no redistribution grant: runtime fetch only",
        format: Format::GcpJson,
        origin: SourceOrigin::Http {
            urls: &["https://www.gstatic.com/ipranges/cloud.json"],
        },
    },
    SourceSpec {
        name: "azure-service-tags",
        category: IntelCategory::Datacenter,
        licence: "Microsoft Download Center terms; rotating URL: operator fetch only",
        format: Format::AzureServiceTags,
        origin: SourceOrigin::OperatorUrl {
            discovery: "https://www.microsoft.com/en-us/download/details.aspx?id=56519",
        },
    },
    SourceSpec {
        name: "oracle-cloud",
        category: IntelCategory::Datacenter,
        licence: "Oracle published endpoint; no redistribution grant: runtime fetch only",
        format: Format::OracleJson,
        origin: SourceOrigin::Http {
            urls: &["https://docs.oracle.com/en-us/iaas/tools/public_ip_ranges.json"],
        },
    },
    SourceSpec {
        name: "digitalocean-geofeed",
        category: IntelCategory::Datacenter,
        licence: "DigitalOcean RFC 8805 geofeed; no redistribution grant: runtime fetch only",
        format: Format::Geofeed,
        origin: SourceOrigin::Http {
            urls: &["https://digitalocean.com/geo/google.csv"],
        },
    },
    SourceSpec {
        name: "linode-geofeed",
        category: IntelCategory::Datacenter,
        licence: "Akamai/Linode RFC 8805 geofeed; no redistribution grant: runtime fetch only",
        format: Format::Geofeed,
        origin: SourceOrigin::Http {
            urls: &["https://geoip.linode.com/"],
        },
    },
    SourceSpec {
        name: "x4bnet-datacenter",
        category: IntelCategory::Datacenter,
        licence: "MIT (generated lists); upstream IPLocate ASN data CC BY-SA 4.0",
        format: Format::Netset,
        origin: SourceOrigin::Git {
            repository: "https://github.com/X4BNet/lists_vpn.git",
            files: &["output/datacenter/ipv4.txt", "output/datacenter/ipv6.txt"],
        },
    },
    SourceSpec {
        name: "tor-bulk-exit",
        category: IntelCategory::TorExit,
        licence: "Tor Project public exit list; runtime fetch only",
        format: Format::Netset,
        origin: SourceOrigin::Http {
            urls: &["https://check.torproject.org/torbulkexitlist"],
        },
    },
    SourceSpec {
        name: "spamhaus-drop",
        category: IntelCategory::Blocklist,
        licence: "Spamhaus DROP terms: free use, no IP licence, no name in marketing; runtime fetch only",
        format: Format::SpamhausJsonl,
        origin: SourceOrigin::Http {
            urls: &[
                "https://www.spamhaus.org/drop/drop_v4.json",
                "https://www.spamhaus.org/drop/drop_v6.json",
            ],
        },
    },
    SourceSpec {
        name: "firehol-level1",
        category: IntelCategory::Blocklist,
        licence: "No repository licence; per-list upstream terms (includes DShield, Spamhaus DROP)",
        format: Format::Netset,
        origin: SourceOrigin::Git {
            repository: "https://github.com/firehol/blocklist-ipsets.git",
            files: &["firehol_level1.netset"],
        },
    },
];

/// Look up a registry entry by name.
pub fn find(name: &str) -> Option<&'static SourceSpec> {
    REGISTRY.iter().find(|spec| spec.name == name)
}

/// Static refusal codes. `line` is a 1-based line number for line formats and
/// zero for document formats; no input text is interpolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceError {
    /// Body exceeds [`MAX_SOURCE_BYTES`].
    TooLarge,
    /// Not UTF-8, or not the expected document structure.
    Malformed {
        /// 1-based line, or zero for a whole document.
        line: usize,
    },
    /// An entry is not an address or CIDR.
    Prefix {
        /// 1-based line, or zero for a whole document.
        line: usize,
    },
    /// No prefixes at all; an empty list is never treated as clean data.
    Empty,
    /// More than [`MAX_SOURCE_PREFIXES`] prefixes.
    TooMany,
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => f.write_str("source_too_large"),
            Self::Malformed { line } => write!(f, "source_malformed:{line}"),
            Self::Prefix { line } => write!(f, "source_prefix:{line}"),
            Self::Empty => f.write_str("source_empty"),
            Self::TooMany => f.write_str("source_too_many_prefixes"),
        }
    }
}
impl std::error::Error for SourceError {}

/// Normalize one upstream body into sorted, de-duplicated canonical prefixes.
/// Host bits in upstream text are masked; anything else malformed refuses the
/// whole body rather than keeping a partial list.
pub fn normalize(format: Format, bytes: &[u8]) -> Result<Vec<Prefix>, SourceError> {
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(SourceError::TooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| SourceError::Malformed { line: 0 })?;
    let mut prefixes = match format {
        Format::AwsJson => format::aws_json::parse(text)?,
        Format::GcpJson => format::gcp_json::parse(text)?,
        Format::AzureServiceTags => format::azure_tags::parse(text)?,
        Format::OracleJson => format::oracle_json::parse(text)?,
        Format::Geofeed => format::geofeed::parse(text)?,
        Format::Netset => format::netset::parse(text)?,
        Format::SpamhausJsonl => format::spamhaus_jsonl::parse(text)?,
    };
    prefixes.sort_unstable();
    prefixes.dedup();
    match prefixes.len() {
        0 => Err(SourceError::Empty),
        n if n > MAX_SOURCE_PREFIXES => Err(SourceError::TooMany),
        _ => Ok(prefixes),
    }
}

#[cfg(test)]
mod tests;

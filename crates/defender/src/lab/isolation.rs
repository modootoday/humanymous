use super::LabError;
use serde::{Deserialize, Serialize};
use std::io::{self, ErrorKind};

/// How the IPv6 half of the isolation check was satisfied. Recorded in every
/// process receipt so an absent IPv6 stack is evidence, never a silent pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ipv6Evidence {
    /// The IPv6 route table was read and lists only `lo` routes.
    LoopbackRoutesOnly,
    /// The route table, the address table and the IPv6 sysctl tree were all
    /// observed not found: the kernel exposes no IPv6 stack, so no IPv6 route.
    StackAbsent,
}

#[cfg(any(target_os = "linux", test))]
fn interfaces(text: &str) -> Result<(), LabError> {
    let names: Vec<_> = text
        .lines()
        .filter_map(|line| line.split_once(':').map(|(name, _)| name.trim()))
        .collect();
    if names == ["lo"] {
        Ok(())
    } else {
        Err(LabError("namespace requires only loopback"))
    }
}
#[cfg(any(target_os = "linux", test))]
fn routes(v4: &str) -> Result<(), LabError> {
    let mut ipv4 = v4.lines();
    let header: Vec<_> = ipv4
        .next()
        .ok_or(LabError("IPv4 route evidence missing"))?
        .split_whitespace()
        .collect();
    if header
        != [
            "Iface",
            "Destination",
            "Gateway",
            "Flags",
            "RefCnt",
            "Use",
            "Metric",
            "Mask",
            "MTU",
            "Window",
            "IRTT",
        ]
    {
        return Err(LabError("IPv4 route evidence malformed"));
    }
    if ipv4.any(|line| !line.trim().is_empty()) {
        return Err(LabError("namespace has external routes"));
    }
    Ok(())
}

/// Judge the IPv6 evidence. A present route table must parse as the kernel's
/// ten-field format with only `lo` routes. Only a not-found route table counts
/// as "no IPv6 stack", and only when the address table and the IPv6 sysctl
/// tree are also not found; any other read error or a mixed observation fails
/// closed.
#[cfg(any(target_os = "linux", test))]
fn ipv6(
    route: io::Result<String>,
    addresses: io::Result<()>,
    sysctl: io::Result<()>,
) -> Result<Ipv6Evidence, LabError> {
    let absent = |result: &io::Result<()>| -> Result<bool, LabError> {
        match result {
            Ok(()) => Ok(false),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(true),
            Err(_) => Err(LabError("IPv6 stack evidence unavailable")),
        }
    };
    let addresses_absent = absent(&addresses)?;
    let sysctl_absent = absent(&sysctl)?;
    match route {
        Ok(text) => {
            for line in text.lines().filter(|line| !line.trim().is_empty()) {
                let fields: Vec<_> = line.split_whitespace().collect();
                if fields.len() != 10 {
                    return Err(LabError("IPv6 route evidence malformed"));
                }
                if fields[9] != "lo" {
                    return Err(LabError("namespace has external routes"));
                }
            }
            Ok(Ipv6Evidence::LoopbackRoutesOnly)
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            if addresses_absent && sysctl_absent {
                Ok(Ipv6Evidence::StackAbsent)
            } else {
                Err(LabError("IPv6 stack evidence inconsistent"))
            }
        }
        Err(_) => Err(LabError("namespace evidence unavailable")),
    }
}

pub(super) fn check() -> Result<Ipv6Evidence, LabError> {
    #[cfg(target_os = "linux")]
    {
        let read = |path| {
            std::fs::read_to_string(path).map_err(|_| LabError("namespace evidence unavailable"))
        };
        let exists = |path| std::fs::symlink_metadata(path).map(|_| ());
        interfaces(&read("/proc/self/net/dev")?)?;
        routes(&read("/proc/self/net/route")?)?;
        ipv6(
            std::fs::read_to_string("/proc/self/net/ipv6_route"),
            exists("/proc/self/net/if_inet6"),
            exists("/proc/sys/net/ipv6"),
        )
    }
    #[cfg(not(target_os = "linux"))]
    Err(LabError(
        "owned network pilot requires isolated Linux Docker execution",
    ))
}

pub(super) fn child_namespace(pid: u32) -> Result<(), LabError> {
    #[cfg(target_os = "linux")]
    {
        let own = std::fs::read_link("/proc/self/ns/net").map_err(|_| LabError("own namespace"))?;
        let child = std::fs::read_link(format!("/proc/{pid}/ns/net"))
            .map_err(|_| LabError("child namespace"))?;
        if own != child {
            return Err(LabError("child namespace mismatch"));
        }
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        Err(LabError("unsupported child namespace"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const LO_ROUTE: &str = "00000000000000000000000000000001 80 00000000000000000000000000000000 00 00000000000000000000000000000000 00000000 00000001 00000000 80200001 lo\n";
    fn missing() -> io::Error {
        io::Error::from(ErrorKind::NotFound)
    }
    fn denied() -> io::Error {
        io::Error::from(ErrorKind::PermissionDenied)
    }
    #[test]
    fn malformed_missing_and_extra_interfaces_fail_closed() {
        assert!(interfaces("Inter|x\n face|x\n lo: 0").is_ok());
        for text in ["", "face|x", "lo: 0\neth0: 0", "lo: 0\nlo: 0"] {
            assert!(interfaces(text).is_err());
        }
    }
    #[test]
    fn external_routes_are_not_isolation() {
        let header = "Iface Destination Gateway Flags RefCnt Use Metric Mask MTU Window IRTT\n";
        assert!(routes(header).is_ok());
        assert!(routes(&format!("{header}eth0 00000000 00000000")).is_err());
        for missing in [
            "",
            "\n",
            "Iface\n",
            "Iface Destination Gateway\n",
            "unknown\n",
        ] {
            assert!(routes(missing).is_err());
        }
    }
    #[test]
    fn loopback_only_ipv6_routes_are_recorded_as_read() {
        assert_eq!(
            ipv6(Ok(LO_ROUTE.into()), Ok(()), Ok(())),
            Ok(Ipv6Evidence::LoopbackRoutesOnly)
        );
        assert_eq!(
            ipv6(Ok(String::new()), Ok(()), Ok(())),
            Ok(Ipv6Evidence::LoopbackRoutesOnly)
        );
    }
    #[test]
    fn absent_ipv6_stack_is_accepted_only_as_recorded_evidence() {
        let evidence = ipv6(Err(missing()), Err(missing()), Err(missing())).unwrap();
        assert_eq!(evidence, Ipv6Evidence::StackAbsent);
        assert_eq!(
            serde_json::to_value(evidence).unwrap(),
            serde_json::json!("stack_absent")
        );
    }
    #[test]
    fn ipv6_read_errors_and_external_routes_fail_closed() {
        let external = LO_ROUTE.replace(" lo\n", " eth0\n");
        let refused = [
            // A route table that is present but unreadable or not text.
            ipv6(Err(denied()), Err(missing()), Err(missing())),
            ipv6(
                Err(io::Error::from(ErrorKind::InvalidData)),
                Err(missing()),
                Err(missing()),
            ),
            ipv6(Err(io::Error::other("io")), Err(missing()), Err(missing())),
            // A present route table with a non-loopback route.
            ipv6(Ok(external.clone()), Ok(()), Ok(())),
            ipv6(Ok(external), Err(missing()), Err(missing())),
            // Truncated or otherwise malformed route lines.
            ipv6(Ok("0000 0000 lo\n".into()), Ok(()), Ok(())),
            ipv6(Ok(format!("{LO_ROUTE} extra")), Ok(()), Ok(())),
            // Route table absent, but the stack is otherwise visible.
            ipv6(Err(missing()), Ok(()), Err(missing())),
            ipv6(Err(missing()), Err(missing()), Ok(())),
            // Route table absent, companion evidence unreadable.
            ipv6(Err(missing()), Err(denied()), Err(missing())),
            ipv6(Err(missing()), Err(missing()), Err(denied())),
            // Route table present, companion evidence unreadable.
            ipv6(Ok(LO_ROUTE.into()), Err(denied()), Ok(())),
        ];
        for (index, result) in refused.into_iter().enumerate() {
            assert!(result.is_err(), "case {index} must fail closed");
        }
    }
    #[test]
    #[cfg(not(target_os = "linux"))]
    fn host_cannot_enable_network_mode() {
        assert!(check().is_err());
    }
}

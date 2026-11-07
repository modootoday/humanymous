//! Read-only Linux prerequisite inventory, not native backend qualification.
//! Fixed local kernel paths, bounded byte reads, no launch/kill/unit creation.
//! File reads have byte bounds, not a claimed cancellation or wall-clock bound.

/// Observation of a prerequisite; unknown never means available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    /// The narrow read-only observation succeeded.
    Observed,
    /// The required object was absent or reported a different configuration.
    Absent,
    /// The observation could not establish the prerequisite.
    Unknown,
}

/// Safe fixed vocabulary; no private filesystem content is returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedReason {
    /// This build is not Linux.
    NonLinux,
    /// PID1 has not been observed as systemd.
    SystemdPid1Unconfirmed,
    /// A unified cgroup-v2 mount has not been observed.
    UnifiedCgroupUnconfirmed,
    /// The conventional system bus socket has not been observed.
    SystemBusUnconfirmed,
    /// Actual manager version negotiation is not implemented.
    ManagerVersionNotQueried,
    /// Actual noninteractive broker authorization has not been established.
    BrokerAuthorizationNotQueried,
    /// Native allocation, invocation retention and cleanup are not qualified.
    NativeBackendUnqualified,
}

/// Prerequisite inventory only. Even all observed paths cannot authorize spawn.
#[derive(Debug)]
pub struct Prerequisites {
    /// Actual fixed-path PID1 comm observation.
    pub systemd_pid1: Availability,
    /// Actual mountinfo cgroup-v2 observation.
    pub unified_cgroup: Availability,
    /// Actual fixed-path Unix socket observation; no authenticated bus handshake.
    pub system_bus_socket: Availability,
    /// Explicit reasons this inventory cannot select a supported backend.
    pub unsupported: Vec<UnsupportedReason>,
}

/// Inspect fixed local prerequisites without creating or changing any resource.
/// No system bus call, policy prompt, subprocess or authorization trial occurs.
pub fn inspect() -> Prerequisites {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::FileTypeExt;
        let systemd_pid1 = bounded_read("/proc/1/comm", 64)
            .map(|bytes| {
                if bytes == b"systemd\n" {
                    Availability::Observed
                } else {
                    Availability::Absent
                }
            })
            .unwrap_or(Availability::Unknown);
        let unified_cgroup = bounded_read("/proc/self/mountinfo", 262_144)
            .map(|bytes| {
                if bytes.split(|byte| *byte == b'\n').any(|line| {
                    line.windows(b" - cgroup2 ".len())
                        .any(|window| window == b" - cgroup2 ")
                }) {
                    Availability::Observed
                } else {
                    Availability::Absent
                }
            })
            .unwrap_or(Availability::Unknown);
        let system_bus_socket = match std::fs::symlink_metadata("/run/dbus/system_bus_socket") {
            Ok(metadata) if metadata.file_type().is_socket() => Availability::Observed,
            Ok(_) => Availability::Absent,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Availability::Absent,
            Err(_) => Availability::Unknown,
        };
        inventory(systemd_pid1, unified_cgroup, system_bus_socket)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let mut result = inventory(
            Availability::Unknown,
            Availability::Unknown,
            Availability::Unknown,
        );
        result.unsupported.insert(0, UnsupportedReason::NonLinux);
        result
    }
}

#[cfg(target_os = "linux")]
fn bounded_read(path: &str, limit: usize) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(std::io::Error::other(
            "prerequisite read exceeded byte bound",
        ));
    }
    Ok(bytes)
}

fn inventory(pid1: Availability, cgroup: Availability, bus: Availability) -> Prerequisites {
    let mut unsupported = Vec::new();
    for (observed, reason) in [
        (pid1, UnsupportedReason::SystemdPid1Unconfirmed),
        (cgroup, UnsupportedReason::UnifiedCgroupUnconfirmed),
        (bus, UnsupportedReason::SystemBusUnconfirmed),
    ] {
        if observed != Availability::Observed {
            unsupported.push(reason);
        }
    }
    unsupported.extend([
        UnsupportedReason::ManagerVersionNotQueried,
        UnsupportedReason::BrokerAuthorizationNotQueried,
        UnsupportedReason::NativeBackendUnqualified,
    ]);
    Prerequisites {
        systemd_pid1: pid1,
        unified_cgroup: cgroup,
        system_bus_socket: bus,
        unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observed_paths_never_qualify_manager_or_authorization() {
        let result = inventory(
            Availability::Observed,
            Availability::Observed,
            Availability::Observed,
        );
        assert_eq!(result.unsupported.len(), 3);
        assert!(
            result
                .unsupported
                .contains(&UnsupportedReason::NativeBackendUnqualified)
        );
    }

    #[test]
    fn unknown_and_absent_are_both_explicitly_unsupported() {
        for observation in [Availability::Unknown, Availability::Absent] {
            let result = inventory(observation, observation, observation);
            assert_eq!(result.unsupported.len(), 6);
        }
    }
}

use super::LabError;

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
fn routes(v4: &str, v6: &str) -> Result<(), LabError> {
    if v4.lines().skip(1).any(|line| !line.trim().is_empty())
        || v6.lines().any(|line| {
            line.split_whitespace()
                .last()
                .is_some_and(|device| device != "lo")
        })
    {
        return Err(LabError("namespace has external routes"));
    }
    Ok(())
}

pub(super) fn check() -> Result<(), LabError> {
    #[cfg(target_os = "linux")]
    {
        let read = |path| {
            std::fs::read_to_string(path).map_err(|_| LabError("namespace evidence unavailable"))
        };
        interfaces(&read("/proc/self/net/dev")?)?;
        routes(
            &read("/proc/self/net/route")?,
            &read("/proc/self/net/ipv6_route")?,
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
    #[test]
    fn malformed_missing_and_extra_interfaces_fail_closed() {
        assert!(interfaces("Inter|x\n face|x\n lo: 0").is_ok());
        for text in ["", "face|x", "lo: 0\neth0: 0", "lo: 0\nlo: 0"] {
            assert!(interfaces(text).is_err());
        }
    }
    #[test]
    fn external_routes_are_not_isolation() {
        assert!(routes("Iface Destination Gateway\n", "0000 0000 lo\n").is_ok());
        assert!(routes("Iface Destination Gateway\neth0 00000000 00000000", "").is_err());
        assert!(routes("Iface Destination Gateway\n", "0000 0000 eth0\n").is_err());
    }
    #[test]
    #[cfg(not(target_os = "linux"))]
    fn host_cannot_enable_network_mode() {
        assert!(check().is_err());
    }
}

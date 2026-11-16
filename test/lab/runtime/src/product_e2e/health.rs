//! Shell-free container health probes, run by the engine through exec form.
//! A probe only observes liveness of its own role; it never sends a request
//! that could issue a session, append audit or reach the origin.
use std::{net::TcpStream, path::Path, time::Duration};

fn reachable(address: &str) -> bool {
    TcpStream::connect_timeout(&super::address(address), Duration::from_secs(2)).is_ok()
}

/// Pure judgement for one role; unknown roles are never healthy.
pub fn healthy(role: Option<&str>) -> bool {
    match role {
        Some("gate") => reachable(super::GATE),
        // Never the application port: every application contact is an origin record.
        Some("origin") => reachable(super::OBSERVE),
        Some("witness") => Path::new(super::witness::READY_MARKER).is_file(),
        _ => false,
    }
}

/// Exit 0 when healthy, 1 otherwise; no panic path reaches the engine.
pub fn run(role: Option<&str>) -> ! {
    std::process::exit(if healthy(role) { 0 } else { 1 })
}

#[cfg(test)]
mod tests {
    #[test]
    fn unknown_roles_are_unhealthy() {
        assert!(!super::healthy(None));
        assert!(!super::healthy(Some("authority")));
    }
}

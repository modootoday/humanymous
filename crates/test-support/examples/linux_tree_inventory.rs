//! Fixed read-only inventory; a successful invocation never qualifies a backend.
use humanymous_test_support::linux_tree_probe::{Availability, UnsupportedReason, inspect};
use serde_json::json;

fn availability(value: Availability) -> &'static str {
    match value {
        Availability::Observed => "observed",
        Availability::Absent => "absent",
        Availability::Unknown => "unknown",
    }
}

fn reason(value: UnsupportedReason) -> &'static str {
    match value {
        UnsupportedReason::NonLinux => "non_linux",
        UnsupportedReason::SystemdPid1Unconfirmed => "systemd_pid1_unconfirmed",
        UnsupportedReason::UnifiedCgroupUnconfirmed => "unified_cgroup_unconfirmed",
        UnsupportedReason::SystemBusUnconfirmed => "system_bus_unconfirmed",
        UnsupportedReason::ManagerVersionNotQueried => "manager_version_not_queried",
        UnsupportedReason::BrokerAuthorizationNotQueried => "broker_authorization_not_queried",
        UnsupportedReason::NativeBackendUnqualified => "native_backend_unqualified",
    }
}

fn main() {
    if std::env::args_os().len() != 1 {
        eprintln!("inventory accepts no arguments");
        std::process::exit(64);
    }
    let facts = inspect();
    let output = json!({
        "schema": "humanymous.linux-tree-prerequisites/1",
        "status": "unsupported",
        "linux_build": cfg!(target_os = "linux"),
        "systemd_pid1": availability(facts.systemd_pid1),
        "cgroup_v2_mount": availability(facts.unified_cgroup),
        "system_bus_socket": availability(facts.system_bus_socket),
        "manager_version": "not_queried",
        "broker_authorization": "not_queried",
        "native_backend_qualified": false,
        "unsupported_reasons": facts.unsupported.into_iter().map(reason).collect::<Vec<_>>(),
    });
    // All fields are closed literals/enums; no raw path, PID, version or bus data.
    println!("{output}");
}

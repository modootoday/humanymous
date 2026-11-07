//! Owned per-namespace DNS/TCP egress prerequisite, never public DNS probing.
use super::Result;
use serde_json::json;
use std::{
    net::{Ipv4Addr, TcpStream},
    time::Duration,
};
pub fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(2).collect();
    if args.len() != 2 {
        return Err("owned outside canary and DNS nonce required".into());
    }
    let outside: Ipv4Addr = args[0].parse()?;
    if !outside.is_private() {
        return Err("owned outside canary private IP required".into());
    }
    let routes = std::fs::read_to_string("/proc/net/route")?;
    if routes
        .lines()
        .skip(1)
        .any(|line| line.split_whitespace().nth(1) == Some("00000000"))
    {
        return Err("Go actor has external default IPv4 route".into());
    }
    let tcp = match TcpStream::connect_timeout(&(outside, 8080).into(), Duration::from_secs(2)) {
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NetworkUnreachable
                    | std::io::ErrorKind::HostUnreachable
                    | std::io::ErrorKind::PermissionDenied
            ) =>
        {
            json!({"kind":format!("{:?}",error.kind()),"os_error":error.raw_os_error()})
        }
        _ => return Err("Go outside TCP route available or denial unverified".into()),
    };
    let resolver = crate::browser_dns::DenyResolver::start()?;
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let dns = executor.block_on(crate::browser_dns::probe(&resolver, &args[1], outside))?;
    let policy = resolver.finish()?;
    println!(
        "{}",
        json!({"schema":"humanymous.go-egress-control/1","no_default_ipv4_route":true,"outside_tcp_denied":tcp,"dns_control":dns,"dns_policy":policy,"timeout_counted_as_denial":false,"full_product_qualified":false})
    );
    Ok(())
}

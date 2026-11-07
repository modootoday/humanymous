//! Untouched upstream binary with shared bounded child ownership and byte relay.
use super::{Result, relay};
use humanymous_platform::testing::process::ManagedChild;
use serde_json::json;
use std::{
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::Path,
    process::Command,
    time::{Duration, Instant},
};
use tokio::{net::TcpListener, sync::watch};

pub fn run() -> Result<()> {
    let contract = humanymous_platform::testing::json::decode_strict_json(
        include_bytes!("../../../../../adapters/go/pilot-contract.json"),
        65536,
    )?;
    let mut command = Command::new("/usr/local/bin/go-httpbin");
    for arg in contract["fixture"]["argv"]
        .as_array()
        .ok_or("Go fixture argv absent")?
    {
        command.arg(arg.as_str().ok_or("Go fixture argv type")?);
    }
    let mut child = ManagedChild::spawn(&mut command, &[])?;
    let peer = SocketAddr::from((Ipv4Addr::LOCALHOST, 18081));
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait()?.is_some() {
            return Err("actual Go backend exited before readiness".into());
        }
        if TcpStream::connect_timeout(&peer, Duration::from_millis(100)).is_ok() {
            break;
        }
        if Instant::now() >= until {
            return Err("actual Go backend readiness deadline".into());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    executor.block_on(async move{
        let listener=TcpListener::bind((Ipv4Addr::UNSPECIFIED,18082)).await?;
        let contacts=relay::Contacts::default();let(stop,receiver)=watch::channel(false);
        let relay=tokio::spawn(relay::serve(listener,peer,contacts.clone(),receiver));
        println!("{}",json!({"schema":"humanymous.go-backend-ready/1","upstream_loopback":peer,"owned_relay":"0.0.0.0:18082","host_profile":"untouched-upstream-go-httpbin"}));
        let until=tokio::time::Instant::now()+Duration::from_secs(300);
        while !Path::new("/tmp/go-backend-stop").exists(){
            if child.try_wait()?.is_some(){return Err("actual Go backend exited during cases".into());}
            if tokio::time::Instant::now()>=until{return Err("Go backend supervisor deadline".into());}
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        stop.send(true).map_err(|_|"Go backend relay controller closed")?;relay.await??;
        let status=child.stop()?;
        println!("{}",json!({"schema":"humanymous.go-backend-stop/1","child_reaped":true,"child_exit_code":status.code(),"graceful_shutdown_qualified":false,"contacts":contacts.snapshot()}));
        Ok(())
    })
}
pub fn stop() -> Result<()> {
    std::fs::write("/tmp/go-backend-stop", b"stop")?;
    Ok(())
}

//! Lab stand-in for the manifest's adopter-owned edge. The runtime service
//! uses `network_mode: service:edge`, so it shares this container's network
//! namespace and the relays below reach it over loopback.
use super::{EDGE_PORT, ORIGIN_LOOPBACK, ORIGIN_UPSTREAM, RUNTIME_LISTEN, emit};
use crate::origin_audit::relay;
use serde_json::json;
use std::{
    io,
    net::{Ipv4Addr, TcpListener},
    time::Duration,
};

pub fn run() -> io::Result<()> {
    // Bind both relays before reporting, so the runtime never starts first.
    relay::spawn(TcpListener::bind(ORIGIN_LOOPBACK)?, ORIGIN_UPSTREAM.into());
    relay::spawn(
        TcpListener::bind((Ipv4Addr::UNSPECIFIED, EDGE_PORT))?,
        RUNTIME_LISTEN.into(),
    );
    emit(
        &json!({"role":"dep_edge_ready","ingress":format!("0.0.0.0:{EDGE_PORT}"),
        "runtime":RUNTIME_LISTEN,"origin_loopback":ORIGIN_LOOPBACK,"origin":ORIGIN_UPSTREAM,
        "tls_terminated":false}),
    )?;
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

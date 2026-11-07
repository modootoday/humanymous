//! Bounded real client transport. Only the independent host fills proof fields.
use super::{Result, response};
use humanymous::adapters::go::{
    Case,
    scenario::{BINARY_ENTITY, BINARY_TARGET, HEAD_TARGET},
};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    time::Duration,
};

fn exchange(address: SocketAddr, bytes: &[u8], half_close: bool) -> Result<Vec<u8>> {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
    stream.set_read_timeout(Some(Duration::from_secs(8)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(bytes)?;
    if half_close {
        stream.shutdown(Shutdown::Write)?;
    }
    let mut output = Vec::new();
    stream.take(147457).read_to_end(&mut output)?;
    if output.len() > 147456 {
        return Err("Go response byte bound".into());
    }
    Ok(output)
}
pub fn run() -> Result<()> {
    let until = std::time::Instant::now() + Duration::from_secs(60);
    while !std::path::Path::new("/tmp/go-client-release").exists() {
        if std::time::Instant::now() >= until {
            return Err("Go independent namespace inspection release absent".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let args: Vec<_> = std::env::args().skip(2).collect();
    if args.len() != 2 && args.len() != 4 {
        return Err("owned Go gate tuple and case required".into());
    }
    let address: SocketAddr = args[0].parse()?;
    if !matches!(address.ip(),std::net::IpAddr::V4(ip)if ip.is_private()) || address.port() != 8080
    {
        return Err("owned Go ingress tuple required".into());
    }
    let case: Case = serde_json::from_value(json!(args[1]))?;
    if case == Case::OriginBypass {
        if args.len() != 4 {
            return Err("independent backend and contact witness tuples required".into());
        }
        let backend: SocketAddr = args[2].parse()?;
        let witness: SocketAddr = args[3].parse()?;
        if !matches!(backend.ip(),std::net::IpAddr::V4(ip)if ip.is_private())
            || backend.port() != 18082
            || witness.ip() != address.ip()
            || witness.port() == 8080
            || witness.port() == 0
        {
            return Err("owned Go bypass tuples required".into());
        }
        for peer in [backend, witness] {
            match TcpStream::connect_timeout(&peer, Duration::from_secs(2)) {
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionRefused
                            | std::io::ErrorKind::NetworkUnreachable
                            | std::io::ErrorKind::HostUnreachable
                            | std::io::ErrorKind::PermissionDenied
                    ) => {}
                _ => return Err("Go bypass kernel denial absent or timed out".into()),
            }
        }
        println!(
            "{}",
            json!({"schema":"humanymous.go-client-observation/1","observation":{"case":case,"http_status":null,"decoded_entity":[],"header_pairs":[],"origin_contacts":null,"actual_host_verified":false,"pre_handoff_cancelled":false,"observed_past_deadline":false,"write_half_closed":false,"backend_and_witness_kernel_denied":true},"raw_response":[],"full_product_qualified":false})
        );
        return Ok(());
    }
    if args.len() != 2 {
        return Err("unexpected Go case control arguments".into());
    }
    let head = case == Case::ResponseSemantics;
    let target = match case {
        Case::BinaryAllow => BINARY_TARGET,
        Case::DependencyDown => "/anything",
        Case::ResponseSemantics => HEAD_TARGET,
        _ => "/dump/request",
    };
    let method = if head {
        "HEAD"
    } else if case == Case::Upgrade {
        "GET"
    } else {
        "POST"
    };
    let body = if case == Case::BinaryAllow {
        BINARY_ENTITY.to_vec()
    } else {
        vec![]
    };
    let mut request = crate::client::request(target, body);
    request.method = method.into();
    if case == Case::BinaryAllow || head {
        // The existing native fixture is explicitly synthetic, not a browser.
        let installation = crate::installation::Installation::new(false)?;
        let typed = super::configuration(installation.config.clone(), false)?;
        let bytes = exchange(
            address,
            &crate::client::wire(&crate::client::request("/_humanymous/session", vec![])),
            false,
        )?;
        let observed = response::observe(&bytes, false)?;
        if observed.status != 200 {
            return Err("Go native session bootstrap rejected".into());
        }
        let issued = humanymous_platform::testing::json::decode_strict_json(observed.entity, 65536)?;
        let mut collect =
            crate::client::request("/_humanymous/collect", crate::client::report(&issued));
        crate::client::sign(&typed, &mut collect, &issued, 1);
        let bytes = exchange(address, &crate::client::wire(&collect), false)?;
        if response::observe(&bytes, false)?.status != 200 {
            return Err("Go synthetic report rejected".into());
        }
        if case == Case::BinaryAllow {
            for (name, value) in &mut request.headers {
                if name.eq_ignore_ascii_case("content-type") {
                    *value = "application/octet-stream".into();
                }
            }
        }
        crate::client::sign(&typed, &mut request, &issued, 2);
    }
    if case == Case::ForgedProvenance {
        request.headers.extend([
            ("x-humanymous-capture".into(), "forged".into()),
            ("x-forwarded-for".into(), "127.0.0.1".into()),
        ]);
    }
    if case == Case::Upgrade {
        request.headers.extend([
            ("Connection".into(), "Upgrade".into()),
            ("Upgrade".into(), "websocket".into()),
        ]);
    }
    let partial = matches!(case, Case::Cancellation | Case::IncompleteBody);
    if partial {
        for (name, value) in &mut request.headers {
            if name.eq_ignore_ascii_case("content-length") {
                *value = "7".into();
            }
        }
        request.body = vec![0];
    }
    let bytes = crate::client::wire(&request);
    let cancelled = case == Case::Cancellation;
    let raw = if cancelled {
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
        stream.set_write_timeout(Some(Duration::from_secs(3)))?;
        stream.write_all(&bytes)?;
        stream.shutdown(Shutdown::Both)?;
        drop(stream);
        // Runtime body timeout is five seconds. Counter observation is separate.
        std::thread::sleep(Duration::from_millis(5500));
        vec![]
    } else {
        exchange(address, &bytes, case == Case::IncompleteBody)?
    };
    let (status, entity, headers) = if cancelled {
        (None, vec![], vec![])
    } else {
        let observed = response::observe(&raw, head)?;
        (
            Some(observed.status),
            observed.entity.to_vec(),
            observed
                .headers
                .into_iter()
                .map(|(n, v)| (n.to_owned(), v.to_owned()))
                .collect::<Vec<_>>(),
        )
    };
    println!(
        "{}",
        json!({"schema":"humanymous.go-client-observation/1","observation":{"case":case,"http_status":status,"decoded_entity":entity,"header_pairs":headers,"origin_contacts":null,"actual_host_verified":false,"pre_handoff_cancelled":cancelled,"observed_past_deadline":cancelled,"write_half_closed":case==Case::IncompleteBody,"backend_and_witness_kernel_denied":false},"raw_response":raw,"full_product_qualified":false})
    );
    Ok(())
}
pub fn release() -> Result<()> {
    std::fs::write("/tmp/go-client-release", b"release")?;
    Ok(())
}

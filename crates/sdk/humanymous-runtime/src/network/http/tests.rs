//! Ports of cmd/server/headerinfo_test.go plus differential checks against
//! tests/fixtures/network/headerinfo-go-golden.json (Go oracle 981d25c).
use super::*;
use crate::network::proxy::Cidr;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{IpAddr, Shutdown, TcpListener, TcpStream};
use std::time::{Duration, Instant};

fn golden() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/network/headerinfo-go-golden.json"
    ))
    .expect("golden json")
}

fn fields(value: &Value) -> Vec<(String, String)> {
    value
        .as_array()
        .expect("fields")
        .iter()
        .map(|pair| {
            (
                pair[0].as_str().expect("name").to_owned(),
                pair[1].as_str().expect("value").to_owned(),
            )
        })
        .collect()
}

type Fields<'a> = &'a [(&'a str, &'a str)];

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

fn dataset(cidrs: &[&str]) -> impl Fn(IpAddr) -> bool {
    let nets: Vec<Cidr> = cidrs
        .iter()
        .map(|cidr| Cidr::parse(cidr).expect("cidr"))
        .collect();
    move |ip| nets.iter().any(|net| net.contains(ip))
}

// TestIsDatacenterIP_FailsOpenByDefault
#[test]
fn datacenter_lookup_fails_open_without_dataset() {
    for ip in [
        "127.0.0.1",
        "::1",
        "10.1.2.3",
        "172.30.0.1",
        "169.254.1.1",
        "garbage",
        "",
        "8.8.8.8",
        "1.2.3.4",
        "203.0.113.7",
    ] {
        assert!(!is_datacenter_ip(ip, |_| false), "{ip}");
    }
}

// TestIsDatacenterIP_WithDataset
#[test]
fn datacenter_lookup_flags_only_wired_ranges() {
    let wired = dataset(&["203.0.113.0/24"]);
    assert!(is_datacenter_ip("203.0.113.7", &wired));
    for ip in ["8.8.8.8", "198.51.100.4", "10.0.0.1", ""] {
        assert!(!is_datacenter_ip(ip, &wired), "{ip}");
    }
    // A dataset that claims everything still never accuses a local peer.
    assert!(!is_datacenter_ip("10.0.0.1", |_| true));
}

// TestIsTrustedProxy
#[test]
fn trusted_proxy_is_loopback_private_or_link_local() {
    for ip in [
        "127.0.0.1",
        "::1",
        "10.0.0.1",
        "192.168.1.1",
        "172.30.0.1",
        "169.254.0.1",
        "fc00::5",
    ] {
        assert!(is_trusted_proxy(ip), "{ip}");
    }
    for ip in ["8.8.8.8", "1.2.3.4", "172.32.0.1", "garbage", ""] {
        assert!(!is_trusted_proxy(ip), "{ip}");
    }
}

// TestClientIP
#[test]
fn client_ip_honours_forwarding_only_from_trusted_peer() {
    let cases: [(&str, Fields<'_>, &str); 7] = [
        ("8.8.8.8:44210", &[], "8.8.8.8"),
        ("8.8.8.8:1000", &[("X-Forwarded-For", "1.2.3.4")], "8.8.8.8"),
        (
            "172.30.0.1:8443",
            &[("X-Forwarded-For", "203.0.113.9, 172.30.0.1")],
            "203.0.113.9",
        ),
        (
            "172.18.0.1:8443",
            &[("X-Real-IP", "203.0.113.7")],
            "203.0.113.7",
        ),
        ("172.30.0.1:8443", &[], "172.30.0.1"),
        (
            "10.0.0.1:80",
            &[
                ("X-Forwarded-For", "203.0.113.1"),
                ("X-Real-IP", "203.0.113.2"),
            ],
            "203.0.113.1",
        ),
        ("[::1]:5000", &[("X-Real-IP", "203.0.113.5")], "203.0.113.5"),
    ];
    for (remote, headers, want) in cases {
        assert_eq!(client_ip(remote, &pairs(headers)), want, "{remote}");
    }
}

// TestHeaderInfoOrderIsNotReliable
#[test]
fn projection_reports_sorted_set_not_wire_order() {
    let header = project_request(
        "GET",
        1,
        1,
        &pairs(&[
            ("User-Agent", "Mozilla/5.0 Chrome/126"),
            ("Accept", "*/*"),
            ("Sec-Fetch-Site", "same-origin"),
        ]),
    );
    assert!(!header.order_reliable && !header.casing_reliable);
    assert!(header.names.windows(2).all(|pair| pair[0] <= pair[1]));
    assert_eq!(header.names, ["Accept", "Sec-Fetch-Site", "User-Agent"]);
}

#[test]
fn client_address_model_matches_go_golden() {
    let golden = golden();
    let cidrs: Vec<&str> = golden["datacenter_cidrs"]
        .as_array()
        .expect("cidrs")
        .iter()
        .map(|cidr| cidr.as_str().expect("cidr"))
        .collect();
    let wired = dataset(&cidrs);
    let addresses = golden["addresses"].as_array().expect("addresses");
    for case in addresses {
        let addr = case["addr"].as_str().expect("addr");
        assert_eq!(is_trusted_proxy(addr), case["trusted_proxy"], "{addr:?}");
        assert_eq!(
            is_datacenter_ip(addr, |_| false),
            case["datacenter_without_dataset"],
            "{addr:?}"
        );
        assert_eq!(
            is_datacenter_ip(addr, &wired),
            case["datacenter_with_dataset"],
            "{addr:?}"
        );
        assert_eq!(direct_peer(addr), case["direct_peer"], "{addr:?}");
        let forwarded = pairs(&[("X-Forwarded-For", addr)]);
        assert_eq!(
            client_subnet("10.0.0.1:1", &forwarded),
            case["client_subnet_of_trusted_xff"],
            "{addr:?}"
        );
    }
    let remotes = golden["remote_addrs"].as_array().expect("remotes");
    for case in remotes {
        let remote = case["remote"].as_str().expect("remote");
        assert_eq!(direct_peer(remote), case["direct_peer"], "{remote:?}");
    }
    let clients = golden["clients"].as_array().expect("clients");
    for case in clients {
        let remote = case["remote"].as_str().expect("remote");
        let headers = fields(&case["fields"]);
        assert_eq!(client_ip(remote, &headers), case["client_ip"], "{case}");
        assert_eq!(forwarded_ip(&headers), case["forwarded_ip"], "{case}");
        assert_eq!(
            client_subnet(remote, &headers),
            case["client_subnet"],
            "{case}"
        );
    }
    assert_eq!(
        (addresses.len(), remotes.len(), clients.len()),
        (59, 23, 29)
    );
}

#[test]
fn request_projection_matches_go_golden() {
    let golden = golden();
    let projections = golden["projections"].as_array().expect("projections");
    for case in projections {
        let proto =
            |at: usize| u8::try_from(case["proto"][at].as_u64().expect("proto")).expect("u8");
        let header = project_request(
            case["method"].as_str().expect("method"),
            proto(0),
            proto(1),
            &fields(&case["fields"]),
        );
        let actual = serde_json::to_value(&header).expect("serialize");
        assert_eq!(actual, case["header"], "{}", case["fields"]);
    }
    assert_eq!(projections.len(), 21);
}

fn peek_input(case: &Value) -> Vec<u8> {
    let part = |key: &str| hex::decode(case[key].as_str().unwrap_or_default()).expect("hex");
    let mut bytes = part("hex");
    if let Some(repeat) = case.get("repeat") {
        let unit = hex::decode(repeat["hex"].as_str().expect("unit")).expect("hex");
        for _ in 0..repeat["count"].as_u64().expect("count") {
            bytes.extend_from_slice(&unit);
        }
    }
    bytes.extend(part("suffix_hex"));
    bytes
}

fn error_label(outcome: Result<(), PeekError>) -> &'static str {
    match outcome {
        Ok(()) => "",
        Err(PeekError::Truncated) => "eof",
        Err(PeekError::TimedOut) => "timeout",
        Err(PeekError::Io(_)) => "io",
    }
}

fn socket_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let client = TcpStream::connect(listener.local_addr().expect("addr")).expect("connect");
    let (server, _) = listener.accept().expect("accept");
    (client, server)
}

#[test]
fn h1_peek_matches_go_golden() {
    let golden = golden();
    let peeks = golden["peeks"].as_array().expect("peeks");
    for case in peeks {
        let name = case["name"].as_str().expect("name");
        let input = peek_input(case);
        let hold_open = case["hold_open"].as_bool().expect("hold_open");
        let expected_names: Vec<String> =
            serde_json::from_value(case["names"].clone()).expect("names");
        assert_eq!(case["replay_is_input"], true, "{name}");

        if !hold_open {
            let mut reader = input.as_slice();
            let peek = peek_header_order(&mut reader);
            assert_eq!(peek.names(), expected_names, "{name} reader");
            assert_eq!(error_label(peek.outcome()), case["error"], "{name} reader");
            assert!(
                peek.wire_order().is_none(),
                "{name} reader cannot mint order"
            );
            let mut replay = Vec::new();
            peek.into_replay(reader)
                .read_to_end(&mut replay)
                .expect("replay");
            assert_eq!(replay, input, "{name} reader replay");
        }

        let (mut client, server) = socket_pair();
        let writer = {
            let input = input.clone();
            std::thread::spawn(move || {
                client.write_all(&input).expect("write");
                if !hold_open {
                    client.shutdown(Shutdown::Write).expect("shutdown");
                }
                client
            })
        };
        let peek = peek_tcp(&server);
        let client = writer.join().expect("writer");
        if hold_open {
            client.shutdown(Shutdown::Write).expect("shutdown");
        }
        assert_eq!(peek.names(), expected_names, "{name} socket");
        assert_eq!(error_label(peek.outcome()), case["error"], "{name} socket");
        assert_eq!(server.read_timeout().expect("timeout"), None, "{name}");
        let mut replay = Vec::new();
        peek.into_replay(&server)
            .read_to_end(&mut replay)
            .expect("replay");
        assert_eq!(replay, input, "{name} replay");
    }
    assert_eq!(peeks.len(), 15);
}

#[test]
fn wire_order_is_minted_only_by_a_complete_socket_peek() {
    let request = b"GET / HTTP/1.1\r\nUser-Agent: UA\r\nAccept: */*\r\nHost: a\r\n\r\n";
    let (mut client, server) = socket_pair();
    client.write_all(request).expect("write");
    let order = peek_tcp(&server)
        .wire_order()
        .expect("complete socket peek");
    assert_eq!(order.names(), ["user-agent", "accept", "host"]);
    let mut header = project_request(
        "GET",
        1,
        1,
        &pairs(&[("User-Agent", "UA"), ("Accept", "*/*"), ("Host", "a")]),
    );
    assert_eq!(header.names, ["Accept", "Host", "User-Agent"]);
    order.apply(&mut header);
    assert!(header.order_reliable && !header.casing_reliable);
    assert_eq!(header.names, ["user-agent", "accept", "host"]);

    // A partial socket peek keeps its names but never claims reliable order.
    let (mut client, server) = socket_pair();
    client
        .write_all(b"GET / HTTP/1.1\r\nUser-Agent: UA\r\n")
        .expect("write");
    client.shutdown(Shutdown::Write).expect("shutdown");
    let peek = peek_tcp(&server);
    assert_eq!(peek.outcome(), Err(PeekError::Truncated));
    assert_eq!(peek.names(), ["user-agent"]);
    assert!(peek.wire_order().is_none());

    // An empty complete header section has no order to report.
    let (mut client, server) = socket_pair();
    client.write_all(b"GET / HTTP/1.1\r\n\r\n").expect("write");
    assert!(peek_tcp(&server).wire_order().is_none());
}

#[test]
fn slow_drip_cannot_extend_the_peek_deadline() {
    let (mut client, server) = socket_pair();
    server
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("timeout");
    let writer = std::thread::spawn(move || {
        client
            .write_all(b"GET / HTTP/1.1\r\nX-Slow: ")
            .expect("write");
        for _ in 0..12 {
            std::thread::sleep(Duration::from_millis(200));
            if client.write_all(b"a").is_err() {
                break;
            }
        }
        client
    });
    let started = Instant::now();
    let peek = peek_tcp(&server);
    let elapsed = started.elapsed();
    assert_eq!(peek.outcome(), Err(PeekError::TimedOut));
    assert!(elapsed >= PEEK_DEADLINE, "{elapsed:?}");
    assert!(elapsed < PEEK_DEADLINE * 3, "{elapsed:?}");
    assert_eq!(
        server.read_timeout().expect("timeout"),
        Some(Duration::from_secs(30))
    );
    drop(writer.join().expect("writer"));
}

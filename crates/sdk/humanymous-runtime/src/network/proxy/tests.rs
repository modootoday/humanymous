//! Ports of legacy internal/gate/proxyproto_test.go (baseline 981d25c) plus
//! fail-closed boundaries for the Rust allowlist and decoder.
use super::*;
use std::io::{Cursor, Write};
use std::net::{TcpListener, TcpStream};

mod golden;

fn v4_header(src: [u8; 4], sport: u16, dst: [u8; 4], dport: u16) -> Vec<u8> {
    let mut b = SIGNATURE.to_vec();
    b.extend_from_slice(&[0x21, 0x11, 0x00, 0x0C]);
    b.extend_from_slice(&src);
    b.extend_from_slice(&dst);
    b.extend_from_slice(&sport.to_be_bytes());
    b.extend_from_slice(&dport.to_be_bytes());
    b
}

#[test]
fn recovers_ipv4_source_and_keeps_payload() {
    let mut bytes = v4_header([203, 0, 113, 9], 44321, [10, 0, 0, 1], 443);
    bytes.extend_from_slice(b"TLS-CLIENTHELLO");
    let mut stream = Cursor::new(bytes);
    let header = read_header(&mut stream).unwrap();
    assert_eq!(header, Header::Source("203.0.113.9:44321".parse().unwrap()));
    let mut rest = Vec::new();
    stream.read_to_end(&mut rest).unwrap();
    assert_eq!(rest, b"TLS-CLIENTHELLO");
}

#[test]
fn recovers_ipv6_source() {
    let mut b = SIGNATURE.to_vec();
    b.extend_from_slice(&[0x21, 0x21, 0x00, 36]);
    let src: Ipv6Addr = "2001:db8::7".parse().unwrap();
    b.extend_from_slice(&src.octets());
    b.extend_from_slice(&Ipv6Addr::LOCALHOST.octets());
    b.extend_from_slice(&8443u16.to_be_bytes());
    b.extend_from_slice(&443u16.to_be_bytes());
    assert_eq!(
        read_header(&mut Cursor::new(b)).unwrap(),
        Header::Source("[2001:db8::7]:8443".parse().unwrap())
    );
}

#[test]
fn local_command_keeps_socket_peer() {
    let mut b = SIGNATURE.to_vec();
    b.extend_from_slice(&[0x20, 0x00, 0x00, 0x00]);
    assert_eq!(read_header(&mut Cursor::new(b)).unwrap(), Header::Local);
}

#[test]
fn unspecified_and_udp_families_keep_socket_peer() {
    for family in [0x00, 0x12] {
        let mut b = SIGNATURE.to_vec();
        b.extend_from_slice(&[0x21, family, 0x00, 0x00]);
        assert_eq!(
            read_header(&mut Cursor::new(b)).unwrap(),
            Header::Unspecified
        );
    }
}

#[test]
fn rejects_garbage_wrong_version_truncation_and_short_blocks() {
    let http = b"GET / HTTP/1.1\r\nHost: x\r\n\r\n".to_vec();
    assert_eq!(
        read_header(&mut Cursor::new(http)),
        Err(Refusal::NotProxyV2)
    );
    let mut v1 = SIGNATURE.to_vec();
    v1.extend_from_slice(&[0x11, 0x11, 0x00, 0x00]);
    assert_eq!(read_header(&mut Cursor::new(v1)), Err(Refusal::NotProxyV2));
    let full = v4_header([1, 2, 3, 4], 1, [5, 6, 7, 8], 2);
    for cut in [0, 11, 15, full.len() - 1] {
        assert_eq!(
            read_header(&mut Cursor::new(full[..cut].to_vec())),
            Err(Refusal::Truncated),
            "{cut}"
        );
    }
    let mut short = SIGNATURE.to_vec();
    short.extend_from_slice(&[0x21, 0x11, 0x00, 0x04, 1, 2, 3, 4]);
    assert_eq!(
        read_header(&mut Cursor::new(short)),
        Err(Refusal::ShortAddress)
    );
}

#[test]
fn trust_gate_over_real_tcp() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let header = v4_header([198, 51, 100, 23], 5555, [10, 0, 0, 1], 443);

    // Trusted peer: the declared source is recovered.
    let trusted = Allowlist::parse("127.0.0.0/8").unwrap();
    let mut client = TcpStream::connect(address).unwrap();
    client.write_all(&header).unwrap();
    client.write_all(b"payload").unwrap();
    let (mut server, peer) = listener.accept().unwrap();
    let resolved = resolve(&trusted, peer, &mut server).unwrap();
    assert_eq!(
        resolved,
        Peer::Declared("198.51.100.23:5555".parse().unwrap())
    );
    let mut rest = [0u8; 7];
    server.read_exact(&mut rest).unwrap();
    assert_eq!(&rest, b"payload");

    // Untrusted peer: nothing is read, the socket peer stands and the header
    // bytes remain ordinary payload (anti-spoof).
    let other = Allowlist::parse("10.0.0.0/8").unwrap();
    let mut client = TcpStream::connect(address).unwrap();
    client.write_all(&header).unwrap();
    let (mut server, peer) = listener.accept().unwrap();
    let resolved = resolve(&other, peer, &mut server).unwrap();
    assert_eq!(resolved, Peer::Socket(peer));
    assert_eq!(resolved.address().ip(), IpAddr::from([127, 0, 0, 1]));
    let mut first = [0u8; 12];
    server.read_exact(&mut first).unwrap();
    assert_eq!(first, SIGNATURE);
}

#[test]
fn allowlist_rejects_broad_and_malformed_entries() {
    for broad in ["0.0.0.0/0", "::/0", "10.0.0.0/4"] {
        assert_eq!(Allowlist::parse(broad), Err(Refusal::TooBroad), "{broad}");
    }
    for bad in [
        "",
        " , ",
        "10.0.0.0/33",
        "::/129",
        "10.0.0/8",
        "10.0.0.0/08",
        "10.0.0.0/",
        "x",
    ] {
        assert!(Allowlist::parse(bad).is_err(), "{bad}");
    }
    let list = Allowlist::parse("172.40.0.0/24, 10.1.2.3").unwrap();
    assert_eq!(list.entries().len(), 2);
    assert!(list.trusts("172.40.0.200".parse().unwrap()));
    assert!(!list.trusts("172.40.1.1".parse().unwrap()));
    assert!(list.trusts("10.1.2.3".parse().unwrap()));
    assert!(!list.trusts("10.1.2.4".parse().unwrap()));
    assert!(list.trusts("::ffff:10.1.2.3".parse().unwrap()));
    let too_many = vec!["10.0.0.1"; MAX_ENTRIES + 1].join(",");
    assert_eq!(Allowlist::parse(&too_many), Err(Refusal::Allowlist));
}

#[test]
fn cidr_masks_host_bits_and_separates_families() {
    let v6 = Cidr::parse("2001:db8::1/32").unwrap();
    assert!(v6.contains("2001:db8:ffff::9".parse().unwrap()));
    assert!(!v6.contains("10.0.0.1".parse().unwrap()));
    let v4 = Cidr::parse("192.0.2.77/24").unwrap();
    assert_eq!(v4, Cidr::parse("192.0.2.0/24").unwrap());
    assert!(!v4.contains("2001:db8::1".parse().unwrap()));
}

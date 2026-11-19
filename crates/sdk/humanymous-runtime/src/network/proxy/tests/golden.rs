//! Differential parity against outputs captured from the frozen Go oracle at
//! 981d25c (see the fixture's `provenance`). Expectations are never
//! regenerated to hide drift. Every captured header decision, remaining-byte
//! position, allowlist parse and trust decision is compared exactly except the
//! deliberate fail-closed differences listed in the module documentation;
//! each of those asserts the captured Go value as well, so the difference
//! stays measured.
use super::*;
use serde_json::Value;
use std::sync::OnceLock;

fn golden() -> &'static Value {
    static GOLDEN: OnceLock<Value> = OnceLock::new();
    GOLDEN.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../tests/fixtures/network/proxyproto-go-golden.json"
        ))
        .expect("golden fixture")
    })
}

fn cases(section: &str) -> &'static [Value] {
    golden()[section].as_array().expect(section)
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

const GO_NOT_V2: &str = "gate: not a PROXY v2 header";

#[test]
fn read_header_matches_go_golden() {
    let rows = cases("read_header");
    assert_eq!(rows.len(), 34);
    for row in rows {
        let name = row["name"].as_str().unwrap();
        let input = unhex(row["input_hex"].as_str().unwrap());
        let mut stream = Cursor::new(input.clone());
        let got = read_header(&mut stream);
        let mut rest = Vec::new();
        stream.read_to_end(&mut rest).unwrap();
        assert_eq!(hex(&rest), row["rest_hex"].as_str().unwrap(), "{name} rest");
        match (&row["addr"], &row["err"]) {
            (Value::String(addr), Value::Null) => match got {
                Ok(Header::Source(source)) => assert_eq!(&source.to_string(), addr, "{name}"),
                other => panic!("{name}: {other:?}, Go declared {addr}"),
            },
            (Value::Null, Value::Null) => {
                let expected = if input[12] & 0x0F == 0 {
                    Header::Local
                } else {
                    Header::Unspecified
                };
                assert_eq!(got, Ok(expected), "{name}");
            }
            (Value::Null, Value::String(err)) => {
                let expected = match err.as_str() {
                    "EOF" | "unexpected EOF" => Refusal::Truncated,
                    GO_NOT_V2 => {
                        // Go used one error for both; Rust says which check failed.
                        let fixed: [u8; FIXED_LEN] = input[..FIXED_LEN].try_into().unwrap();
                        if block_len(&fixed).is_ok() {
                            Refusal::ShortAddress
                        } else {
                            Refusal::NotProxyV2
                        }
                    }
                    other => panic!("{name}: unmapped Go error {other}"),
                };
                assert_eq!(got, Err(expected), "{name}");
            }
            other => panic!("{name}: malformed golden row {other:?}"),
        }
    }
}

#[test]
fn ipv4_mapped_source_is_keyed_as_ipv4_like_go() {
    let row = cases("read_header")
        .iter()
        .find(|row| row["name"] == "v6_mapped_v4")
        .unwrap();
    assert_eq!(row["addr"], "203.0.113.9:8080");
    assert_eq!(row["ip_len"], 16);
    let header = read_header(&mut Cursor::new(unhex(row["input_hex"].as_str().unwrap())));
    assert_eq!(
        header,
        Ok(Header::Source(SocketAddr::from(([203, 0, 113, 9], 8080))))
    );
}

fn expected_net(text: &str) -> (IpAddr, u8) {
    let (address, prefix) = text.split_once('/').unwrap();
    (address.parse().unwrap(), prefix.parse().unwrap())
}

/// Go networks (`None` for none) and the Rust outcome of one difference.
type Difference = (
    Option<&'static [&'static str]>,
    Result<Vec<&'static str>, Refusal>,
);

/// Inputs where the Rust allowlist deliberately differs from Go.
fn cidr_difference(input: &str) -> Option<Difference> {
    Some(match input {
        "" | " " | "," | " , " => (None, Err(Refusal::Allowlist)),
        "10.1.2.3/024" => (Some(&["10.1.2.0/24"]), Err(Refusal::Cidr)),
        "10.0.0.0/08" => (Some(&["10.0.0.0/8"]), Err(Refusal::Cidr)),
        "::ffff:10.0.0.1" => (Some(&["::/32"]), Err(Refusal::Cidr)),
        "::ffff:0:0/96" => (Some(&["0.0.0.0/0"]), Err(Refusal::TooBroad)),
        "::ffff:10.0.0.0/100" => (Some(&["0.0.0.0/4"]), Err(Refusal::TooBroad)),
        _ => return None,
    })
}

fn rust_nets(list: &Allowlist) -> Vec<(IpAddr, u8)> {
    list.entries()
        .iter()
        .map(|cidr| (cidr.network(), cidr.prefix()))
        .collect()
}

#[test]
fn parse_cidrs_matches_go_golden() {
    let rows = cases("parse_cidrs");
    assert_eq!(rows.len(), 43);
    let mut differences = 0;
    for row in rows {
        let input = row["input"].as_str().unwrap();
        let go_nets: Option<Vec<&str>> = row["nets"]
            .as_array()
            .map(|nets| nets.iter().map(|n| n.as_str().unwrap()).collect());
        let got = Allowlist::parse(input);
        if let Some((go_expected, rust_expected)) = cidr_difference(input) {
            differences += 1;
            assert!(row["err"].is_null(), "{input}: Go accepted it");
            assert_eq!(go_nets.as_deref(), go_expected, "{input} Go side");
            match rust_expected {
                Ok(nets) => assert_eq!(
                    rust_nets(&got.unwrap()),
                    nets.into_iter().map(expected_net).collect::<Vec<_>>(),
                    "{input}"
                ),
                Err(refusal) => assert_eq!(got, Err(refusal), "{input}"),
            }
            continue;
        }
        match row["err"].as_str() {
            None => {
                let nets = go_nets.expect("Go networks");
                assert_eq!(
                    rust_nets(&got.unwrap_or_else(|e| panic!("{input}: {e:?}"))),
                    nets.into_iter().map(expected_net).collect::<Vec<_>>(),
                    "{input}"
                );
            }
            Some(err) if err.contains("is too broad") => {
                assert_eq!(got, Err(Refusal::TooBroad), "{input}");
            }
            Some(err) if err.starts_with("invalid CIDR address") => {
                assert_eq!(got, Err(Refusal::Cidr), "{input}");
            }
            Some(err) => panic!("{input}: unmapped Go error {err}"),
        }
    }
    assert_eq!(differences, 9);
}

#[test]
fn trust_gate_matches_go_golden() {
    let rows = cases("trusts");
    assert_eq!(rows.len(), 21);
    for row in rows {
        let list = row["allowlist"].as_str().unwrap();
        // Rust socket addresses carry a numeric scope id, not an interface name.
        let peer: SocketAddr = row["peer"]
            .as_str()
            .unwrap()
            .replace("%eth0", "%2")
            .parse()
            .unwrap();
        let go = row["trusted"].as_bool().unwrap();
        match list {
            // Refused as too broad in Rust; Go trusted every IPv4 peer.
            "::ffff:0:0/96" => {
                assert!(go);
                assert_eq!(Allowlist::parse(list), Err(Refusal::TooBroad));
            }
            // Go parsed this bare entry as `::/32` and did not trust the
            // IPv4 host; Rust refuses the entry, so it never trusts it either.
            "::ffff:10.0.0.1" => {
                assert!(!go, "{list} {peer}");
                assert_eq!(Allowlist::parse(list), Err(Refusal::Cidr));
                assert!(
                    Allowlist::parse("172.40.0.0/24, ::ffff:10.0.0.1").is_err(),
                    "one bare mapped entry refuses the whole list"
                );
            }
            _ => assert_eq!(
                Allowlist::parse(list).unwrap().trusts_peer(peer),
                go,
                "{list} {peer}"
            ),
        }
    }
}

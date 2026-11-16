//! Owned DNS canary for the browser topology (unit 35 increment 4).
//!
//! A real UDP listener on port 53 in its own container on a separate internal
//! lab network. It answers every well-formed query with NXDOMAIN and prints
//! one JSON line per arrival (source address, byte count and the SHA-256 of an
//! owned `.invalid` question, never the name). The runner proves the listener
//! is alive with positive-control probes from a container on the canary's
//! network, then requires that no query from the browser namespace arrived.
use crate::dns_wire;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    net::{Ipv4Addr, UdpSocket},
    time::Duration,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Canary event lines: `listening` once, then one `arrival` per datagram.
pub const SCHEMA: &str = "humanymous.product-e2e-dns-canary/1";
/// One positive-control probe result line.
pub const PROBE_SCHEMA: &str = "humanymous.product-e2e-dns-probe/1";
const PORT: u16 = 53;
/// Arrivals beyond this bound end the canary: the lab sends two on purpose.
const ARRIVAL_BOUND: u64 = 256;
/// NXDOMAIN: the canary never resolves a name.
const NXDOMAIN: u8 = 3;

fn emit(value: &Value) -> Result<()> {
    let mut out = std::io::stdout().lock();
    writeln!(out, "{value}")?;
    out.flush()?;
    Ok(())
}

fn digest(name: &str) -> String {
    format!("{:x}", Sha256::digest(name.as_bytes()))
}

/// One arrival record; pure so the wire judgement is testable.
pub fn arrival(sequence: u64, source: &str, packet: &[u8], answered: bool) -> Value {
    json!({"schema":SCHEMA,"event":"arrival","sequence":sequence,"source":source,
        "bytes":packet.len(),"question_sha256":dns_wire::question(packet).ok().map(|n| digest(&n)),
        "answered":answered})
}

/// Serve until the container is stopped.
pub fn run() -> Result<()> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, PORT))?;
    emit(&json!({"schema":SCHEMA,"event":"listening","port":PORT}))?;
    let mut arrivals = 0u64;
    loop {
        let mut packet = [0; 513];
        let (length, peer) = socket.recv_from(&mut packet)?;
        arrivals = arrivals.checked_add(1).ok_or("canary arrival count")?;
        let received = &packet[..length.min(512)];
        let answered = match dns_wire::reply(received, NXDOMAIN) {
            Ok(reply) => socket.send_to(&reply, peer).is_ok(),
            Err(_) => false,
        };
        emit(&arrival(
            arrivals,
            &peer.ip().to_string(),
            received,
            answered,
        ))?;
        if arrivals >= ARRIVAL_BOUND {
            return Err("canary arrival bound".into());
        }
    }
}

/// Positive control: one owned `.invalid` query to the canary, from a
/// container on the canary's own network. Fails unless the canary answers.
pub fn probe() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let [canary, name] = args.as_slice() else {
        return Err("canary address and owned name required".into());
    };
    let canary: Ipv4Addr = canary.parse()?;
    if !canary.is_private() {
        return Err("owned private canary address required".into());
    }
    let query = dns_wire::query(name).map_err(|e| e.to_string())?;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    socket.connect((canary, PORT))?;
    socket.set_read_timeout(Some(Duration::from_secs(3)))?;
    socket.send(&query)?;
    let mut reply = [0; 513];
    let received = socket.recv(&mut reply);
    let (answered, rcode) = match &received {
        Ok(length) => (
            *length == query.len() && reply[..2] == query[..2] && reply[2] & 0x80 != 0,
            Some(reply[3] & 0x0f),
        ),
        Err(_) => (false, None),
    };
    emit(
        &json!({"schema":PROBE_SCHEMA,"source":socket.local_addr()?.ip().to_string(),
        "question_sha256":digest(name),"answered":answered,"rcode":rcode,
        "receive_error":received.as_ref().err().map(|e| format!("{:?}", e.kind()))}),
    )?;
    if !answered {
        return Err("DNS canary did not answer the positive control".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arrival_records_only_a_question_hash() {
        let packet = dns_wire::query("canary-control.invalid").unwrap();
        let record = arrival(1, "172.30.0.3", &packet, true);
        assert_eq!(record["question_sha256"], digest("canary-control.invalid"));
        assert!(!record.to_string().contains("canary-control"));
        let record = arrival(2, "172.30.0.3", b"junk", false);
        assert!(record["question_sha256"].is_null());
        assert_eq!(record["bytes"], 4);
    }
}

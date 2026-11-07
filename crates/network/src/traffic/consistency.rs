use super::{Limits, Record, subnet_key};
use crate::history::Unavailable;
use humanymous_signals::observation::{Signal, Source, Verdict};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

/// Neither variant asserts healthy serving coverage or authenticated history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    Insufficient,
    UnqualifiedHistory,
}
pub struct Outcome {
    pub signals: Vec<Signal>,
    pub coverage: Coverage,
}

/// Preserve frozen predicates; disabled permutation/order/alpn rules stay absent.
pub fn check(records: &[Record], limits: Limits) -> Result<Outcome, Unavailable> {
    if records.len() > 64 {
        return Err(Unavailable::Budget);
    }
    let mut bytes = 0usize;
    for record in records {
        limits.validate(record)?;
        bytes = bytes
            .checked_add(record.bytes())
            .ok_or(Unavailable::Budget)?;
    }
    if bytes > limits.max_payload_bytes {
        return Err(Unavailable::Budget);
    }
    let mut outcome = Outcome {
        signals: Vec::new(),
        coverage: Coverage::Insufficient,
    };
    if records.len() < 2 {
        return Ok(outcome);
    }
    if records.iter().all(|r| {
        !r.ja4.is_empty()
            && !r.engine.is_empty()
            && !r.ua_hash.is_empty()
            && !r.remote_addr.is_empty()
            && !r.proto.is_empty()
    }) {
        outcome.coverage = Coverage::UnqualifiedHistory;
    }
    let mut engines = BTreeSet::new();
    let mut ja4 = BTreeSet::new();
    let mut ua = BTreeSet::new();
    let mut subnets = BTreeSet::new();
    let mut protocols = BTreeMap::<&str, usize>::new();
    for record in records {
        if !record.engine.is_empty() {
            engines.insert(record.engine.as_str());
        }
        if !record.ja4.is_empty() {
            ja4.insert(stable(&record.ja4));
        }
        if !record.ua_hash.is_empty() {
            ua.insert(record.ua_hash.as_str());
        }
        if !record.remote_addr.is_empty() {
            subnets.insert(subnet_key(&record.remote_addr));
        }
        if !record.proto.is_empty() {
            *protocols.entry(&record.proto).or_default() += 1;
        }
    }
    let mut add = |id, value, verdict, notes| {
        outcome
            .signals
            .push(Signal::new(id, value, verdict, 1.0, Source::Server, notes))
    };
    if engines.len() > 1 {
        add(
            "l5.traffic.engine_rotation",
            json!(engines),
            Verdict::Bot,
            "TLS engine family rotated within session",
        );
    }
    if ja4.len() > 2 {
        add(
            "l5.traffic.ja4_rotation",
            json!(ja4.len()),
            Verdict::Bot,
            "JA4 (stable part) rotated within session",
        );
    }
    if ua.len() > 1 {
        add(
            "l5.traffic.ua_rotation",
            json!(ua.len()),
            Verdict::Bot,
            "User-Agent rotated within session",
        );
    }
    if subnets.len() > 1 {
        add(
            "l5.traffic.ip_hop",
            json!(subnets),
            Verdict::Suspicious,
            "remote subnet changed within session",
        );
    }
    if protocols.contains_key("1.1") && protocols.contains_key("2") {
        add(
            "l5.traffic.proto_downgrade",
            json!(protocols),
            Verdict::Suspicious,
            "mixed h2/h1 within session",
        );
    }
    Ok(outcome)
}
fn stable(ja4: &str) -> &str {
    ja4.match_indices('_')
        .nth(1)
        .map_or(ja4, |(index, _)| &ja4[..index])
}

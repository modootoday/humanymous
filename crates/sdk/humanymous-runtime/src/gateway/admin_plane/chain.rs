//! Chain-backed admin read routes: `integrity`, `proof`, `audit`,
//! `incidents/{handle}`, `keys` and `checkpoints` of legacy
//! internal/gate/admin.go and incident.go (`adminIntegrity`, `adminProof`,
//! `adminAudit`, `adminIncident`, `adminKeys`, `adminCheckpoints`,
//! `incidentHandle`, `resolveIncident`) at baseline 981d25c.
//!
//! [`AdminPlane::with_chain`] attaches a shared [`Log`]. The plane then audits
//! into that log, so its own `admin.access` meta-audit is part of the chain it
//! reports on, as Go's `sink.Log()` was. Without a chain the routes answer the
//! plain 404, as before.
//!
//! Kept from Go:
//! - `integrity` reports node, ok, class, records and checkpoints; on failure
//!   `divergentSeq` and `detail`; with a co-signer `witnessed` (and
//!   `witnessFailAt`, the tree size of the first checkpoint without a valid
//!   co-signature); and `lastSTH` (`treeSize`, `root`) once a head exists. An
//!   empty chain is `ok: false`, class `empty-chain`.
//! - `keys` publishes only verification material: `node_id`, `key_id` `k1`,
//!   the hex writer key and the hex witness key or `""`. No HMAC key or seed.
//! - `checkpoints` lists every signed head (`null` when there is none) and the
//!   count. `proof?seq=N` proves inclusion of record N against the latest
//!   signed head, with Go's 400 and 404 texts.
//! - `audit` is `gateway::operations::audit_search` newest first with an
//!   opaque incident handle on every record; a query name takes its first
//!   value.
//! - `incidents/{handle}` accepts only the opaque `INC-` handle (an HMAC of the
//!   sequence under the incident key), never a raw sequence. Each operator may
//!   look up [`INCIDENT_HARD`] incidents per [`INCIDENT_WINDOW`]; the lookup
//!   that reaches the cap, and every one after it in the window, answers 429
//!   and records `recon.decision_probing`.
//! - `ok` is `Log::self_verify`, which, like Go `SelfVerify`, also requires
//!   every co-signature when a witness is configured.
//!
//! Deliberate fail-closed differences:
//! - An unreadable chain, a poisoned writer or a failed recon record answers
//!   503 instead of a partial answer (not reachable from these tests: the
//!   shared log has no failure injection).
//! - A lookup the limiter cannot admit (a resource bound or a clock
//!   regression) counts as over the cap. Pinned by a test.
//! - `proof` accepts only plain decimal digits for `seq` (Go `ParseUint`
//!   semantics; Rust `parse` alone would also accept a `+` sign). Pinned by a
//!   test.
use super::{AdminPlane, Clock, Response, audit_unavailable, query_value};
use crate::audit::log::Log;
use crate::audit::record::Record;
use crate::authority::decision::DurableAudit;
use crate::gateway::admin_approval::Operator;
use crate::gateway::operations::audit_search::{self, Query};
use crate::gateway::token::TokenKey;
use crate::network::history::Unavailable;
use crate::network::history::rate::{self, Level, Limiter, Observation};
use serde_json::{Map, Value, json};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// Incident lookup window (Go `abuse.NewLimiter(time.Minute, 60, 120)`).
pub const INCIDENT_WINDOW: Duration = Duration::from_secs(60);
/// Soft cutpoint of the incident limiter (informational, as in Go).
pub const INCIDENT_SOFT: usize = 60;
/// Lookups per window per operator at which the cap trips.
pub const INCIDENT_HARD: usize = 120;

const NO_SEQ: &str = "seq query parameter required";
const NO_HEAD: &str = "no signed tree head yet (record not checkpointed)";
const NOT_COVERED: &str = "record not covered by the latest signed tree head";
const TOO_MANY: &str = "too many incident lookups";
const NOT_FOUND: &str = "incident not found";
const RECON_NOTE: &str = "incident enumeration cap exceeded (bulk-read)";

/// The shared chain, the incident handle key, the per-operator incident
/// limiter and the cached integrity gauges that `metrics` serves.
pub struct Chain {
    log: Arc<Mutex<Log>>,
    incidents: TokenKey,
    limiter: Mutex<Limiter>,
    integrity_ok: AtomicBool,
    witnessed: AtomicBool,
}

/// The audit-chain families of Go `adminMetrics`.
pub(super) struct Gauges {
    pub records: u64,
    pub checkpoints: usize,
    pub integrity_ok: bool,
    pub witnessed: bool,
}

impl Chain {
    /// `incidents` keys the opaque handles (Go used the shared token key).
    /// `limits` bounds the incident limiter; its window must allow
    /// [`INCIDENT_WINDOW`].
    pub fn new(
        log: Arc<Mutex<Log>>,
        incidents: TokenKey,
        limits: rate::Limits,
    ) -> Result<Self, Unavailable> {
        let config = rate::Config::new(INCIDENT_WINDOW, INCIDENT_SOFT, INCIDENT_HARD, limits)?;
        Ok(Self {
            log,
            incidents,
            limiter: Mutex::new(Limiter::new(config)),
            // Go stores false at construction (SoT-38 P0-5: an empty chain
            // is not a green pass) until RefreshIntegrityMetrics runs.
            integrity_ok: AtomicBool::new(false),
            witnessed: AtomicBool::new(false),
        })
    }

    pub(super) fn log(&self) -> MutexGuard<'_, Log> {
        self.log.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Record and checkpoint counts read now; integrity and witness state
    /// from the cache that [`AdminPlane::refresh_integrity_metrics`] fills.
    pub(super) fn gauges(&self) -> io::Result<Gauges> {
        let log = self.log();
        Ok(Gauges {
            records: log.len(),
            checkpoints: log.checkpoints()?.len(),
            integrity_ok: self.integrity_ok.load(Ordering::SeqCst),
            witnessed: self.witnessed.load(Ordering::SeqCst),
        })
    }

    /// Opaque handle for a sequence number (Go `incidentHandle`).
    pub fn incident_handle(&self, seq: u64) -> String {
        self.incidents.incident_handle(seq)
    }
}

/// The plane's audit sink when a chain is attached: append to the shared log.
struct ChainSink(Arc<Mutex<Log>>);

impl DurableAudit for ChainSink {
    fn append(&mut self, record: Record) -> io::Result<()> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .append(record)
            .map(|_| ())
    }
}

impl AdminPlane {
    /// Serve the chain-backed read routes from `chain`, and audit into its
    /// log instead of the sink given to [`AdminPlane::new`].
    pub fn with_chain(mut self, chain: Chain) -> Self {
        self.audit = Mutex::new(Box::new(ChainSink(chain.log.clone())));
        self.chain = Some(chain);
        self
    }

    /// Go `RefreshIntegrityMetrics`: recompute the cached
    /// `hmn_gate_audit_integrity_ok` and `hmn_gate_audit_witnessed` gauges.
    /// Call it off the request path (at boot and on a maintenance tick),
    /// never per scrape: a full-chain self-verify reads the whole log under
    /// the audit lock. `witnessed` needs at least one signed head, a witness
    /// key, and a valid co-signature on every head (Go `VerifyWitness`). An
    /// unreadable checkpoint list reads as not witnessed. Without a chain
    /// this does nothing.
    pub fn refresh_integrity_metrics(&self) {
        let Some(chain) = &self.chain else {
            return;
        };
        let log = chain.log();
        let ok = log.self_verify().ok;
        let witnessed = match (log.checkpoints(), log.witness_public_key()) {
            (Ok(checkpoints), Some(key)) if !checkpoints.is_empty() => checkpoints
                .iter()
                .all(|checkpoint| checkpoint.verify_witness(&key)),
            _ => false,
        };
        drop(log);
        chain.integrity_ok.store(ok, Ordering::SeqCst);
        chain.witnessed.store(witnessed, Ordering::SeqCst);
    }

    pub(super) fn integrity(&self) -> Response {
        let Some(chain) = &self.chain else {
            return Response::not_found();
        };
        let log = chain.log();
        let Ok(checkpoints) = log.checkpoints() else {
            return audit_unavailable();
        };
        let report = log.self_verify();
        let mut out = Map::new();
        out.insert("node".into(), json!(self.node));
        out.insert("ok".into(), json!(report.ok));
        out.insert("class".into(), json!(report.class));
        out.insert("records".into(), json!(log.len()));
        out.insert("checkpoints".into(), json!(checkpoints.len()));
        if !report.ok {
            out.insert("divergentSeq".into(), json!(report.at_seq));
            out.insert("detail".into(), json!(report.detail));
        }
        if let Some(witness) = log.witness_public_key() {
            match checkpoints
                .iter()
                .find(|checkpoint| !checkpoint.verify_witness(&witness))
            {
                Some(failed) => {
                    out.insert("witnessed".into(), json!(false));
                    out.insert("witnessFailAt".into(), json!(failed.tree_size));
                }
                None => {
                    out.insert("witnessed".into(), json!(true));
                }
            }
        }
        if let Some(last) = checkpoints.last() {
            out.insert(
                "lastSTH".into(),
                json!({"treeSize": last.tree_size, "root": last.root}),
            );
        }
        Response::json(&Value::Object(out))
    }

    pub(super) fn keys(&self) -> Response {
        let Some(chain) = &self.chain else {
            return Response::not_found();
        };
        let log = chain.log();
        Response::json(&json!({
            "node_id": self.node,
            "key_id": "k1",
            "sth_public_key": hex::encode(log.writer_public_key()),
            "witness_public_key": log.witness_public_key().map(hex::encode).unwrap_or_default(),
        }))
    }

    pub(super) fn checkpoints(&self) -> Response {
        let Some(chain) = &self.chain else {
            return Response::not_found();
        };
        let Ok(checkpoints) = chain.log().checkpoints() else {
            return audit_unavailable();
        };
        let count = checkpoints.len();
        // Go encodes a nil slice as null.
        let list = if checkpoints.is_empty() {
            Value::Null
        } else {
            json!(checkpoints)
        };
        Response::json(&json!({"checkpoints": list, "count": count}))
    }

    pub(super) fn proof(&self, target: &str) -> Response {
        let Some(chain) = &self.chain else {
            return Response::not_found();
        };
        let text = query_value(target, "seq");
        let seq = if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) {
            text.parse::<u64>().unwrap_or(0)
        } else {
            0
        };
        if seq == 0 {
            return Response::text(400, NO_SEQ);
        }
        let log = chain.log();
        let Ok(checkpoints) = log.checkpoints() else {
            return audit_unavailable();
        };
        let Some(head) = checkpoints.last() else {
            return Response::text(404, NO_HEAD);
        };
        let proof = match log.inclusion_proof_at(seq, head.tree_size) {
            Ok(Some(proof)) => proof,
            Ok(None) => return Response::text(404, NOT_COVERED),
            Err(_) => return audit_unavailable(),
        };
        let path: Vec<String> = proof.path.iter().map(hex::encode).collect();
        Response::json(&json!({
            "seq": seq,
            "leafData": hex::encode(&proof.leaf_data),
            "leafIndex": proof.leaf_index,
            "treeSize": proof.tree_size,
            "proof": path,
            "sth": {
                "treeSize": head.tree_size,
                "merkleRoot": head.merkle_root,
                "sig": head.sig,
                "witnessSig": head.witness_sig,
            },
        }))
    }

    pub(super) fn audit_stream(&self, target: &str) -> Response {
        let Some(chain) = &self.chain else {
            return Response::not_found();
        };
        let values: Vec<(&str, String)> = [
            "limit", "verdict", "host", "route", "rule", "subject", "minRisk", "before",
        ]
        .into_iter()
        .map(|name| (name, query_value(target, name)))
        .collect();
        let query = Query::from_pairs(values.iter().map(|(name, value)| (*name, value.as_str())));
        let Ok(records) = chain.log().records() else {
            return audit_unavailable();
        };
        let page = audit_search::search(&records, &query, |seq| chain.incident_handle(seq));
        let records: Vec<Record> = page
            .records
            .into_iter()
            .map(|hit| Record {
                incident: hit.incident,
                ..hit.record
            })
            .collect();
        Response::json(&json!({
            "records": records,
            "count": page.count,
            "nextBefore": page.next_before,
        }))
    }

    pub(super) fn incident(&self, handle: &str, operator: &Operator, clock: Clock) -> Response {
        let Some(chain) = &self.chain else {
            return Response::not_found();
        };
        let observed = chain
            .limiter
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .observe(&format!("inc:{}", operator.id), clock.monotonic);
        let capped = !matches!(
            observed,
            Ok(Observation::Counted {
                level: Level::Normal | Level::Soft,
                ..
            })
        );
        if capped {
            let recon = Record {
                event_type: "recon.decision_probing".into(),
                actor: self.actor(&operator.id),
                tenant_id: self.node.clone(),
                route_class: "control".into(),
                fail_reason: RECON_NOTE.into(),
                key_id: "k1".into(),
                ..Record::default()
            };
            if self.emit(recon).is_err() {
                return audit_unavailable();
            }
            return Response::text(429, TOO_MANY);
        }
        let Ok(records) = chain.log().records() else {
            return audit_unavailable();
        };
        match records
            .into_iter()
            .find(|record| chain.incident_handle(record.seq) == handle)
        {
            Some(record) => Response::json(&Record {
                incident: handle.to_owned(),
                ..record
            }),
            None => Response::text(404, NOT_FOUND),
        }
    }
}

#[cfg(test)]
mod tests;

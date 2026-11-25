//! Differential ports of legacy internal/gate/admin_integrity_empty_test.go,
//! admin_offline_verify_test.go, ws4_test.go and the chain half of
//! admin_test.go (TestAdminReadAPIs) at baseline 981d25c against outputs
//! recorded from the Go oracle
//! (`tests/fixtures/gateway/attest-floor-and-admin-chain-go-golden.json`).
//! Requests go through `AdminPlane::handle`, like Go `adminDo` through the
//! admin handler, with a shared sealed log like Go `sink.Log()`.
use super::*;
use crate::audit::checkpoint::{Checkpoint, Witness};
use crate::audit::control::Scope;
use crate::audit::log::Config;
use crate::audit::merkle::Hash;
use crate::audit::storage::{Wal, WalOptions};
use crate::audit::verification::{Mismatch, verify, verify_with_witness};
use crate::gateway::admin_approval::Role;
use crate::gateway::admin_plane::PREFIX;
use crate::gateway::ban::Store;
use crate::gateway::http::Request;
use crate::network::history::rate::Limits;
use ed25519_dalek::SigningKey;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{fs, io};

static GOLDEN: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/gateway/attest-floor-and-admin-chain-go-golden.json"
    ))
    .unwrap()
});

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A unique scratch directory removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "humanymous-admin-chain-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Ok(entries) = fs::read_dir(&self.0) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
        let _ = fs::remove_dir(&self.0);
    }
}

/// Discards the plane's original sink; `with_chain` replaces it.
struct Discard;

impl DurableAudit for Discard {
    fn append(&mut self, _: Record) -> io::Result<()> {
        Ok(())
    }
}

/// Go `banStack` (node `test`, HMAC key `k`, a head every eight records) or
/// the offline-verify stack (node `audit-node`, a head every four, a witness).
struct Stack {
    plane: AdminPlane,
    log: Arc<Mutex<Log>>,
    _scratch: Scratch,
}

fn open(dir: &Path, node: &str, hmac: &[u8], every: u64, witness: bool) -> Log {
    let seed = [7; 32];
    let writer = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
    Log::open(
        Wal::open(dir, WalOptions::default()).unwrap(),
        Config {
            node_id: node.into(),
            hmac_key: hmac.to_vec(),
            signing_seed: seed,
            checkpoint_every: every,
            witness: witness.then(|| Witness::from_seed(&[9; 32], writer, node.into())),
        },
        None,
        true,
    )
    .unwrap()
}

fn limits(hits: usize) -> Limits {
    Limits::new(64, hits, 128, Duration::from_secs(60), 1 << 20, 64).unwrap()
}

fn stack_with(node: &str, hmac: &[u8], every: u64, witness: bool, hits: usize) -> Stack {
    let scratch = Scratch::new();
    let log = Arc::new(Mutex::new(open(&scratch.0, node, hmac, every, witness)));
    let rate =
        crate::network::history::rate::Config::new(Duration::from_secs(60), 998, 1000, limits(64))
            .unwrap();
    let bans = Arc::new(Mutex::new(Store::new(rate, 64).unwrap()));
    let mut incident_key = b"incident-handle-key".to_vec();
    incident_key.resize(32, b'.');
    let chain = Chain::new(
        log.clone(),
        TokenKey::new(incident_key).unwrap(),
        limits(hits),
    )
    .unwrap();
    let plane = AdminPlane::new(
        node,
        Scope {
            installation: "installation".into(),
            node: node.into(),
        },
        b"k",
        bans,
        Box::new(Discard),
    )
    .with_chain(chain);
    for role in [Role::Auditor, Role::Operator, Role::Approver, Role::Dpo] {
        plane
            .auth()
            .add(&token(role.as_str()), &format!("{}-1", role.as_str()), role)
            .unwrap();
    }
    Stack {
        plane,
        log,
        _scratch: scratch,
    }
}

fn ban_stack() -> Stack {
    stack_with("test", b"k", 8, false, 1024)
}

fn token(role: &str) -> String {
    format!("{role}-token-0123456789")
}

const CLOCK: Clock = Clock {
    monotonic: Duration::from_secs(5),
    unix: 1_800_000_000,
};

impl Stack {
    /// Go `adminDo`.
    fn get(&self, role: &str, sub: &str) -> Response {
        self.at(role, sub, CLOCK)
    }

    fn at(&self, role: &str, sub: &str, clock: Clock) -> Response {
        let mut headers = vec![("Host".to_owned(), "admin".to_owned())];
        if !role.is_empty() {
            headers.push(("Authorization".into(), format!("Bearer {}", token(role))));
        }
        let request = Request {
            method: "GET".into(),
            target: format!("{PREFIX}{sub}"),
            headers,
            body: Vec::new(),
        };
        self.plane.handle(&request, clock)
    }

    /// Go `sink.Emit` of a sealed decision.
    fn emit(&self) {
        self.log
            .lock()
            .unwrap()
            .append(Record {
                event_type: "gate.decision".into(),
                verdict: "allow".into(),
                enforcement_mode: "enforce".into(),
                key_id: "k1".into(),
                ..Record::default()
            })
            .unwrap();
    }

    fn records(&self) -> Vec<Record> {
        self.log.lock().unwrap().records().unwrap()
    }
}

fn body(response: &Response) -> &str {
    std::str::from_utf8(&response.body).unwrap()
}

fn json(response: &Response) -> Value {
    serde_json::from_slice(&response.body).unwrap()
}

fn keys(value: &Value) -> Value {
    Value::Array(
        value
            .as_object()
            .unwrap()
            .keys()
            .map(|key| Value::String(key.clone()))
            .collect(),
    )
}

/// Go TestAdminIntegrityEmptyChain: an empty chain reports `ok: false`, class
/// `empty-chain`, byte for byte; one sealed decision self-verifies; the
/// authenticated route meta-audits first and still verifies.
#[test]
fn go_admin_integrity_empty_chain_matches_capture() {
    let golden = &GOLDEN["integrity"];
    let s = ban_stack();
    let empty = s.plane.integrity();
    assert_eq!(empty.status, golden["empty"]["status"]);
    assert_eq!(empty.content_type, golden["empty"]["content_type"]);
    assert_eq!(body(&empty), golden["empty"]["body"]);

    s.emit();
    let one = json(&s.plane.integrity());
    for (name, field) in [
        ("keys", keys(&one)),
        ("ok", one["ok"].clone()),
        ("class", one["class"].clone()),
        ("records", one["records"].clone()),
        ("checkpoints", one["checkpoints"].clone()),
    ] {
        assert_eq!(field, golden["one_record"][name], "one_record.{name}");
    }

    let unauthenticated = s.get("", "integrity");
    assert_eq!(unauthenticated.status, golden["unauth"]["status"]);
    assert_eq!(body(&unauthenticated), golden["unauth"]["body"]);
    let w = s.get("auditor", "integrity");
    assert_eq!(w.status, golden["http_auditor"]["status"]);
    let http = json(&w);
    for (name, field) in [
        ("keys", keys(&http)),
        ("ok", http["ok"].clone()),
        ("class", http["class"].clone()),
        ("records", http["records"].clone()),
        ("checkpoints", http["checkpoints"].clone()),
    ] {
        assert_eq!(field, golden["http_auditor"][name], "http_auditor.{name}");
    }
}

/// Go TestAdminKeysEnableOfflineVerify: an auditor fetches the published keys
/// and signed heads, verifies the exported chain with the public key alone
/// (`hmac-unchecked`) and an inclusion proof against the exported root.
#[test]
fn go_admin_keys_enable_offline_verify() {
    let golden = &GOLDEN["chain_reads"];
    let s = stack_with("audit-node", b"offline-hmac-key", 4, true, 1024);
    for _ in 0..6 {
        s.emit();
    }

    let w = s.get("auditor", "keys");
    let published = json(&w);
    let go = &golden["keys_witness"];
    assert_eq!(w.status, go["status"]);
    assert_eq!(keys(&published), go["keys"]);
    assert_eq!(published["key_id"], go["key_id"]);
    assert_eq!(published["node_id"], go["node_id"]);
    let writer: [u8; 32] = hex::decode(published["sth_public_key"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    let witness: [u8; 32] = hex::decode(published["witness_public_key"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(64, go["sth_public_key_len"]);
    assert_eq!(64, go["witness_public_key_len"]);

    let w = s.get("auditor", "proof?seq=1");
    let proof = json(&w);
    let go = &golden["proof_ok"];
    assert_eq!(w.status, go["status"]);
    assert_eq!(keys(&proof), go["keys"]);
    assert_eq!(keys(&proof["sth"]), go["sth_keys"]);
    for name in ["seq", "leafIndex"] {
        assert_eq!(proof[name], go[name], "proof.{name}");
    }
    // Existing audit::log difference: with a witness, Rust seals the first
    // record at once and then every four (heads at 1, 5, 9); Go sealed at 4
    // and 8. The latest head when the proof is served covers 5 records in
    // Rust and 8 in Go.
    assert_eq!(
        (&go["treeSize"], &go["sth_treeSize"]),
        (&json!(8), &json!(8))
    );
    assert_eq!(
        (&proof["treeSize"], &proof["sth"]["treeSize"]),
        (&json!(5), &json!(5))
    );
    assert_eq!(proof["proof"].as_array().unwrap().len(), 3);
    assert!(!proof["sth"]["sig"].as_str().unwrap().is_empty());

    let w = s.get("auditor", "proof?seq=1000");
    assert_eq!(w.status, golden["proof_uncovered"]["status"]);
    assert_eq!(body(&w), golden["proof_uncovered"]["body"]);

    let w = s.get("auditor", "checkpoints");
    let exported = json(&w);
    let go = &golden["checkpoints_witness"];
    assert_eq!(w.status, go["status"]);
    assert_eq!(keys(&exported), go["keys"]);
    assert_eq!(keys(&exported["checkpoints"][0]), go["checkpoint_keys"]);
    // Heads 1, 5, 9 here; 4, 8 in Go (see above).
    assert_eq!((&go["count"], &go["tree_size"]), (&json!(2), &json!(4)));
    assert_eq!(
        (&exported["count"], &exported["checkpoints"][0]["tree_size"]),
        (&json!(3), &json!(1))
    );

    let integrity = json(&s.get("auditor", "integrity"));
    let go = &golden["integrity_witness"];
    assert_eq!(keys(&integrity), go["keys"]);
    assert_eq!(keys(&integrity["lastSTH"]), go["lastSTH_keys"]);
    for name in ["ok", "class", "witnessed", "records"] {
        assert_eq!(integrity[name], go[name], "integrity.{name}");
    }
    assert_eq!(
        (&go["checkpoints"], &integrity["checkpoints"]),
        (&json!(2), &json!(3))
    );

    // Offline: only the export and the published keys, no HMAC key.
    let heads: Vec<Checkpoint> = serde_json::from_value(exported["checkpoints"].clone()).unwrap();
    let records = s.records();
    let report = verify(&records, &heads, &[], Some(&writer));
    assert_eq!(report.ok, golden["offline_verify"]["ok"]);
    assert_eq!(json!(report.class), golden["offline_verify"]["class"]);
    assert_eq!(report.class, Mismatch::HmacUnchecked);
    assert!(verify_with_witness(&records, &heads, &[], Some(&writer), &witness).ok);
    let leaf = hex::decode(proof["leafData"].as_str().unwrap()).unwrap();
    let root: Hash = hex::decode(proof["sth"]["merkleRoot"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    let path: Vec<Hash> = proof["proof"]
        .as_array()
        .unwrap()
        .iter()
        .map(|sibling| {
            hex::decode(sibling.as_str().unwrap())
                .unwrap()
                .try_into()
                .unwrap()
        })
        .collect();
    let inclusion = crate::audit::log::InclusionProof {
        leaf_data: leaf,
        leaf_index: 0,
        tree_size: proof["treeSize"].as_u64().unwrap() as usize,
        path,
    };
    assert!(
        inclusion.verify(&root),
        "inclusion against the exported root"
    );
}

/// Keys without a co-signer, the `proof` refusals and an empty head list.
#[test]
fn go_keys_proof_and_checkpoint_refusals_match_capture() {
    let golden = &GOLDEN["chain_reads"];
    let s = ban_stack();
    let published = json(&s.get("auditor", "keys"));
    let go = &golden["keys_no_witness"];
    assert_eq!(keys(&published), go["keys"]);
    for name in ["key_id", "node_id", "witness_public_key"] {
        assert_eq!(published[name], go[name], "keys.{name}");
    }
    assert_eq!(
        published["sth_public_key"].as_str().unwrap().len(),
        go["sth_public_key_len"]
    );
    for (case, sub) in [
        ("proof_no_seq", "proof"),
        ("proof_bad_seq", "proof?seq=abc"),
        ("proof_no_sth", "proof?seq=1"),
        ("checkpoints_empty", "checkpoints"),
    ] {
        let w = s.get("auditor", sub);
        assert_eq!(w.status, golden[case]["status"], "{case}");
        assert_eq!(body(&w), golden[case]["body"], "{case}");
        if case == "proof_no_sth" {
            assert_eq!(s.log.lock().unwrap().len(), golden[case]["records"]);
        }
    }
    // Go ParseUint has no sign; neither does this route.
    assert_eq!(s.get("auditor", "proof?seq=+1").status, 400);
}

/// The audit stream and incident lookup of Go TestAdminReadAPIs, then Go
/// TestIncidentEnumerationCap on the same stack: the cap trips at the Go
/// lookup, answers 429 with the Go body for the rest of the window, records
/// `recon.decision_probing` for every capped lookup, and is per operator.
#[test]
fn go_audit_stream_incidents_and_enumeration_cap_match_capture() {
    let golden = &GOLDEN["incidents"];
    let s = ban_stack();
    for _ in 0..3 {
        s.emit();
    }
    let w = s.get("auditor", "audit?limit=2");
    let stream = json(&w);
    let go = &golden["audit"];
    assert_eq!(w.status, go["status"]);
    assert_eq!(keys(&stream), go["keys"]);
    assert_eq!(stream["count"], go["count"]);
    assert_eq!(stream["nextBefore"], go["nextBefore"]);
    let seqs: Vec<Value> = stream["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["seq"].clone())
        .collect();
    assert_eq!(Value::Array(seqs), go["seqs"]);
    let handle = stream["records"][0]["incident"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(&handle[..4], go["handle_prefix"]);
    assert_eq!(handle.len(), go["handle_len"]);
    assert_ne!(handle, s.plane.chain.as_ref().unwrap().incident_handle(3));

    let w = s.get("auditor", &format!("incidents/{handle}"));
    let incident = json(&w);
    let go = &golden["incident"];
    assert_eq!(w.status, go["status"]);
    assert_eq!(incident["seq"], go["seq"]);
    assert_eq!(
        incident["incident"] == handle.as_str(),
        go["incident_equals_handle"]
    );
    assert_eq!(incident["event_type"], go["event_type"]);
    let raw = s.get("auditor", "incidents/1");
    assert_eq!(raw.status, golden["incident_raw"]["status"]);
    assert_eq!(body(&raw), golden["incident_raw"]["body"]);

    let go = &golden["enumeration"];
    let before = s.records().len();
    let mut first = None;
    let mut sampled = Vec::new();
    for index in 0..200 {
        let w = s.get("auditor", "incidents/INC-NOPE");
        if w.status == 429 && first.is_none() {
            first = Some(index);
            assert_eq!(body(&w), go["body_429"]);
        }
        if index < 3 || (115..=120).contains(&index) || index >= 197 {
            sampled.push(json!(w.status));
        }
    }
    assert_eq!(json!(first.unwrap()), go["first_429_index"]);
    assert_eq!(Value::Array(sampled), go["sampled_statuses"]);
    let recon: Vec<Record> = s.records()[before..]
        .iter()
        .filter(|record| record.event_type == "recon.decision_probing")
        .cloned()
        .collect();
    assert_eq!(json!(recon.len()), go["recon_records"]);
    let expected = &go["recon_first"];
    let record = &recon[0];
    for (name, field) in [
        ("event_type", &record.event_type),
        ("actor_kind", &record.actor.kind),
        ("tenant_id", &record.tenant_id),
        ("route_class", &record.route_class),
        ("verdict", &record.verdict),
        ("action", &record.enforcement_action),
        ("mode", &record.enforcement_mode),
        ("fail_reason", &record.fail_reason),
        ("key_id", &record.key_id),
    ] {
        assert_eq!(field.as_str(), expected[name], "recon.{name}");
    }
    let other = s.get("operator", "incidents/INC-NOPE");
    assert_eq!(other.status, go["other_operator_status"]);
    assert_eq!(body(&other), go["other_operator_body"]);
    // Every authenticated read was meta-audited (Go hasEvent admin.access).
    assert!(s.records().iter().any(|r| r.event_type == "admin.access"));
}

/// A lookup the limiter cannot admit is capped like an over-limit one: a
/// per-key history bound, then a clock regression.
#[test]
fn unadmitted_incident_lookups_are_capped() {
    let s = stack_with("test", b"k", 8, false, 3);
    for _ in 0..3 {
        assert_eq!(s.get("auditor", "incidents/INC-NOPE").status, 404);
    }
    assert_eq!(s.get("auditor", "incidents/INC-NOPE").status, 429);
    let s = ban_stack();
    assert_eq!(s.get("auditor", "incidents/INC-NOPE").status, 404);
    let earlier = Clock {
        monotonic: Duration::from_secs(1),
        ..CLOCK
    };
    assert_eq!(s.at("operator", "incidents/INC-NOPE", earlier).status, 429);
}

/// Without an attached chain every chain read is the plain 404, as before.
#[test]
fn chain_routes_are_not_served_without_a_chain() {
    let rate =
        crate::network::history::rate::Config::new(Duration::from_secs(60), 998, 1000, limits(64))
            .unwrap();
    let plane = AdminPlane::new(
        "test",
        Scope {
            installation: "installation".into(),
            node: "test".into(),
        },
        b"k",
        Arc::new(Mutex::new(Store::new(rate, 64).unwrap())),
        Box::new(Discard),
    );
    plane
        .auth()
        .add(&token("auditor"), "auditor-1", Role::Auditor)
        .unwrap();
    for sub in [
        "integrity",
        "proof?seq=1",
        "audit",
        "incidents/INC-1",
        "keys",
        "checkpoints",
    ] {
        let request = Request {
            method: "GET".into(),
            target: format!("{PREFIX}{sub}"),
            headers: vec![(
                "Authorization".into(),
                format!("Bearer {}", token("auditor")),
            )],
            body: Vec::new(),
        };
        let w = plane.handle(&request, CLOCK);
        assert_eq!((w.status, body(&w)), (404, "404 page not found\n"), "{sub}");
    }
}

/// Go TestWargameR166_MetaAuditOnBanList on a chain-backed plane: an
/// auditor's ban list is meta-audited into the shared chain, and the soft
/// check `GET integrity` then answers 200 with the chain verifying.
#[test]
fn wargame_r166_meta_audit_on_ban_list_keeps_the_chain_intact() {
    let s = ban_stack();
    assert_eq!(s.get("auditor", "bans").status, 200);
    let access: Vec<_> = s
        .records()
        .into_iter()
        .filter(|record| record.event_type == "admin.access")
        .collect();
    assert_eq!(access.len(), 1);
    assert_eq!(access[0].fail_reason, "bans (role=auditor)");
    let w = s.get("auditor", "integrity");
    assert_eq!(w.status, 200, "{}", body(&w));
    let integrity = json(&w);
    assert_eq!(integrity["ok"], true);
    assert_eq!(integrity["class"], "ok");
    // The integrity read meta-audits itself before verifying: two records.
    assert_eq!(integrity["records"], 2);
}

static POLICY_GOLDEN: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/gateway/admin-policy-console-go-golden.json"
    ))
    .unwrap()
});

/// Go wargame R113 (wargame_escalation_r81_test.go): the keys route publishes
/// `sth_public_key` and no `hmac_key`. The body equals the capture once the
/// random writer key hex is masked as the capture masked it.
#[test]
fn r113_keys_omit_hmac_secret() {
    let s = ban_stack();
    let w = s.get("auditor", "keys");
    assert_eq!(w.status, 200);
    let text = body(&w);
    assert!(!text.contains("hmac_key"), "{text}");
    assert!(text.contains("sth_public_key"), "{text}");
    let key = json(&w)["sth_public_key"].as_str().unwrap().to_owned();
    assert_eq!(key.len(), 64);
    let masked = text.replacen(&key, "<64 hex chars>", 1);
    let golden = &POLICY_GOLDEN["keys"];
    assert_eq!(w.content_type, golden["content_type"]);
    assert_eq!(masked, golden["body"]);
}

/// Go wargame R131 (wargame_escalation_r121_test.go): a log without a
/// co-signer has no witness public key, so integrity omits `witnessed` and
/// `witnessFailAt`.
#[test]
fn r131_nil_witness_public_omits_witnessed_field() {
    let golden = &POLICY_GOLDEN["integrity_no_witness"];
    let s = ban_stack();
    assert_eq!(
        s.log.lock().unwrap().witness_public_key().is_none(),
        golden["witness_public_key_nil"]
    );
    let w = s.get("auditor", "integrity");
    assert_eq!(w.status, golden["status"]);
    let report = json(&w);
    let object = report.as_object().unwrap();
    assert_eq!(object.contains_key("witnessed"), golden["has_witnessed"]);
    assert_eq!(
        object.contains_key("witnessFailAt"),
        golden["has_witnessFailAt"]
    );
}

fn gauge(text: &str, name: &str) -> String {
    let prefix = format!("{name} ");
    text.lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .unwrap_or_else(|| panic!("{name} missing: {text}"))
        .to_owned()
}

/// Go `adminMetrics` with a chain: the four audit families appear in Go's
/// order and with Go's HELP and TYPE text
/// (`tests/fixtures/gateway/admin-metrics-go-golden.json`); integrity and
/// witness read 0 until `refresh_integrity_metrics` runs (Go stores false at
/// construction), and the counts read the shared log.
#[test]
fn go_admin_metrics_audit_families_follow_the_chain() {
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/gateway/admin-metrics-go-golden.json"
    ))
    .unwrap();
    let s = ban_stack();
    let fresh = s.get("auditor", "metrics");
    assert_eq!(fresh.status, 200);
    let text = body(&fresh).to_owned();
    // Every header and line the chain plane emits is Go's, in Go's order.
    let go: Vec<&str> = golden["fresh"].as_str().unwrap().lines().collect();
    let mut cursor = 0;
    for line in text.lines().filter(|line| line.starts_with('#')) {
        let at = go[cursor..]
            .iter()
            .position(|candidate| *candidate == line)
            .unwrap_or_else(|| panic!("{line} not in Go order"));
        cursor += at + 1;
    }
    for name in [
        "hmn_gate_audit_records_total",
        "hmn_gate_audit_checkpoints_total",
        "hmn_gate_audit_integrity_ok",
        "hmn_gate_audit_witnessed",
    ] {
        assert!(text.contains(&format!("# TYPE {name} ")), "{name}: {text}");
    }
    // The scrape's own admin.access record is sealed before dispatch.
    assert_eq!(gauge(&text, "hmn_gate_audit_records_total"), "1");
    assert_eq!(gauge(&text, "hmn_gate_audit_checkpoints_total"), "0");
    assert_eq!(gauge(&text, "hmn_gate_audit_integrity_ok"), "0");
    assert_eq!(gauge(&text, "hmn_gate_audit_witnessed"), "0");

    for _ in 0..8 {
        s.emit();
    }
    // Not refreshed yet: the cached gauges stay 0 on the scrape path.
    let stale = body(&s.get("auditor", "metrics")).to_owned();
    assert_eq!(gauge(&stale, "hmn_gate_audit_integrity_ok"), "0");
    let (records, heads) = {
        let log = s.log.lock().unwrap();
        (log.len(), log.checkpoints().unwrap().len())
    };
    assert_eq!(
        gauge(&stale, "hmn_gate_audit_records_total"),
        records.to_string()
    );
    assert_eq!(
        gauge(&stale, "hmn_gate_audit_checkpoints_total"),
        heads.to_string()
    );
    assert!(heads >= 1, "a head every eight records");

    s.plane.refresh_integrity_metrics();
    let refreshed = body(&s.get("auditor", "metrics")).to_owned();
    assert_eq!(gauge(&refreshed, "hmn_gate_audit_integrity_ok"), "1");
    // No witness configured: never witnessed.
    assert_eq!(gauge(&refreshed, "hmn_gate_audit_witnessed"), "0");
}

/// Go `RefreshIntegrityMetrics` with a co-signer: witnessed needs a signed
/// head; an empty chain is neither verified nor witnessed.
#[test]
fn go_refresh_integrity_metrics_reports_the_witness() {
    let s = stack_with("audit-node", b"k", 4, true, 1024);
    s.plane.refresh_integrity_metrics();
    let empty = body(&s.get("auditor", "metrics")).to_owned();
    // The refresh ran before the scrape's own access record: empty chain.
    assert_eq!(gauge(&empty, "hmn_gate_audit_integrity_ok"), "0");
    assert_eq!(gauge(&empty, "hmn_gate_audit_witnessed"), "0");
    for _ in 0..4 {
        s.emit();
    }
    s.plane.refresh_integrity_metrics();
    let signed = body(&s.get("auditor", "metrics")).to_owned();
    assert_eq!(gauge(&signed, "hmn_gate_audit_integrity_ok"), "1");
    assert_eq!(gauge(&signed, "hmn_gate_audit_witnessed"), "1");
}

/// Without a chain the refresh does nothing and the families stay omitted.
#[test]
fn refresh_without_a_chain_is_inert() {
    let rate =
        crate::network::history::rate::Config::new(Duration::from_secs(60), 998, 1000, limits(64))
            .unwrap();
    let plane = AdminPlane::new(
        "n1",
        Scope {
            installation: "installation".into(),
            node: "n1".into(),
        },
        b"k",
        Arc::new(Mutex::new(Store::new(rate, 64).unwrap())),
        Box::new(Discard),
    );
    plane
        .auth()
        .add(&token("auditor"), "auditor-1", Role::Auditor)
        .unwrap();
    plane.refresh_integrity_metrics();
    let request = Request {
        method: "GET".into(),
        target: format!("{PREFIX}metrics"),
        headers: vec![
            ("Host".to_owned(), "admin".to_owned()),
            (
                "Authorization".into(),
                format!("Bearer {}", token("auditor")),
            ),
        ],
        body: Vec::new(),
    };
    let response = plane.handle(&request, CLOCK);
    assert_eq!(response.status, 200);
    assert!(!body(&response).contains("hmn_gate_audit_records_total"));
    assert!(!body(&response).contains("hmn_gate_audit_integrity_ok"));
}

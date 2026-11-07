//! Pure authority invariants; fixture ALLOW is not a production security profile.
mod support;
use humanymous_audit::record::Record;
use humanymous_authority::{
    Rejection,
    configuration::Required,
    decision::{ApplicationHttp, Authority, DurableAudit, Evidence},
};
use humanymous_core::integrity::adapter::ValidatedRequest;
use humanymous_core::signals::report::SessionReport;
use std::sync::{Arc, Mutex};
use support::*;

struct Audit {
    records: Arc<Mutex<Vec<Record>>>,
    fail: bool,
}
impl DurableAudit for Audit {
    fn append(&mut self, record: Record) -> std::io::Result<()> {
        if self.fail {
            return Err(std::io::Error::other("injected disk failure"));
        }
        self.records.lock().unwrap().push(record);
        Ok(())
    }
}
// Only in the test binary: this report is frozen synthetic engine evidence,
// not a production collector/capture profile and never an IPC input.
struct Fixture;
impl Evidence for Fixture {
    fn report(&mut self, _: &ValidatedRequest, _: bool) -> Result<SessionReport, Rejection> {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../sdk/humanymous-core/tests/scoring/fixtures/frozen/human-chrome-allow.json"
        ))
        .unwrap();
        Ok(serde_json::from_value(fixture["report"].clone()).unwrap())
    }
}
fn authority(
    config: humanymous_authority::configuration::Configuration,
    fail: bool,
    fixture: bool,
) -> (Authority, Arc<Mutex<Vec<Record>>>) {
    let records = Arc::new(Mutex::new(Vec::new()));
    let evidence: Box<dyn Evidence> = if fixture {
        Box::new(Fixture)
    } else {
        Box::new(ApplicationHttp)
    };
    (
        Authority::new(
            config,
            Default::default(),
            evidence,
            Box::new(Audit {
                records: records.clone(),
                fail,
            }),
        )
        .unwrap(),
        records,
    )
}
fn call(
    a: &mut Authority,
    value: &serde_json::Value,
) -> Result<humanymous_authority::decision::Response, Rejection> {
    let auth = format!("Bearer {}", a.credential());
    a.decide(peer(), &auth, &envelope(value), || Ok(1000))
}
#[test]
fn exact_binding_and_fixture_allow_only_after_audit() {
    let d = Directory::new();
    let (mut a, records) = authority(config(&d), false, true);
    let response = call(&mut a, &request(1000, 1)).unwrap();
    assert_eq!(response.verdict, "ALLOW");
    let wire = serde_json::to_value(&response).unwrap();
    assert_eq!(wire.as_object().unwrap().len(), 7);
    assert_eq!(wire["id"], request(1000, 1)["id"]);
    assert_eq!(response.tenant, "t");
    assert_eq!(response.origin, "o");
    assert_eq!(response.revision, "r");
    assert_eq!(response.expires, 6000);
    let records = records.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].verdict, "ALLOW");
    assert_eq!(records[0].correlation_id.len(), 64);
    let persisted = serde_json::to_string(&records[0]).unwrap();
    assert!(!persisted.contains("Mozilla"));
    assert!(!persisted.contains("YWJj"));
    assert!(!persisted.contains(a.credential()));
}
#[test]
fn auth_scope_revision_inventory_and_no_admin() {
    let d = Directory::new();
    let (mut a, records) = authority(config(&d), false, false);
    let v = request(1000, 1);
    assert_eq!(
        a.decide(peer(), "Bearer wrong", &envelope(&v), || Ok(1000))
            .err()
            .unwrap()
            .status,
        401
    );
    let other = match peer() {
        humanymous_authority::decision::NativePeer::Unix(uid) => {
            humanymous_authority::decision::NativePeer::Unix(uid.wrapping_add(1))
        }
        _ => humanymous_authority::decision::NativePeer::Windows(1),
    };
    assert_eq!(
        a.decide(
            other,
            &format!("Bearer {}", a.credential()),
            &envelope(&v),
            || Ok(1000)
        )
        .err()
        .unwrap()
        .status,
        403
    );
    for field in ["tenant", "origin", "authority"] {
        let mut v = v.clone();
        v[field] = "other".into();
        if field == "authority" {
            v["headers"][0][1] = "other".into();
        }
        assert_eq!(call(&mut a, &v).err().unwrap().status, 403);
    }
    let mut changed = v.clone();
    changed["adapter"]["installation"] = "other".into();
    assert_eq!(call(&mut a, &changed).err().unwrap().status, 403);
    for (field, value, status) in [
        ("revision", "stale", 409),
        ("target", "/unknown", 409),
        ("target", "/_humanymous/admin", 403),
    ] {
        let mut changed = v.clone();
        changed[field] = value.into();
        assert_eq!(call(&mut a, &changed).err().unwrap().status, status);
    }
    assert!(records.lock().unwrap().is_empty());
}
#[test]
fn atomic_replay_and_restart_credential_rotation() {
    let d = Directory::new();
    let (mut a, _) = authority(config(&d), false, false);
    let v = request(1000, 1);
    let old = format!("Bearer {}", a.credential());
    assert!(call(&mut a, &v).is_ok());
    assert_eq!(call(&mut a, &v).err().unwrap().status, 409);
    let mut changed = v.clone();
    changed["target"] = "/api?changed".into();
    assert_eq!(call(&mut a, &changed).err().unwrap().status, 409);
    let (mut restarted, _) = authority(config(&d), false, false);
    assert_eq!(
        restarted
            .decide(peer(), &old, &envelope(&v), || Ok(1000))
            .err()
            .unwrap()
            .status,
        401
    );
    // Fresh epoch can admit this UUID only using its newly provisioned token.
    assert!(call(&mut restarted, &v).is_ok());
}
#[test]
fn no_fabricated_collector_allow_and_explicit_required_signals() {
    let d = Directory::new();
    let (mut a, _) = authority(config(&d), false, false);
    assert_ne!(call(&mut a, &request(1000, 1)).unwrap().verdict, "ALLOW");
    for required in [
        Required {
            raw_tls: true,
            ..Default::default()
        },
        Required {
            original_h2: true,
            ..Default::default()
        },
        Required {
            collector: true,
            ..Default::default()
        },
        Required {
            rit: true,
            ..Default::default()
        },
    ] {
        let mut c = config(&d);
        c.routes[0].required = required;
        let (mut a, records) = authority(c, false, false);
        assert_eq!(call(&mut a, &request(1000, 1)).err().unwrap().status, 422);
        assert!(records.lock().unwrap().is_empty());
    }
    let mut v = request(1000, 2);
    v["headerMode"] = "fetch-normalized".into();
    v["peer"] = serde_json::Value::Null;
    v["connectionId"] = serde_json::Value::Null;
    v["capabilities"]["rawHeaderOrder"] = false.into();
    assert_eq!(call(&mut a, &v).err().unwrap().status, 422);
}
#[test]
fn body_schema_audit_failure_expiry_capacity() {
    let d = Directory::new();
    let (mut a, records) = authority(config(&d), true, true);
    let v = request(1000, 1);
    assert_eq!(call(&mut a, &v).err().unwrap().status, 503);
    assert_eq!(call(&mut a, &request(1000, 2)).err().unwrap().status, 503);
    assert!(records.lock().unwrap().is_empty());
    let mut c = config(&d);
    c.replay_capacity = 1;
    let (mut a, _) = authority(c, false, false);
    let mut body = v.clone();
    body["body"]["sha256"] = "00".repeat(32).into();
    assert_eq!(call(&mut a, &body).err().unwrap().status, 422);
    let mut expired = v.clone();
    expired["deadline"] = 1000.into();
    assert_eq!(call(&mut a, &expired).err().unwrap().status, 400);
    assert!(call(&mut a, &v).is_ok());
    assert_eq!(call(&mut a, &request(1000, 2)).err().unwrap().status, 429);
    let (mut a, records) = authority(config(&d), false, true);
    let clock = std::cell::Cell::new(999);
    let result = a.decide(
        peer(),
        &format!("Bearer {}", a.credential()),
        &envelope(&v),
        || {
            let t = clock.get() + 1;
            clock.set(if t == 1001 { 6000 } else { t });
            Ok(t)
        },
    );
    assert_eq!(result.err().unwrap().status, 503);
    assert_eq!(records.lock().unwrap().len(), 1); // Durable but too late: no response.
    let mut rebound = v;
    rebound["deadline"] = 7000.into();
    let auth = format!("Bearer {}", a.credential());
    assert_eq!(
        a.decide(peer(), &auth, &envelope(&rebound), || Ok(6001))
            .err()
            .unwrap()
            .status,
        409
    );
}

#[test]
fn clock_rollback_cannot_revive_an_evicted_nonce() {
    let d = Directory::new();
    let (mut a, _) = authority(config(&d), false, false);
    let auth = format!("Bearer {}", a.credential());
    assert!(
        a.decide(peer(), &auth, &envelope(&request(1000, 1)), || Ok(1000))
            .is_ok()
    );
    assert!(
        a.decide(peer(), &auth, &envelope(&request(8000, 2)), || Ok(8000))
            .is_ok()
    );
    assert_eq!(
        a.decide(peer(), &auth, &envelope(&request(1000, 1)), || Ok(1000))
            .err()
            .unwrap()
            .code,
        "clock_rollback"
    );
    assert_eq!(
        a.decide(peer(), &auth, &envelope(&request(8000, 3)), || Ok(8000))
            .err()
            .unwrap()
            .status,
        503
    );
}

#[test]
fn cancellation_before_commit_discards_work_but_keeps_nonce() {
    let d = Directory::new();
    let (mut a, records) = authority(config(&d), false, true);
    let auth = format!("Bearer {}", a.credential());
    let calls = std::cell::Cell::new(0);
    let value = envelope(&request(1000, 1));
    let result = a.decide(peer(), &auth, &value, || {
        calls.set(calls.get() + 1);
        if calls.get() == 1 {
            Ok(1000)
        } else {
            Err(Rejection::new(503, "request_cancelled"))
        }
    });
    assert_eq!(result.err().unwrap().status, 503);
    assert!(records.lock().unwrap().is_empty());
    assert_eq!(call(&mut a, &request(1000, 1)).err().unwrap().status, 409);
}

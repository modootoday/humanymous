//! Owner decision #8 (issue #75 comment 7017558784): once a signed policy is
//! installed, served records carry the signed effective-policy version of
//! legacy internal/gate/configsign.go (`Server.configVersion()`) in place of
//! `scope.revision`, read at record time. Decisions themselves are unchanged.
use super::*;
use crate::gateway::settings::configsign::{BootPolicy, SignedPolicy};
use std::collections::BTreeMap;
use std::sync::RwLock;

const NOW_MS: u64 = 10_000;

fn decision_records(audit: &Audit) -> Vec<Record> {
    audit
        .records
        .lock()
        .unwrap()
        .iter()
        .filter(|record| record.event_type == "runtime.decision")
        .cloned()
        .collect()
}

fn decide_once(app: &mut Application, n: u64) -> Decision {
    let cfg = config();
    let issued = issue(app, NOW_MS);
    let mut req = request("POST", "/effect?exact=1", b"original bytes".to_vec());
    sign(
        &cfg,
        &mut req,
        issued["session"].as_str().unwrap(),
        issued["key"].as_str().unwrap(),
        n,
        NOW_MS,
    );
    app.decide(&req, || Ok(NOW_MS)).unwrap()
}

fn boot() -> BootPolicy {
    BootPolicy {
        global_monitor: false,
        routes: BTreeMap::from([("/effect".to_owned(), "strict".to_owned())]),
        rate_window_nanos: 0,
        rate_soft: 0,
        rate_hard: 0,
        hmac_key: b"lane-j-signed-policy-key".to_vec(),
    }
}

#[test]
fn records_carry_the_signed_policy_version_once_installed() {
    // Without a signed policy the provisioned scope revision stays.
    let audit = Audit::default();
    let mut plain =
        Application::new(config(), Default::default(), Box::new(audit.clone())).unwrap();
    let unsigned = decide_once(&mut plain, 1);
    let records = decision_records(&audit);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].config_version, config().scope.revision);

    let audit = Audit::default();
    let mut app = Application::new(config(), Default::default(), Box::new(audit.clone())).unwrap();
    let kill = Arc::new(AtomicBool::new(false));
    let active = Arc::new(RwLock::new(None));
    let policy = SignedPolicy::new(boot(), kill.clone(), active);
    app.install_signed_policy(policy.clone());
    let signed = decide_once(&mut app, 1);
    // The decision itself does not depend on the label.
    assert_eq!(
        std::mem::discriminant(&signed),
        std::mem::discriminant(&unsigned)
    );
    let version = policy.version();
    assert!(
        version.starts_with("cfg-") && version.len() == 16,
        "{version}"
    );
    assert_eq!(version, boot().config_version(false, None));
    // A kill-switch flip is a different policy: the next record shows it.
    kill.store(true, Ordering::SeqCst);
    decide_once(&mut app, 1);
    let records = decision_records(&audit);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].config_version, version);
    assert_eq!(records[1].config_version, boot().config_version(true, None));
    assert_ne!(records[1].config_version, version);
    assert_eq!(
        (records[0].verdict.as_str(), records[0].risk_score),
        (records[1].verdict.as_str(), records[1].risk_score)
    );
}

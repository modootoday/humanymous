use super::*;
use crate::settings::OperationalSnapshot;

#[test]
fn operational_candidate_freezes_exact_parent_scope_and_serialized_bytes() {
    let config = config();
    let initial = OperationalSnapshot::initial(&config).unwrap();
    let expected=br#"{"protocol":"humanymous.operational/0.1","scope":{"installation":"install","tenant":"tenant","origin":"origin","revision":"r1","authority":"owned.test"},"node":"runtime-test","revision":0,"settings":{"session_capacity":16,"session_ttl_ms":60000,"timeout_ms":1000,"rate_per_second":1000,"session_rate_per_second":100}}"#;
    assert_eq!(initial.bytes().unwrap(), expected);
    let mut desired = initial.settings();
    desired.session_rate_per_second = 40;
    let prepared = initial.prepare(&config, 0, desired).unwrap();
    assert_eq!(prepared.parent_revision(), 0);
    assert_eq!(
        prepared.parent_commitment(),
        "a022446c62c485f1cc18cf7a51c7f32dc7a753c68a39a105fe3484c57b61e867"
    );
    assert_eq!(prepared.snapshot().revision(), 1);
    assert_eq!(prepared.bytes(), prepared.snapshot().bytes().unwrap());
    assert_ne!(prepared.parent_commitment(), prepared.commitment());
    assert_eq!(initial.revision(), 0);
    assert_eq!(initial.settings().session_rate_per_second, 100);
    assert_eq!(
        initial.prepare(&config, 1, desired).err().unwrap().code,
        "settings_parent_revision"
    );
}

#[test]
fn untrusted_snapshot_cannot_change_scope_scoring_or_overflow_revision() {
    let config = config();
    let initial = OperationalSnapshot::initial(&config).unwrap();
    let original: Value = serde_json::from_slice(&initial.bytes().unwrap()).unwrap();
    for field in ["installation", "tenant", "origin", "revision", "authority"] {
        let mut changed = original.clone();
        changed["scope"][field] = json!("different");
        assert_eq!(
            OperationalSnapshot::from_json(&serde_json::to_vec(&changed).unwrap(), &config)
                .err()
                .unwrap()
                .code,
            "settings_snapshot_binding"
        );
    }
    let mut changed = original.clone();
    changed["settings"]["weights"] = json!({});
    assert!(
        OperationalSnapshot::from_json(&serde_json::to_vec(&changed).unwrap(), &config).is_err()
    );
    let mut changed = original;
    changed["revision"] = json!(humanymous_integrity::request::browser::MAX_SAFE_INTEGER);
    let canonical: OperationalSnapshot = serde_json::from_value(changed).unwrap();
    let exhausted = OperationalSnapshot::from_json(&canonical.bytes().unwrap(), &config).unwrap();
    assert_eq!(
        exhausted
            .prepare(&config, exhausted.revision(), exhausted.settings())
            .err()
            .unwrap()
            .code,
        "settings_revision_exhausted"
    );
    assert!(OperationalSnapshot::from_json(&vec![b' '; 16385], &config).is_err());
    let spaced = format!(" {}", String::from_utf8(initial.bytes().unwrap()).unwrap());
    assert_eq!(
        OperationalSnapshot::from_json(spaced.as_bytes(), &config)
            .err()
            .unwrap()
            .code,
        "settings_snapshot_noncanonical"
    );
}

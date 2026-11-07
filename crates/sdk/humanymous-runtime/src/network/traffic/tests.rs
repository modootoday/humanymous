use super::*;
use std::time::Duration;
fn limits() -> Limits {
    Limits::new(2, 2, 256, 100_000).unwrap()
}
fn rec(engine: &str, ja4: &str, ua: &str) -> Record {
    Record {
        session_id: "s".into(),
        remote_addr: "1.2.3.4:100".into(),
        engine: engine.into(),
        ja4: ja4.into(),
        ua_hash: hash_ua(ua),
        proto: "2".into(),
        method: "GET".into(),
        ..Record::default()
    }
}
fn has(records: &[Record], id: &str) -> bool {
    check(records, limits())
        .unwrap()
        .signals
        .iter()
        .any(|s| s.id == id)
}
#[test]
fn oracle_engine_rotation() {
    assert!(has(
        &[
            rec("chrome", "a_b_c", "Chrome"),
            rec("go", "x_y_z", "Chrome")
        ],
        "l5.traffic.engine_rotation"
    ));
}
#[test]
fn oracle_ua_rotation() {
    assert!(has(
        &[
            rec("chrome", "a_b_c", "Chrome"),
            rec("chrome", "a_b_c", "Firefox")
        ],
        "l5.traffic.ua_rotation"
    ));
}
#[test]
fn oracle_stable_session_is_signal_free_but_not_healthy_provenance() {
    let r = rec("chrome", "a_b_c", "Chrome");
    let result = check(&[r.clone(), r], limits()).unwrap();
    assert!(result.signals.is_empty());
    assert_eq!(result.coverage, Coverage::UnqualifiedHistory);
}
#[test]
fn oracle_c_segment_permutation_tolerated() {
    assert!(!has(
        &[
            rec("chrome", "a_b_c", "Chrome"),
            rec("chrome", "a_b_d", "Chrome")
        ],
        "l5.traffic.ja4_rotation"
    ));
}
#[test]
fn oracle_backfill_is_address_only_and_snapshot_is_copied() {
    let mut log = Log::new(Config::new(Duration::from_secs(10), limits()).unwrap());
    let mut tls = rec("chrome", "a_b_c", "");
    tls.method.clear();
    assert!(tls.is_tls());
    log.record_tls(tls).unwrap();
    let http = rec("", "", "Chrome");
    assert!(http.is_http());
    assert_eq!(log.record_http(http).unwrap(), Association::AddressOnly);
    let mut copy = log.session("s");
    assert_eq!(copy[0].engine, "chrome");
    copy[0].engine.clear();
    assert_eq!(log.session("s")[0].engine, "chrome");
}
#[test]
fn oracle_order_and_extension_permutation_stay_disabled() {
    let mut records = Vec::new();
    for name in ["nav", "cors", "no-cors", "other"] {
        let mut r = rec("chrome", "a_b_c", "Chrome");
        r.header_order = name.into();
        r.ext_order = "static".into();
        records.push(r);
    }
    assert!(check(&records, limits()).unwrap().signals.is_empty());
}
#[test]
fn oracle_subnet_and_hash_transforms_retain_text_quirks() {
    assert_eq!(subnet_key("192.168.5.9:443"), "192.168.5");
    assert_eq!(subnet_key("[::1]:443"), "::1]");
    assert_eq!(subnet_key("not-an-ip"), "not-an-ip");
    assert_eq!(hash_ua(""), "");
    assert_eq!(hash_order(&[]), "");
    assert_eq!(hash_ua("abc"), "ba7816bf8f01");
    assert_ne!(hash_order(&["a", "b"]), hash_order(&["b", "a"]));
}
#[test]
fn two_ja4_resumption_values_are_tolerated_but_three_rotate() {
    let a = rec("chrome", "a_b_1", "Chrome");
    let b = rec("chrome", "x_y_2", "Chrome");
    assert!(!has(&[a.clone(), b.clone()], "l5.traffic.ja4_rotation"));
    assert!(has(
        &[a, b, rec("chrome", "p_q_3", "Chrome")],
        "l5.traffic.ja4_rotation"
    ));
}
#[test]
fn ring_ttl_reset_and_clock_boundaries() {
    let mut log = Log::new(Config::new(Duration::from_secs(10), limits()).unwrap());
    for i in 0..65 {
        let mut r = rec("chrome", "a_b_c", "Chrome");
        r.time = Duration::from_secs(i);
        log.record_http(r).unwrap();
    }
    assert_eq!(log.session("s").len(), 64);
    assert_eq!(log.session("s")[0].time, Duration::from_secs(1));
    log.gc(Duration::from_secs(74)).unwrap();
    assert_eq!(log.session("s").len(), 64);
    assert!(log.reset(Duration::from_secs(73)).is_err());
    log.gc(Duration::from_secs(75)).unwrap();
    assert!(log.session("s").is_empty());
    log.reset(Duration::from_secs(75)).unwrap();
}
#[test]
fn missing_fields_and_insufficient_history_never_report_healthy() {
    assert_eq!(
        check(&[], limits()).unwrap().coverage,
        Coverage::Insufficient
    );
    assert_eq!(
        check(&[Record::default(), Record::default()], limits())
            .unwrap()
            .coverage,
        Coverage::Insufficient
    );
    assert!(check(&vec![Record::default(); 65], limits()).is_err());
}
#[test]
fn budget_failure_preserves_prior_session() {
    let mut log = Log::new(
        Config::new(
            Duration::from_secs(10),
            Limits::new(1, 1, 256, 100_000).unwrap(),
        )
        .unwrap(),
    );
    log.record_http(rec("chrome", "a_b_c", "Chrome")).unwrap();
    let before = log.session("s");
    let mut other = rec("chrome", "a_b_c", "Chrome");
    other.session_id = "other".into();
    assert_eq!(
        log.record_http(other),
        Err(crate::network::history::Unavailable::Budget)
    );
    assert_eq!(log.session("s"), before);
}
#[test]
fn subnet_hop_and_exact_h1_h2_mix_remain_suspicious() {
    let a = rec("chrome", "a_b_c", "Chrome");
    let mut b = a.clone();
    b.remote_addr = "9.8.7.6:100".into();
    b.proto = "1.1".into();
    assert!(has(&[a.clone(), b.clone()], "l5.traffic.ip_hop"));
    assert!(has(&[a, b], "l5.traffic.proto_downgrade"));
}

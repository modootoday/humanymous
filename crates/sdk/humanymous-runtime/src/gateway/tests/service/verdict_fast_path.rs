//! Owner decision #4 (option 1) in the shipped composition: the server-side
//! verdict fast path is installed only from a configured `verdict_fast_path`
//! object, reported in readiness, and refused bounds stop startup.
use super::*;

#[tokio::test]
async fn unix_verdict_fast_path_is_reported_and_refused_bounds_stop_startup() {
    let fixture = PrivateFixture::new();
    let mut config = fixture.configuration();
    config.verdict_fast_path =
        Some(serde_json::from_value(serde_json::json!({"max_sessions":1024})).unwrap());
    let host = ServiceHost::bind(config).await.unwrap();
    let ready = serde_json::to_value(host.readiness()).unwrap();
    assert_eq!(
        ready["profile"]["verdict_fast_path"],
        serde_json::json!({"mode":"enforce","ttl_ms":1_800_000,"max_sessions":1024})
    );
    drop(host);

    let fixture = PrivateFixture::new();
    let host = ServiceHost::bind(fixture.configuration()).await.unwrap();
    let ready = serde_json::to_value(host.readiness()).unwrap();
    assert!(ready["profile"].get("verdict_fast_path").is_none());
    drop(host);

    let fixture = PrivateFixture::new();
    let mut config = fixture.configuration();
    config.verdict_fast_path =
        Some(serde_json::from_value(serde_json::json!({"max_sessions":1,"ttl_ms":0})).unwrap());
    assert!(config.validate().is_err());
    assert!(ServiceHost::bind(config).await.is_err());
}

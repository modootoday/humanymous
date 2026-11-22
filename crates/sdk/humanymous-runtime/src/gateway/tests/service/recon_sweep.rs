//! Owner decision #2 in the shipped composition: the decision-probing sweep
//! gate (Go `NewSweepDetector`, HR-30) is installed only from a configured
//! `recon_sweep` object, reported in readiness, and refused bounds stop
//! startup.
use super::*;
use crate::gateway::configuration::recon_sweep::ReconSweep;

fn sweep(value: serde_json::Value) -> Option<ReconSweep> {
    Some(serde_json::from_value(value).unwrap())
}

#[tokio::test]
async fn unix_configured_recon_sweep_is_reported_in_readiness() {
    let fixture = PrivateFixture::new();
    let mut config = fixture.configuration();
    config.recon_sweep = sweep(serde_json::json!({"mode":"monitor","max_bindings":4096}));
    let host = ServiceHost::bind(config).await.unwrap();
    let ready = serde_json::to_value(host.readiness()).unwrap();
    assert_eq!(
        ready["profile"]["recon_sweep"],
        serde_json::json!({"mode":"monitor","window_ms":30000,"max_sessions":8,
            "max_bindings":4096})
    );
    drop(host);

    // Without the object nothing is installed or reported.
    let fixture = PrivateFixture::new();
    let host = ServiceHost::bind(fixture.configuration()).await.unwrap();
    let ready = serde_json::to_value(host.readiness()).unwrap();
    assert!(ready["profile"].get("recon_sweep").is_none());
}

#[tokio::test]
async fn unix_refused_recon_sweep_bounds_stop_startup() {
    for bad in [
        serde_json::json!({"max_bindings":0}),
        serde_json::json!({"max_bindings":1,"window_ms":0}),
        serde_json::json!({"max_bindings":1,"max_sessions":0}),
    ] {
        let fixture = PrivateFixture::new();
        let mut config = fixture.configuration();
        config.recon_sweep = sweep(bad.clone());
        assert!(config.validate().is_err(), "{bad}");
        assert!(ServiceHost::bind(config).await.is_err(), "{bad}");
    }
}

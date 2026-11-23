//! Owner decision #7 in the shipped composition: loader injection is off
//! unless an `inject` object lists inventoried GET routes; it is reported in
//! readiness, and a refused object stops startup.
use super::*;

fn with_page(config: &mut Configuration) {
    config.routes.push(
        serde_json::from_value(serde_json::json!({"method":"GET","path":"/page",
            "required":{"raw_tls":false,"original_h2":false,"raw_header_order":true,
            "collector":false,"rit":true}}))
        .unwrap(),
    );
}

#[tokio::test]
async fn unix_configured_inject_is_reported_and_refused_routes_stop_startup() {
    let fixture = PrivateFixture::new();
    let mut config = fixture.configuration();
    with_page(&mut config);
    config.inject = Some(
        serde_json::from_value(serde_json::json!({"mode":"monitor","routes":["/page"],
            "loader_src":"/static/loader.js"}))
        .unwrap(),
    );
    let host = ServiceHost::bind(config).await.unwrap();
    let ready = serde_json::to_value(host.readiness()).unwrap();
    assert_eq!(
        ready["profile"]["inject"],
        serde_json::json!({"mode":"monitor","routes":1,"loader_src":"/static/loader.js"})
    );
    drop(host);

    let fixture = PrivateFixture::new();
    let host = ServiceHost::bind(fixture.configuration()).await.unwrap();
    let ready = serde_json::to_value(host.readiness()).unwrap();
    assert!(ready["profile"].get("inject").is_none());
    drop(host);

    // A listed route outside the inventory refuses startup.
    let fixture = PrivateFixture::new();
    let mut config = fixture.configuration();
    config.inject = Some(
        serde_json::from_value(
            serde_json::json!({"routes":["/page"],"loader_src":"/static/loader.js"}),
        )
        .unwrap(),
    );
    assert!(config.validate().is_err());
    assert!(ServiceHost::bind(config).await.is_err());
}

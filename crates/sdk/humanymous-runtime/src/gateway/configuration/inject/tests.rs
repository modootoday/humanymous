use super::*;
use serde_json::json;

fn route(method: &str, path: &str, stream: bool) -> Route {
    serde_json::from_value(
        json!({"method":method,"path":path,"required":{"raw_tls":false,
        "original_h2":false,"raw_header_order":true,"collector":false,"rit":true},
        "response_profile":if stream {"native_stream"} else {"buffered"}}),
    )
    .unwrap()
}

fn inject(value: serde_json::Value) -> Inject {
    serde_json::from_value(value).unwrap()
}

/// Owner decision #7: listed routes must be inventoried buffered GET routes
/// outside the reserved prefix, and the loader a bounded same-origin path.
#[test]
fn inject_routes_and_loader_are_validated() {
    let inventory = [
        route("GET", "/", false),
        route("GET", "/app", false),
        route("POST", "/form", false),
        route("GET", "/feed", true),
    ];
    let ok = inject(json!({"routes":["/","/app"],"loader_src":"/static/hmn/loader-1.0.js"}));
    assert!(ok.valid(&inventory));
    assert_eq!(ok.mode, Mode::Enforce);
    assert!(ok.covers("GET", "/app?x=1"));
    assert!(!ok.covers("HEAD", "/app") && !ok.covers("GET", "/other"));
    assert_eq!(
        ok.snippet(),
        b"<!--hmn-injected--><script src=\"/static/hmn/loader-1.0.js\" defer></script>"
    );
    assert_eq!(
        ok.readiness(),
        json!({"mode":"enforce","routes":2,"loader_src":"/static/hmn/loader-1.0.js"})
    );
    for bad in [
        json!({"routes":[],"loader_src":"/l.js"}),
        json!({"routes":["/","/"],"loader_src":"/l.js"}),
        json!({"routes":["/missing"],"loader_src":"/l.js"}),
        json!({"routes":["/form"],"loader_src":"/l.js"}),
        json!({"routes":["/feed"],"loader_src":"/l.js"}),
        json!({"routes":["/"],"loader_src":"l.js"}),
        json!({"routes":["/"],"loader_src":"//cdn.example/l.js"}),
        json!({"routes":["/"],"loader_src":"/l.js\" onload=\"x"}),
        json!({"routes":["/"],"loader_src":"/_humanymous/loader.js"}),
        json!({"routes":["/"],"loader_src":format!("/{}", "a".repeat(MAX_LOADER_SRC))}),
    ] {
        assert!(!inject(bad.clone()).valid(&inventory), "{bad}");
    }
    let many: Vec<String> = (0..=MAX_ROUTES).map(|n| format!("/{n}")).collect();
    let wide: Vec<Route> = many.iter().map(|path| route("GET", path, false)).collect();
    assert!(!inject(json!({"routes":many,"loader_src":"/l.js"})).valid(&wide));
    assert!(serde_json::from_value::<Inject>(json!({"routes":["/"]})).is_err());
    assert!(
        serde_json::from_value::<Inject>(json!({"routes":["/"],"loader_src":"/l.js","x":1}))
            .is_err()
    );
}

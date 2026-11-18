//! Ports of legacy cmd/gate/routes_test.go, internal/gate/resolve_test.go and
//! configsign_test.go at baseline 981d25c.
use super::*;

fn table(pairs: &[(&str, &str)]) -> RouteTable {
    RouteTable {
        routes: pairs
            .iter()
            .map(|(prefix, preset)| ((*prefix).to_owned(), (*preset).to_owned()))
            .collect(),
        global_monitor: false,
    }
}

#[test]
fn load_routes_valid() {
    let routes = parse_routes(
        "# my routes\n/login    strict\n/checkout strict\n\n/health   off\n/api      balanced\n",
    )
    .unwrap();
    assert_eq!(
        routes,
        table(&[
            ("/login", "strict"),
            ("/checkout", "strict"),
            ("/health", "off"),
            ("/api", "balanced"),
        ])
        .routes
    );
}

#[test]
fn load_routes_unknown_preset() {
    assert!(matches!(
        parse_routes("/login supersecure\n"),
        Err(RoutesError::UnknownPreset { line: 1, .. })
    ));
}

#[test]
fn load_routes_attested_valid_and_catch_all_refused() {
    let routes = parse_routes("/transfer attested\n/checkout attested\n").unwrap();
    assert_eq!(routes["/transfer"], "attested");
    assert!(matches!(
        parse_routes("/ attested\n"),
        Err(RoutesError::AttestedCatchAll { .. })
    ));
}

#[test]
fn validate_attested_route_preconditions() {
    let attested = table(&[("/transfer", "attested"), ("/browse", "balanced")]).routes;
    assert!(matches!(
        validate_attested_routes(&attested, false, true),
        Err(RoutesError::AttestedWithoutSharedKey { .. })
    ));
    assert_eq!(
        validate_attested_routes(&attested, true, false)
            .unwrap()
            .len(),
        1
    );
    assert!(
        validate_attested_routes(&attested, true, true)
            .unwrap()
            .is_empty()
    );
    let plain = table(&[("/x", "balanced")]).routes;
    assert!(
        validate_attested_routes(&plain, false, false)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn load_routes_malformed() {
    assert!(matches!(
        parse_routes("/login strict extra\n"),
        Err(RoutesError::Malformed { line: 1, .. })
    ));
}

#[test]
fn load_routes_missing_file() {
    let missing = std::env::temp_dir().join(format!(
        "humanymous-routes-missing-{}-{:?}.conf",
        std::process::id(),
        std::thread::current().id()
    ));
    assert!(matches!(load_routes(&missing), Err(LoadError::Read(_))));
}

#[test]
fn resolve_path_confusion_selects_strict() {
    let routes = table(&[("/admin", "strict"), ("/health", "off")]);
    for path in [
        "/admin",
        "/admin/",
        "/admin/users",
        "/./admin",
        "//admin",
        "/admin/../admin",
        "/foo/../admin",
    ] {
        assert_eq!(routes.resolve(path).name, "strict", "{path}");
    }
    assert_ne!(routes.resolve("/healthstatus-secret").name, "off");
    assert_eq!(routes.resolve("/health").name, "off");
    assert_eq!(routes.resolve("/store/item").name, "balanced");
}

#[test]
fn resolve_longest_prefix_presets_and_global_monitor() {
    let mut routes = table(&[
        ("/", "monitor"),
        ("/api", "strict"),
        ("/api/pay", "attested"),
    ]);
    assert_eq!(routes.resolve("/api/pay/now"), ATTESTED);
    assert_eq!(routes.resolve("/api/list"), STRICT);
    assert_eq!(routes.resolve("/static/app.js"), MONITOR);
    assert_eq!(routes.resolve(""), MONITOR);
    routes.global_monitor = true;
    let policy = routes.resolve("/api/pay");
    assert!(!policy.enforce && policy.attest_floor && policy.name == "attested");
    assert_eq!(preset_by_name("anything"), BALANCED);
    assert_eq!(preset_by_name("off"), OFF);
    assert_eq!(
        (
            OFF.inject,
            OFF.enforce,
            ATTESTED.fail_closed,
            STRICT.sync_score
        ),
        (false, false, true, true)
    );
}

#[test]
fn normalize_match_path_cleans_lexically() {
    for (raw, want) in [
        ("", "/"),
        ("admin", "/admin"),
        ("/a/./b/../c/", "/a/c"),
        ("/../..", "/"),
        ("//x//y", "/x/y"),
    ] {
        assert_eq!(normalize_match_path(raw), want, "{raw}");
    }
}

#[test]
fn config_version_is_signed_stable_and_tracks_the_effective_policy() {
    let key = [b'k'; 32];
    let routes = table(&[("/admin", "strict"), ("/health", "off")]);
    let mut policy = EffectivePolicy {
        routes: &routes,
        kill_switch: false,
        rate_window: DEFAULT_RATE_WINDOW,
        rate_soft: DEFAULT_RATE_SOFT,
        rate_hard: DEFAULT_RATE_HARD,
    };
    let first = config_version(&key, &policy);
    // Known answer for HMAC-SHA256 over the Go-formatted policy string
    // "monitor=false|kill=false|/admin=strict|/health=off|rl=10/60/120".
    assert_eq!(first, "cfg-694cb55271dd");
    assert_eq!(config_version(&key, &policy), first);
    policy.kill_switch = true;
    assert_ne!(config_version(&key, &policy), first);
    policy.kill_switch = false;
    assert_ne!(config_version(&[b'x'; 32], &policy), first);
}

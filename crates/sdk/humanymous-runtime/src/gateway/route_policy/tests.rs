//! One-to-one ports of the oracle route-policy tests at 981d25c:
//! internal/gate resolve_test.go, ws5_test.go (`TestFailClosedActionMapping`),
//! wargame_escalation_r81_test.go (R081-R085), wargame_escalation_r161_test.go
//! (R171-R173) and cmd/gate routes_test.go, plus the differential golden
//! parity in `parity`.
use super::*;
use std::path::PathBuf;

mod parity;

fn table(entries: &[(&str, &str)]) -> RouteTable {
    RouteTable {
        routes: entries
            .iter()
            .map(|(prefix, preset)| ((*prefix).to_owned(), (*preset).to_owned()))
            .collect(),
        global_monitor: false,
    }
}

pub(super) struct TempDir(PathBuf);
impl TempDir {
    pub(super) fn new(tag: &str) -> Self {
        let mut nonce = [0_u8; 8];
        getrandom::fill(&mut nonce).unwrap();
        let path = std::env::temp_dir().join(format!("hmn-routes-{tag}-{}", hex::encode(nonce)));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub(super) fn write(&self, body: &[u8]) -> PathBuf {
        let path = self.0.join("routes.conf");
        std::fs::write(&path, body).unwrap();
        path
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn load(body: &str) -> Result<BTreeMap<String, String>, RoutesError> {
    let directory = TempDir::new("case");
    load_routes(&directory.write(body.as_bytes()))
}

// resolve_test.go

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
        assert_eq!(routes.resolve(path).name, "strict", "resolve({path:?})");
    }
    assert_ne!(routes.resolve("/healthstatus-secret").name, "off");
    assert_eq!(routes.resolve("/health").name, "off");
    assert_eq!(routes.resolve("/store/item").name, "balanced");
}

// ws5_test.go

/// `TestFailClosedActionMapping`. Case 0 is the owner-accepted difference:
/// Go passed an `Unknown` verdict on a safe balanced route; Rust challenges.
#[test]
fn fail_closed_action_mapping() {
    let cases = [
        (
            Verdict::Unknown,
            PRESET_BALANCED,
            false,
            "pass",
            "challenge_pow",
        ),
        (
            Verdict::Unknown,
            PRESET_BALANCED,
            true,
            "challenge_pow",
            "challenge_pow",
        ),
        (
            Verdict::Unknown,
            PRESET_STRICT,
            false,
            "challenge_pow",
            "challenge_pow",
        ),
        (Verdict::Allow, PRESET_BALANCED, true, "pass", "pass"),
        (Verdict::Deny, PRESET_BALANCED, false, "block", "block"),
    ];
    for (index, (verdict, route, unsafe_method, go, rust)) in cases.into_iter().enumerate() {
        let _ = route; // Go's route input no longer changes the Rust result.
        let got = edge_action(verdict, unsafe_method).action;
        assert_eq!(got, rust, "case {index}");
        assert_eq!(
            go == rust,
            index != 0,
            "case {index}: only case 0 differs from Go"
        );
    }
    assert_eq!(
        edge_action(Verdict::Unknown, true).event_type,
        EVENT_FAILCLOSED_MUTATING
    );
}

// wargame_escalation_r81_test.go

#[test]
fn wargame_r081_path_dot_dot_admin_stays_strict() {
    let routes = table(&[("/admin", "strict")]);
    assert_eq!(routes.resolve("/foo/../admin/secret").name, "strict");
}

#[test]
fn wargame_r082_path_encoded_traversal_normalized() {
    let routes = table(&[("/admin", "strict")]);
    assert_eq!(routes.resolve("//admin").name, "strict");
}

#[test]
fn wargame_r083_path_prefix_not_confused_with_off_route() {
    let routes = table(&[("/health", "off")]);
    assert_ne!(routes.resolve("/healthz").name, "off");
}

#[test]
fn wargame_r084_path_case_sensitive_strict() {
    let routes = table(&[("/Admin", "strict"), ("/admin", "balanced")]);
    assert_eq!(routes.resolve("/admin").name, "balanced");
}

#[test]
fn wargame_r085_default_balanced_for_unknown() {
    let routes = table(&[("/login", "strict")]);
    assert_eq!(routes.resolve("/api/v2/widget").name, "balanced");
}

// wargame_escalation_r161_test.go

#[test]
fn wargame_r171_strict_path_login() {
    assert_eq!(
        table(&[("/login", "strict")]).resolve("/login").name,
        "strict"
    );
}

#[test]
fn wargame_r172_nested_strict_under_admin() {
    let routes = table(&[("/admin", "strict")]);
    assert_eq!(routes.resolve("/admin/settings/users").name, "strict");
}

/// Go leaves the `/metrics/public` outcome unasserted; segment matching
/// makes it `off` in both implementations.
#[test]
fn wargame_r173_off_route_exact_only() {
    let routes = table(&[("/metrics", "off")]);
    assert_eq!(routes.resolve("/metrics").name, "off");
    assert_eq!(routes.resolve("/metrics/public").name, "off");
}

// cmd/gate routes_test.go

#[test]
fn load_routes_valid() {
    let routes = load(
        "# my routes\n/login    strict\n/checkout strict\n\n/health   off\n/api      balanced\n",
    )
    .unwrap();
    let want: BTreeMap<String, String> = [
        ("/login", "strict"),
        ("/checkout", "strict"),
        ("/health", "off"),
        ("/api", "balanced"),
    ]
    .into_iter()
    .map(|(prefix, preset)| (prefix.to_owned(), preset.to_owned()))
    .collect();
    assert_eq!(routes, want);
}

#[test]
fn load_routes_unknown_preset() {
    assert!(matches!(
        load("/login supersecure\n"),
        Err(RoutesError::UnknownPreset { .. })
    ));
}

#[test]
fn load_routes_attested_valid_and_catch_all_refused() {
    let routes = load("/transfer attested\n/checkout attested\n").unwrap();
    assert_eq!(routes["/transfer"], "attested");
    assert!(matches!(
        load("/ attested\n"),
        Err(RoutesError::AttestedCatchAll { .. })
    ));
}

#[test]
fn validate_attested_routes_preconditions() {
    let attested = table(&[("/transfer", "attested"), ("/browse", "balanced")]).routes;
    assert!(validate_attested_routes(&attested, false, true).is_err());
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
    let balanced = table(&[("/x", "balanced")]).routes;
    assert!(validate_attested_routes(&balanced, false, false).is_ok());
}

#[test]
fn load_routes_malformed() {
    assert!(matches!(
        load("/login strict extra\n"),
        Err(RoutesError::Malformed { .. })
    ));
}

#[test]
fn load_routes_missing_file() {
    let directory = TempDir::new("missing");
    assert!(matches!(
        load_routes(&directory.0.join("nope.conf")),
        Err(RoutesError::Io(_))
    ));
}

use super::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use humanymous_authority::startup::authorization::{
    AttemptData, AudienceRole, ExpectedAuthorizationData, InstallerData, PublisherData, RoleData,
    ScopeData, parse_bundle, verify_bundle,
};
use serde_json::Value;

// Consume Authority's published signed vector, never manufacture a native proof
// or introduce a second signing/commitment implementation in Runtime tests.
fn fixture() -> SignatureCheckedAuthorization {
    let v: Value = serde_json::from_str(include_str!(
        "../../../../authority/tests/fixtures/authorization-v2-minimal.json"
    ))
    .unwrap();
    let e = &v["expected"];
    let i = &e["installer"];
    let p = &e["publisher"];
    let a = &e["attempt"];
    let s = &e["scope"];
    let expected = ExpectedAuthorizationData {
        installer: InstallerData {
            public_key: bytes(i, "public_key"),
            sid: hex::decode(text(i, "sid")).unwrap(),
            installation_epoch: bytes(i, "installation_epoch"),
            delegation_commitment: bytes(i, "delegation_commitment"),
            delegation_id: bytes(i, "delegation_id"),
            not_before_ms: number(i, "not_before_ms"),
            not_after_ms: number(i, "not_after_ms"),
        },
        publisher: PublisherData {
            name: text(p, "name"),
            sid: hex::decode(text(p, "sid")).unwrap(),
            public_key: bytes(p, "public_key"),
            artifact: bytes(p, "artifact"),
            pid: number(p, "pid").try_into().unwrap(),
            creation: number(p, "creation"),
            epoch: bytes(p, "epoch"),
        },
        service: role(&e["service"], AudienceRole::Service),
        observer: role(&e["observer"], AudienceRole::Observer),
        scope: ScopeData {
            installation: text(s, "installation"),
            tenant: text(s, "tenant"),
            origin: text(s, "origin"),
            revision: text(s, "revision"),
            application_authority: text(s, "application_authority"),
        },
        attempt: AttemptData {
            grant_id: bytes(a, "grant_id"),
            credential_epoch: bytes(a, "credential_epoch"),
            startup_cap_ms: number(a, "startup_cap_ms").try_into().unwrap(),
            earliest_issued_ms: number(a, "earliest_issued_ms"),
            latest_not_after_ms: number(a, "latest_not_after_ms"),
        },
    };
    let artifacts = ["delegation", "prepared", "commit"]
        .map(|key| URL_SAFE_NO_PAD.decode(text(&v, key)).unwrap());
    verify_bundle(
        parse_bundle(&artifacts[0], &artifacts[1], &artifacts[2]).unwrap(),
        &expected,
        number(&v, "now_ms"),
    )
    .unwrap()
}

fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap().into()
}
fn number(v: &Value, key: &str) -> u64 {
    v[key].as_u64().unwrap()
}
fn bytes<const N: usize>(v: &Value, key: &str) -> [u8; N] {
    hex::decode(text(v, key)).unwrap().try_into().unwrap()
}
fn role(v: &Value, role: AudienceRole) -> RoleData {
    RoleData {
        role,
        name: text(v, "name"),
        sid: hex::decode(text(v, "sid")).unwrap(),
        pid: number(v, "pid").try_into().unwrap(),
        creation: number(v, "creation"),
        epoch: bytes(v, "epoch"),
        base_plan: bytes(v, "base_plan"),
        artifact: bytes(v, "artifact"),
    }
}
fn configuration() -> Configuration {
    let mut c = crate::tests::config();
    c.scope.installation = "i".into();
    c.scope.tenant = "t".into();
    c.scope.origin = "o".into();
    c.scope.revision = "r".into();
    c.scope.authority = "example.test".into();
    c.audit.keys.passphrase.clear();
    c.audit.require_external_witness = true;
    c
}

#[test]
fn checked_fixture_retains_same_configuration_and_public_data() {
    let configuration = configuration();
    let routes = configuration.routes.as_ptr();
    let authorization = fixture();
    let service_name = authorization.service().name.as_ptr();
    let data = AuthorizationData::bind(configuration, authorization).unwrap();
    assert_eq!(data.configuration().routes.as_ptr(), routes);
    assert_eq!(data.authorization().service().name.as_ptr(), service_name);
    assert_eq!(data.authorization().credential_epoch(), &[10; 16]);
    let (configuration, authorization) = data.into_parts();
    assert_eq!(configuration.routes.as_ptr(), routes);
    assert_eq!(authorization.service().name.as_ptr(), service_name);
    assert!(configuration.audit.keys.passphrase.is_empty());
    assert!(configuration.audit.require_external_witness);
}

#[test]
fn every_scope_component_and_authority_case_or_port_mismatch_refuses() {
    for field in 0..7 {
        let mut c = configuration();
        match field {
            0 => c.scope.installation.push('x'),
            1 => c.scope.tenant.push('x'),
            2 => c.scope.origin.push('x'),
            3 => c.scope.revision.push('x'),
            4 => c.scope.authority.push('x'),
            5 => c.scope.authority = "EXAMPLE.test".into(),
            _ => c.scope.authority.push_str(":443"),
        }
        assert!(matches!(
            AuthorizationData::bind(c, fixture()),
            Err(AuthorizationDataError::Scope)
        ));
    }
}

#[test]
fn valid_signatures_cannot_bypass_invalid_runtime_route_configuration() {
    let mut c = configuration();
    c.routes[0].required.rit = false;
    assert!(matches!(
        AuthorizationData::bind(c, fixture()),
        Err(AuthorizationDataError::Configuration)
    ));
}

#[test]
fn pre_admission_secret_is_rejected_without_unseal() {
    let mut c = configuration();
    c.audit.keys.passphrase.push_str("must-not-be-unsealed");
    assert!(matches!(
        AuthorizationData::bind(c, fixture()),
        Err(AuthorizationDataError::Secret)
    ));
}

#[test]
fn observer_data_profile_refuses_writer_only_configuration() {
    let mut c = configuration();
    c.audit.require_external_witness = false;
    assert!(matches!(
        AuthorizationData::bind(c, fixture()),
        Err(AuthorizationDataError::RequiredWitness)
    ));
}

#[test]
fn successful_data_binding_cannot_open_native_preparation() {
    let data = AuthorizationData::bind(configuration(), fixture()).unwrap();
    assert_eq!(data.authorization().startup_cap_ms(), 1000);
    assert!(matches!(
        humanymous_authority::startup::observer::native_preparation(),
        Err(humanymous_authority::startup::observer::NativePreparationUnavailable::Unsupported)
    ));
}

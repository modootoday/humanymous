//! Actual hosted Waitress/Flask/Django HTTP pilot; no authored Python source.
#![cfg(all(feature = "host-pilot", unix))]
#[path = "../../framework-adapter/tests/support/authority.rs"]
mod authority_fixture;
#[path = "support/framework_server_cases.rs"]
mod cases;
#[path = "support/component_lifecycle.rs"]
mod component_lifecycle;
#[path = "../../node-adapter/tests/support/http.rs"]
mod http;
#[path = "support/wsgi_authority.rs"]
mod wsgi_authority;
#[path = "support/wsgi_host.rs"]
mod wsgi_host;
fn suite() -> cases::Suite {
    cases::Suite {
        frameworks: &["flask", "django"],
        host: |directory, framework, endpoint, credential| {
            wsgi_host::host(directory, framework, endpoint, credential, false)
        },
        asgi: false,
    }
}
#[test]
#[ignore = "mandatory actual hosted Waitress Flask/Django plus production authority process"]
fn production_authority_nonallow_is_audited_before_origin() {
    cases::production_authority_nonallow_is_audited_before_origin(suite());
}
#[test]
#[ignore = "mandatory actual hosted Waitress Flask/Django with real engine synthetic evidence and durable audit"]
fn actual_allow_preserves_binary_and_two_cookies() {
    cases::actual_allow_preserves_binary_and_two_cookies(suite());
}
#[test]
#[ignore = "mandatory actual hosted Waitress Flask/Django dependency-down and rejected-route requests"]
fn dependency_down_and_invalid_routes_never_reach_origin() {
    cases::dependency_down_and_invalid_routes_never_reach_origin(suite());
}
#[test]
#[ignore = "mandatory actual hosted Waitress Flask/Django body abort and server input deadline with available authority"]
fn incomplete_bodies_never_reach_available_authority_or_origin() {
    cases::incomplete_bodies_never_reach_available_authority_or_origin(suite());
}
#[test]
#[ignore = "mandatory actual hosted Waitress Flask/Django disconnect before released real-engine ALLOW"]
fn disconnected_client_cannot_reach_origin_after_allow() {
    cases::disconnected_client_cannot_reach_origin_after_allow(suite());
}
#[test]
#[ignore = "mandatory actual hosted Waitress Flask/Django accepted IPC response timeout"]
fn authority_response_deadline_closes_without_origin() {
    cases::authority_response_deadline_closes_without_origin(suite());
}

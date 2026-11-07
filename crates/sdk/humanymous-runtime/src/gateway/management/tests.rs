use super::*;
fn handler() -> Handler {
    Handler {
        credential: Arc::new(Credential::generate().unwrap()),
        peer: NativePeer::Unix(123),
        scope: ManagementScope {
            installation: "installation".into(),
            tenant: "tenant".into(),
            origin: "origin".into(),
            revision: "r1".into(),
            authority: "owned.test".into(),
        },
        profile: Arc::new(RwLock::new(serde_json::json!({"product_qualified":false}))),
        operator: None,
    }
}
fn request(handler: &Handler) -> LocalRequest {
    LocalRequest {
        method: "POST".into(),
        target: "/management/v0.1/status".into(),
        headers: vec![
            ("Host".into(), "humanymous.management.local".into()),
            ("Content-Type".into(), "application/json".into()),
        ],
        authorization: format!("Bearer {}", handler.credential.expose_for_provisioning()),
        body: serde_json::to_vec(&handler.scope).unwrap(),
    }
}
#[test]
fn observer_requires_native_peer_independent_credential_exact_scope() {
    let handler = handler();
    let mut request = request(&handler);
    assert!(handler.authorize(&request, NativePeer::Unix(123)).is_ok());
    assert_eq!(
        handler
            .authorize(&request, NativePeer::Unix(124))
            .unwrap_err()
            .code,
        "management_peer"
    );
    request.authorization = "Bearer session-capability".into();
    assert_eq!(
        handler
            .authorize(&request, NativePeer::Unix(123))
            .unwrap_err()
            .status,
        401
    );
    request.authorization = format!("Bearer {}", handler.credential.expose_for_provisioning());
    let mut value: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    value["revision"] = serde_json::json!("stale");
    request.body = serde_json::to_vec(&value).unwrap();
    assert_eq!(
        handler
            .authorize(&request, NativePeer::Unix(123))
            .unwrap_err()
            .code,
        "management_scope"
    );
}
#[test]
fn observer_refuses_browser_authority_ambiguous_headers_and_mutations() {
    let handler = handler();
    for name in [
        "Origin",
        "Cookie",
        "Referer",
        "Sec-Fetch-Site",
        "X-Humanymous-Session",
        "Proxy-Authorization",
    ] {
        let mut request = request(&handler);
        request.headers.push((name.into(), "fixture".into()));
        assert_eq!(
            handler
                .authorize(&request, NativePeer::Unix(123))
                .unwrap_err()
                .code,
            "management_native_only"
        );
    }
    for target in [
        "/management/v0.1/kill",
        "/management/v0.1/status?x=1",
        "/adapter/v0.1/decision",
    ] {
        let mut request = request(&handler);
        request.target = target.into();
        assert_eq!(
            handler
                .authorize(&request, NativePeer::Unix(123))
                .unwrap_err()
                .status,
            403
        );
    }
    let mut request = request(&handler);
    request
        .headers
        .push(("host".into(), "humanymous.management.local".into()));
    assert_eq!(
        handler
            .authorize(&request, NativePeer::Unix(123))
            .unwrap_err()
            .code,
        "management_duplicate_header"
    );
}

#[test]
fn settings_view_is_observer_only_and_cannot_be_used_for_mutation() {
    let handler = handler();
    let mut request = request(&handler);
    request.target = "/management/v0.1/settings".into();
    assert!(handler.authorize(&request, NativePeer::Unix(123)).is_ok());
    for method in ["PATCH", "PUT", "DELETE"] {
        request.method = method.into();
        assert_eq!(
            handler
                .authorize(&request, NativePeer::Unix(123))
                .unwrap_err()
                .status,
            403
        );
    }
}

#[test]
fn operator_resource_change_requires_its_own_credential_scope_and_native_envelope() {
    let handler = handler();
    let operator = Credential::generate().unwrap();
    let duplicate = OperatorPolicy {
        credential: Credential::from_private_provisioning(zeroize::Zeroizing::new(
            handler.credential.expose_for_provisioning().to_owned(),
        ))
        .unwrap(),
    };
    assert!(duplicate.validate_observer(&handler.credential).is_err());
    let mut change = request(&handler);
    change.target = "/management/v0.1/settings/commit".into();
    let mut body = serde_json::json!({
        "scope":handler.scope,"expected_revision":0,
        "settings":crate::gateway::tests::config().operational()
    });
    change.body = serde_json::to_vec(&body).unwrap();
    assert_eq!(
        handler
            .authorize_change(&change, NativePeer::Unix(123), &operator)
            .err()
            .unwrap()
            .status,
        401
    );
    change.authorization = format!("Bearer {}", operator.expose_for_provisioning());
    assert!(
        handler
            .authorize_change(&change, NativePeer::Unix(123), &operator)
            .is_ok()
    );
    assert_eq!(
        handler
            .authorize_change(&change, NativePeer::Unix(124), &operator)
            .err()
            .unwrap()
            .status,
        403
    );
    change
        .headers
        .push(("Origin".into(), "http://owned.test".into()));
    assert_eq!(
        handler
            .authorize_change(&change, NativePeer::Unix(123), &operator)
            .err()
            .unwrap()
            .status,
        403
    );
    change.headers.pop();
    body["scope"]["tenant"] = serde_json::json!("foreign");
    change.body = serde_json::to_vec(&body).unwrap();
    assert_eq!(
        handler
            .authorize_change(&change, NativePeer::Unix(123), &operator)
            .err()
            .unwrap()
            .status,
        403
    );
    body["scope"]["tenant"] = serde_json::json!(handler.scope.tenant);
    body["settings"]["threshold"] = serde_json::json!(0);
    change.body = serde_json::to_vec(&body).unwrap();
    assert_eq!(
        handler
            .authorize_change(&change, NativePeer::Unix(123), &operator)
            .err()
            .unwrap()
            .status,
        400
    );
}

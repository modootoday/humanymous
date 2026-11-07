use super::*;
use serde_json::{Value, json};

fn scope() -> Scope {
    Scope {
        installation: "i".into(),
        tenant: "t".into(),
        origin: "o".into(),
        revision: "r".into(),
        authority: "example.test".into(),
    }
}
fn bootstrap() -> Value {
    json!({"protocol":browser::PROTOCOL,"session":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","key":"AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE","counter":0,"expires":9000,"installation":"i","tenant":"t","origin":"o","revision":"r","authority":"example.test"})
}
fn session() -> Session {
    Session::from_json(
        scope(),
        &serde_json::to_vec(&bootstrap()).unwrap(),
        1000,
        2000,
    )
    .unwrap()
}
fn request() -> Request<'static> {
    Request {
        method: "POST",
        target: "/api?q=1",
        content_type: "application/json",
        authorization: "Bearer synthetic",
        body: b"{}",
    }
}
fn ack(signed: &Signed, status: u16) -> String {
    browser::acknowledgement(
        &[1; 32],
        signed.token(),
        signed.counter,
        signed.bucket,
        status,
    )
    .unwrap()
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn strict_bootstrap_rejects_missing_unknown_duplicate_fields_and_scope_changes() {
    let original = bootstrap();
    for field in original.as_object().unwrap().keys() {
        let mut bad = original.clone();
        bad.as_object_mut().unwrap().remove(field);
        assert!(
            Session::from_json(scope(), &serde_json::to_vec(&bad).unwrap(), 1000, 2000).is_err(),
            "{field}"
        );
    }
    for field in ["installation", "tenant", "origin", "revision", "authority"] {
        let mut bad = original.clone();
        bad[field] = json!("other");
        assert!(matches!(
            Session::from_json(scope(), &serde_json::to_vec(&bad).unwrap(), 1000, 2000),
            Err(SessionError::Scope)
        ));
    }
    let text = serde_json::to_string(&original).unwrap();
    for prefix in ["\"counter\":0,", "\"coun\\u0074er\":0,", "\"unknown\":0,"] {
        let bad = format!("{{{prefix}{}", &text[1..]);
        assert!(Session::from_json(scope(), bad.as_bytes(), 1000, 2000).is_err());
    }
    assert!(Session::from_json(scope(), &vec![b' '; 8193], 1000, 2000).is_err());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn malformed_keys_protocol_clocks_and_timeout_cannot_create_an_epoch() {
    for (field, value) in [
        ("protocol", json!("native/0.1")),
        ("counter", json!(1)),
        ("expires", json!(1000)),
        ("expires", json!(MAX_SAFE_INTEGER + 1)),
        ("session", json!("x")),
        ("key", json!("AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB")),
        ("key", json!("AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=")),
    ] {
        let mut bad = bootstrap();
        bad[field] = value;
        assert!(
            Session::from_json(scope(), &serde_json::to_vec(&bad).unwrap(), 1000, 2000).is_err(),
            "{field}"
        );
    }
    for timeout in [0, 30_001, u64::MAX] {
        assert!(
            Session::from_json(
                scope(),
                &serde_json::to_vec(&bootstrap()).unwrap(),
                1000,
                timeout
            )
            .is_err()
        );
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn one_flight_and_authenticated_consumption_advance_exactly_once_even_on_http_error() {
    let mut session = session();
    let signed = session.begin(&request(), 1000).unwrap();
    assert_eq!(signed.counter, 1);
    assert_eq!(signed.deadline, 3000);
    assert!(session.in_flight());
    assert!(matches!(
        session.begin(&request(), 1001),
        Err(SessionError::Busy)
    ));
    session
        .complete(browser::PROTOCOL, 1, 503, &ack(&signed, 503), 1002)
        .unwrap();
    assert!(!session.in_flight());
    assert!(!session.is_closed());
    let next = session.begin(&request(), 1003).unwrap();
    assert_eq!(next.counter, 2);
    // Old ACK cannot consume the next request/counter.
    assert_eq!(
        session.complete(browser::PROTOCOL, 2, 503, &ack(&signed, 503), 1004),
        Err(SessionError::Acknowledgement)
    );
    assert!(session.is_closed());
    assert!(session.key.iter().all(|b| *b == 0));
    assert!(session.id.is_empty());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn request_binding_is_the_shared_codec_not_a_parallel_canonicalizer() {
    let mut session = session();
    let request = request();
    let signed = session.begin(&request, 1000).unwrap();
    let text = browser::canonical(&Binding {
        installation: "i",
        tenant: "t",
        origin_identity: "o",
        revision: "r",
        authority: "example.test",
        session: signed.session(),
        method: request.method,
        target: request.target,
        counter: 1,
        bucket: 0,
        deadline: 3000,
        content_type: request.content_type,
        application_authorization: request.authorization,
        body: request.body,
    })
    .unwrap();
    assert_eq!(
        signed.token(),
        compute_token(&seed(&[1; 32], 0), &text, 1, 0)
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn ambiguity_expiry_rollback_and_counter_exhaustion_clear_secret_state() {
    let mut closed = session();
    closed.begin(&request(), 1000).unwrap();
    closed.invalidate();
    assert!(matches!(
        closed.begin(&request(), 1001),
        Err(SessionError::Closed)
    ));
    assert!(closed.key.iter().all(|b| *b == 0));
    assert!(closed.id.is_empty());
    assert!(!closed.in_flight());
    let mut rollback = session();
    rollback.begin(&request(), 1001).unwrap();
    assert!(matches!(
        rollback.begin(&request(), 1000),
        Err(SessionError::Clock)
    ));
    assert!(rollback.is_closed());
    let mut expired = session();
    let signed = expired.begin(&request(), 1000).unwrap();
    assert_eq!(
        expired.complete(browser::PROTOCOL, 1, 200, &ack(&signed, 200), 3000),
        Err(SessionError::Expired)
    );
    assert!(expired.is_closed());
    let mut exhausted = session();
    exhausted.last = MAX_SAFE_INTEGER;
    assert!(matches!(
        exhausted.begin(&request(), 1000),
        Err(SessionError::Closed)
    ));
    assert!(exhausted.is_closed());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn rejected_local_binding_does_not_reserve_or_advance_a_counter() {
    let mut session = session();
    let mut bad = request();
    bad.target = "//other.test";
    assert!(matches!(
        session.begin(&bad, 1000),
        Err(SessionError::Binding)
    ));
    assert!(!session.in_flight());
    assert!(!session.is_closed());
    assert_eq!(session.begin(&request(), 1000).unwrap().counter, 1);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn forged_ack_protocol_counter_status_and_mac_poison_the_epoch() {
    for variant in 0..4 {
        let mut session = session();
        let signed = session.begin(&request(), 1000).unwrap();
        let protocol = if variant == 0 {
            "native/0.1"
        } else {
            browser::PROTOCOL
        };
        let counter = if variant == 1 { 2 } else { 1 };
        let status = if variant == 2 { 201 } else { 200 };
        let mac = if variant == 3 {
            String::new()
        } else {
            ack(&signed, 200)
        };
        assert_eq!(
            session.complete(protocol, counter, status, &mac, 1001),
            Err(SessionError::Acknowledgement)
        );
        assert!(session.is_closed());
        assert!(session.key.iter().all(|b| *b == 0));
    }
}

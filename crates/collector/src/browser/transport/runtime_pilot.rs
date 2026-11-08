//! Actual Fetch against the separate owned runtime lab, never a Fetch fixture.
//! The lab coordinator must independently prove proxy hits, namespace isolation,
//! no origin bypass, no egress, audit/witness outcomes and process cleanup.
use super::*;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test(async)]
async fn actual_runtime_session_report_and_required_capability_fail_closed() {
    let global: JsValue = js_sys::global().into();
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../pilot/fixture.json")).unwrap();
    let scope: Scope = serde_json::from_value(fixture["scope"].clone()).unwrap();
    let authority = io::text(
        &get(&get(&global, "location").unwrap(), "host").unwrap(),
        1024,
    )
    .unwrap();
    assert_eq!(authority, scope.authority, "owned lab origin required");
    let mut document = SensitiveDocument::enter(&global).expect("owned sensitive document fixture");
    let routes = &fixture["routes"];
    let session = BrowserSession::connect_with(global.clone(), scope, 5000)
        .await
        .expect("actual runtime bootstrap");
    let headers = r#"[["content-type","application/octet-stream"],["authorization","Bearer owned-browser-fixture"]]"#;
    let response = session
        .fetch(
            "POST",
            routes["effect"].as_str().unwrap(),
            headers,
            b"effect".to_vec(),
        )
        .await
        .expect("actual consumed request must have a valid acknowledgement");
    assert_eq!(response.status(), 403);
    let body: serde_json::Value = serde_json::from_slice(&response.bytes()).unwrap();
    assert_eq!(body["verdict"], "CHALLENGE");
    assert!(!session.is_closed());
    let exposed: Vec<[String; 2]> = serde_json::from_str(&response.headers_json()).unwrap();
    assert!(
        exposed
            .iter()
            .all(|[name, _]| !name.to_ascii_lowercase().starts_with("x-humanymous-"))
    );

    // Oversized requests fail locally before a counter or network operation.
    // This does not observe a server413 or poison a valid current epoch.
    assert!(
        session
            .fetch(
                "POST",
                routes["effect"].as_str().unwrap(),
                headers,
                vec![b'x'; 65537],
            )
            .await
            .is_err()
    );
    assert!(!session.is_closed());

    // The largest supported body must traverse actual Fetch and validate its
    // consumed ACK. The lab separately checks exactly two effect proxy hits.
    let largest = session
        .fetch(
            "POST",
            routes["effect"].as_str().unwrap(),
            headers,
            vec![b'x'; 65536],
        )
        .await
        .expect("maximum supported body must receive a consumed acknowledgement");
    assert_eq!(
        u64::from(largest.status()),
        fixture["expected"]["before_report_status"]
            .as_u64()
            .unwrap()
    );
    let body: serde_json::Value = serde_json::from_slice(&largest.bytes()).unwrap();
    assert_eq!(
        body["verdict"],
        fixture["expected"]["before_report_verdict"]
    );
    assert!(!session.is_closed());

    // Preserve actual headless/browser observations. Never fabricate a benign
    // report or interpret accepted report ingestion as human/code attestation.
    let mut collector = crate::BrowserCollector::new().unwrap();
    let report = collector.snapshot_json().unwrap();
    collector.stop().unwrap();
    let collected = session
        .fetch(
            "POST",
            routes["collect"].as_str().unwrap(),
            r#"[["content-type","application/json"]]"#,
            report.into_bytes(),
        )
        .await
        .expect("actual report ingestion must acknowledge consumed counter");
    assert_eq!(collected.status(), 200);
    let body: serde_json::Value = serde_json::from_slice(&collected.bytes()).unwrap();
    assert_eq!(body["verdict"], "COLLECTED");
    assert!(!session.is_closed());

    // Missing required fresh provenance fails before counter consumption. No
    // ACK is issued, so the transport must poison this ambiguous client epoch.
    assert!(
        session
            .fetch(
                "POST",
                routes["required_collector"].as_str().unwrap(),
                headers,
                b"effect".to_vec()
            )
            .await
            .is_err()
    );
    assert!(session.is_closed());
    session.close().unwrap();
    assert!(
        session
            .fetch(
                "POST",
                routes["effect"].as_str().unwrap(),
                headers,
                b"must-not-be-sent".to_vec()
            )
            .await
            .is_err()
    );
    document.restore().expect("restore owned document history");
}

/// Keep the actual network pilot under a sensitive same-origin document URL.
struct SensitiveDocument {
    history: JsValue,
    state: JsValue,
    url: JsValue,
    restored: bool,
}

impl SensitiveDocument {
    fn enter(global: &JsValue) -> Result<Self, JsValue> {
        let history = get(global, "history")?;
        let mut document = Self {
            state: get(&history, "state")?,
            url: get(&get(global, "location")?, "href")?,
            history,
            restored: false,
        };
        call(
            &document.history,
            "replaceState",
            &[
                JsValue::NULL,
                "".into(),
                "/private-document/path?private-query=fixture#private-fragment".into(),
            ],
        )?;
        let location = get(global, "location")?;
        if get(&location, "pathname")?.as_string().as_deref() != Some("/private-document/path")
            || get(&location, "search")?.as_string().as_deref() != Some("?private-query=fixture")
            || get(&location, "hash")?.as_string().as_deref() != Some("#private-fragment")
        {
            document.restore()?;
            return Err("sensitive document fixture mismatch".into());
        }
        Ok(document)
    }

    fn restore(&mut self) -> Result<(), JsValue> {
        call(
            &self.history,
            "replaceState",
            &[self.state.clone(), "".into(), self.url.clone()],
        )?;
        self.restored = true;
        Ok(())
    }
}

impl Drop for SensitiveDocument {
    fn drop(&mut self) {
        if !self.restored {
            // Best effort on early failure; successful pilots require restore above.
            let _ = self.restore();
        }
    }
}

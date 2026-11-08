use super::*;
use humanymous_core::integrity::request::{
    browser::{Binding, acknowledgement, canonical},
    seed, verify_token,
};
use io::set;
use std::{cell::Cell, future::Future, task::Poll};
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::{JsFuture, future_to_promise};
use wasm_bindgen_test::wasm_bindgen_test;

struct Fixture {
    global: JsValue,
    scope: Scope,
    mode: Rc<Cell<u8>>,
    calls: Rc<Cell<u32>>,
    last: Rc<Cell<u64>>,
}
impl Fixture {
    fn new() -> Self {
        let actual: JsValue = js_sys::global().into();
        let global: JsValue = js_sys::Object::new().into();
        for key in [
            "Request",
            "Headers",
            "Response",
            "AbortController",
            "URL",
            "Date",
            "location",
            "ReadableStream",
        ] {
            set(&global, key, &get(&actual, key).unwrap()).unwrap();
        }
        for key in ["setTimeout", "clearTimeout"] {
            let bound = call(
                &get(&actual, key).unwrap(),
                "bind",
                std::slice::from_ref(&actual),
            )
            .unwrap();
            set(&global, key, &bound).unwrap();
        }
        let authority = get(&get(&actual, "location").unwrap(), "host")
            .unwrap()
            .as_string()
            .unwrap();
        let scope = Scope {
            installation: "i".into(),
            tenant: "t".into(),
            origin: "o".into(),
            revision: "r".into(),
            authority,
        };
        let mode = Rc::new(Cell::new(0));
        let calls = Rc::new(Cell::new(0));
        let last = Rc::new(Cell::new(0));
        let server_mode = mode.clone();
        let server_calls = calls.clone();
        let server_last = last.clone();
        let server_scope = scope.clone();
        let server_global = global.clone();
        let fetch=Closure::<dyn Fn(JsValue)->Promise>::new(move |request:JsValue| {
            server_calls.set(server_calls.get()+1);
            let bootstrap=get(&request,"url").unwrap().as_string().unwrap().ends_with("/_humanymous/browser/session");
            if server_mode.get()==6 && !bootstrap{return Promise::new(&mut|_,_|{});}
            let global=server_global.clone();let scope=server_scope.clone();let mode=server_mode.get();let last=server_last.clone();
            future_to_promise(async move {
                assert_eq!(get(&request,"mode")?.as_string().as_deref(),Some("same-origin"));
                assert_eq!(get(&request,"credentials")?.as_string().as_deref(),Some("omit"));
                assert_eq!(get(&request,"redirect")?.as_string().as_deref(),Some("error"));
                let origin=request::origin(&global,&scope)?;
                assert_eq!(get(&request,"referrer")?.as_string(),Some(format!("{origin}/")));
                assert_eq!(get(&request,"referrerPolicy")?.as_string().as_deref(),Some("same-origin"));
                assert_eq!(get(&request,"cache")?.as_string().as_deref(),Some("no-store"));
                if bootstrap {
                    if (15..=18).contains(&mode) {
                        let body: &[u8] = match mode {
                            15 => br#"{"error":"browser_origin"}"#,
                            16 => br#"{"error":"browser_fetch_metadata"}"#,
                            17 => br#"{"error":"browser_origin","secret":"sensitive-bootstrap-fixture"}"#,
                            _ => &[b'x'; 257],
                        };
                        return response(&global,403,&[],body,None);
                    }
                    if (9..=11).contains(&mode) {
                        let status=match mode {9=>403,10=>422,_=>503};
                        return response(&global,status,&[],b"sensitive-bootstrap-fixture",None);
                    }
                    if mode==12{return response(&global,200,&[],b"{}",None);}
                    if mode==14{return Err(error("sensitive-bootstrap-fixture"));}
                    let mut reply=serde_json::json!({"protocol":PROTOCOL,"session":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","key":"AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE","counter":0,"expires":io::now(&global)?+60000,"installation":scope.installation,"tenant":scope.tenant,"origin":scope.origin,"revision":scope.revision,"authority":scope.authority});
                    if mode==13{reply["tenant"]=serde_json::json!("other-tenant");}
                    return response(&global,200,&[],reply.to_string().as_bytes(),None);
                }
                if mode==1{return Err(error("owned network failure fixture"));}
                let headers=get(&request,"headers")?;
                let n=io::header(&headers,"x-humanymous-counter")?.parse::<u64>().unwrap();
                let bucket=io::header(&headers,"x-humanymous-bucket")?.parse::<u64>().unwrap();
                let deadline=io::header(&headers,"x-humanymous-deadline")?.parse::<u64>().unwrap();
                assert_eq!(n,last.get()+1);assert_eq!(io::header(&headers,"x-humanymous-protocol")?,PROTOCOL);
                let body=JsFuture::from(call(&request,"arrayBuffer",&[])?.dyn_into::<Promise>()?).await?;
                let body=js_sys::Uint8Array::new(&body).to_vec();
                let origin=request::origin(&global,&scope)?;
                let target=request::final_target(&global,&origin,&request)?;
                let method=io::text(&get(&request,"method")?,32)?;
                let content=io::header(&headers,"content-type")?;let authorization=io::header(&headers,"authorization")?;
                let id=io::header(&headers,"x-humanymous-session")?;
                let token=io::header(&headers,"x-humanymous-token")?;
                let text=canonical(&Binding{installation:&scope.installation,tenant:&scope.tenant,origin_identity:&scope.origin,revision:&scope.revision,authority:&scope.authority,session:&id,method:&method,target:&target,counter:n,bucket,deadline,content_type:&content,application_authorization:&authorization,body:&body}).unwrap();
                assert!(verify_token(&seed(&[1;32],last.get()),&text,n,bucket,&token));
                last.set(n);
                let status=if mode==4{503}else{200};
                let ack=if mode==3{"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string()}else{acknowledgement(&[1;32],&token,n,bucket,status).unwrap()};
                let mut reply_headers=vec![("content-type".to_string(),"application/octet-stream".to_string())];
                if mode!=2 {reply_headers.extend([("x-humanymous-protocol".into(),PROTOCOL.into()),("x-humanymous-counter".into(),n.to_string()),("x-humanymous-ack".into(),ack)]);}
                if mode==5 {
                    let start=Closure::<dyn Fn(JsValue)>::new(|controller:JsValue|{call(&controller,"error",&[error("owned body failure fixture")]).unwrap();}).into_js_value();
                    let source=js_sys::Object::new();set(&source,"start",&start)?;
                    let stream=io::construct(&global,"ReadableStream",&[source.into()])?;
                    return response(&global,status,&reply_headers,&[],Some(stream));
                }
                if mode==8 {
                    let start=Closure::<dyn Fn(JsValue)>::new(|controller:JsValue|{call(&controller,"enqueue",&[js_sys::Uint8Array::from(&[1u8][..]).into()]).unwrap();}).into_js_value();
                    let source=js_sys::Object::new();set(&source,"start",&start)?;
                    let stream=io::construct(&global,"ReadableStream",&[source.into()])?;
                    return response(&global,status,&reply_headers,&[],Some(stream));
                }
                let payload=if mode==7{vec![42;1_048_577]}else{vec![0,255,128,2]};
                response(&global,status,&reply_headers,&payload,None)
            })
        }).into_js_value();
        set(&global, "fetch", &fetch).unwrap();
        Self {
            global,
            scope,
            mode,
            calls,
            last,
        }
    }
    async fn connect(&self, timeout: u32) -> BrowserSession {
        BrowserSession::connect_with(self.global.clone(), self.scope.clone(), timeout)
            .await
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // The Rust fetch closure captures global, whose fetch property owns that
        // same closure. Break this cross-language cycle between test cases.
        let _ = set(&self.global, "fetch", &JsValue::UNDEFINED);
    }
}
fn response(
    global: &JsValue,
    status: u16,
    headers: &[(String, String)],
    body: &[u8],
    stream: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    let options = js_sys::Object::new();
    set(&options, "status", &status.into())?;
    let header_object = io::construct(global, "Headers", &[])?;
    for (name, value) in headers {
        call(
            &header_object,
            "set",
            &[name.as_str().into(), value.as_str().into()],
        )?;
    }
    set(&options, "headers", &header_object)?;
    io::construct(
        global,
        "Response",
        &[
            stream.unwrap_or_else(|| js_sys::Uint8Array::from(body).into()),
            options.into(),
        ],
    )
}
const HEADERS: &str = r#"[["authorization","Bearer synthetic"],["content-type","application/json"],["x-tag","first"],["X-Tag","second"]]"#;

#[wasm_bindgen_test]
fn actual_final_url_keeps_empty_query_and_encoded_query_data() {
    let fixture = Fixture::new();
    let origin = request::origin(&fixture.global, &fixture.scope).unwrap();
    let abort = Abort(io::construct(&fixture.global, "AbortController", &[]).unwrap());
    for target in ["/api?", "/api?next=%2Fhome&hash=%23&unicode=%C3%A9"] {
        let request =
            request::build(&fixture.global, &origin, "GET", target, "[]", &[], &abort).unwrap();
        assert_eq!(
            request::final_target(&fixture.global, &origin, &request).unwrap(),
            target
        );
    }
    let actual: JsValue = js_sys::global().into();
    let history = get(&actual, "history").unwrap();
    let original_state = get(&history, "state").unwrap();
    let original_url = get(&get(&actual, "location").unwrap(), "href").unwrap();
    call(
        &history,
        "replaceState",
        &[
            JsValue::NULL,
            "".into(),
            "/private-document/path?private-query=fixture#private-fragment".into(),
        ],
    )
    .unwrap();
    let observed: Result<Vec<JsValue>, JsValue> =
        ["/_humanymous/browser/session", "/api?effect=fixture"]
            .into_iter()
            .map(|target| {
                let request = request::build(
                    &fixture.global,
                    &origin,
                    "POST",
                    target,
                    "[]",
                    b"{}",
                    &abort,
                )?;
                get(&request, "referrer")
            })
            .collect();
    // Restore document history before inspecting any fallible observations.
    call(
        &history,
        "replaceState",
        &[original_state, "".into(), original_url.clone()],
    )
    .unwrap();
    assert_eq!(
        get(&get(&actual, "location").unwrap(), "href").unwrap(),
        original_url
    );
    for referrer in observed.unwrap() {
        assert_eq!(referrer.as_string(), Some(format!("{origin}/")));
    }
}

#[wasm_bindgen_test(async)]
async fn actual_request_response_apis_bind_final_bytes_and_advance_on_valid_ack() {
    let fixture = Fixture::new();
    let session = fixture.connect(2000).await;
    let first = session
        .fetch("POST", "/api?q=1", HEADERS, b"{\"v\":1}".to_vec())
        .await
        .unwrap();
    assert_eq!(first.status(), 200);
    assert_eq!(first.bytes(), [0, 255, 128, 2]);
    assert!(!first.headers_json().contains("x-humanymous-"));
    fixture.mode.set(4);
    assert_eq!(
        session
            .fetch("PUT", "/api", HEADERS, b"{}".to_vec())
            .await
            .unwrap()
            .status(),
        503
    );
    assert_eq!(fixture.last.get(), 2);
    assert!(!session.is_closed());
    session.close().unwrap();
    assert!(session.is_closed());
}

#[wasm_bindgen_test(async)]
async fn bad_local_requests_never_fetch_or_consume_and_valid_request_still_works() {
    let fixture = Fixture::new();
    let session = fixture.connect(2000).await;
    let calls = fixture.calls.get();
    for target in [
        "//other.test/x",
        "/a/../b",
        "/a%20b",
        "/api\r\n",
        "https://other.test/",
    ] {
        assert!(
            session
                .fetch("POST", target, HEADERS, b"{}".to_vec())
                .await
                .is_err(),
            "{target}"
        );
    }
    for headers in [
        r#"[["cookie","secret"]]"#,
        r#"[["x-humanymous-token","x"]]"#,
        r#"[["Host","other.test"]]"#,
        r#"[["authorization","a"],["Authorization","b"]]"#,
    ] {
        assert!(
            session
                .fetch("POST", "/api", headers, b"{}".to_vec())
                .await
                .is_err()
        );
    }
    assert_eq!(fixture.calls.get(), calls);
    assert!(!session.is_closed());
    session
        .fetch("POST", "/api", HEADERS, b"{}".to_vec())
        .await
        .unwrap();
    assert_eq!(fixture.last.get(), 1);
}

#[wasm_bindgen_test(async)]
async fn network_bad_ack_missing_ack_body_error_and_overflow_all_close_without_retry() {
    for mode in [1, 2, 3, 5, 7, 8] {
        let fixture = Fixture::new();
        let session = fixture.connect(if mode == 8 { 50 } else { 2000 }).await;
        fixture.mode.set(mode);
        assert!(
            session
                .fetch("POST", "/api", HEADERS, b"{}".to_vec())
                .await
                .is_err(),
            "mode{mode}"
        );
        assert!(session.is_closed());
        assert_eq!(fixture.calls.get(), 2);
        assert!(
            session
                .fetch("POST", "/api", HEADERS, b"{}".to_vec())
                .await
                .is_err()
        );
        assert_eq!(fixture.calls.get(), 2);
    }
}

#[wasm_bindgen_test(async)]
async fn pending_request_is_single_flight_and_explicit_close_or_future_drop_aborts() {
    let fixture = Fixture::new();
    let session = fixture.connect(100).await;
    fixture.mode.set(6);
    let mut future =
        Box::pin(session.fetch_inner("POST", "/api", HEADERS, Zeroizing::new(b"{}".to_vec())));
    std::future::poll_fn(|context| {
        assert!(future.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    let signal = get(&session.abort.borrow().as_ref().unwrap().0, "signal").unwrap();
    assert!(
        session
            .fetch("POST", "/api", HEADERS, b"{}".to_vec())
            .await
            .is_err()
    );
    assert!(!session.is_closed());
    drop(future);
    assert!(session.is_closed());
    assert_eq!(get(&signal, "aborted").unwrap().as_bool(), Some(true));
    let fixture = Fixture::new();
    let session = fixture.connect(25).await;
    fixture.mode.set(6);
    let mut future =
        Box::pin(session.fetch_inner("POST", "/api", HEADERS, Zeroizing::new(b"{}".to_vec())));
    std::future::poll_fn(|context| {
        assert!(future.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    session.close().unwrap();
    assert!(session.is_closed());
    assert!(future.await.is_err());
}

#[wasm_bindgen_test(async)]
async fn bootstrap_future_drop_and_timeout_abort_the_actual_request_signal() {
    for discard in [true, false] {
        let fixture = Fixture::new();
        let observed = Rc::new(RefCell::new(None));
        let captured = observed.clone();
        let fetch = Closure::<dyn Fn(JsValue) -> Promise>::new(move |request: JsValue| {
            *captured.borrow_mut() = Some(get(&request, "signal").unwrap());
            Promise::new(&mut |_, _| {})
        })
        .into_js_value();
        set(&fixture.global, "fetch", &fetch).unwrap();
        let mut future = Box::pin(BrowserSession::connect_with(
            fixture.global.clone(),
            fixture.scope.clone(),
            25,
        ));
        std::future::poll_fn(|context| {
            assert!(future.as_mut().poll(context).is_pending());
            Poll::Ready(())
        })
        .await;
        let signal = observed.borrow().as_ref().unwrap().clone();
        assert_eq!(get(&signal, "aborted").unwrap().as_bool(), Some(false));
        if discard {
            drop(future);
        } else {
            assert_eq!(future.await.err(), Some(bootstrap::Failure::FetchAwait));
        }
        assert_eq!(get(&signal, "aborted").unwrap().as_bool(), Some(true));
    }
}

#[wasm_bindgen_test(async)]
async fn real_same_origin_fetch_to_missing_owned_route_cannot_create_a_session() {
    let global: JsValue = js_sys::global().into();
    let scope = Fixture::new().scope.clone();
    assert!(
        BrowserSession::connect_with(global, scope, 500)
            .await
            .is_err()
    );
    for (mode, expected) in [
        (9, bootstrap::Failure::HttpForbidden),
        (10, bootstrap::Failure::HttpUnsupported),
        (11, bootstrap::Failure::HttpServer),
        (12, bootstrap::Failure::SessionReply),
        (13, bootstrap::Failure::SessionScope),
        (14, bootstrap::Failure::FetchAwait),
        (15, bootstrap::Failure::HttpForbiddenOrigin),
        (16, bootstrap::Failure::HttpForbiddenFetchMetadata),
        (17, bootstrap::Failure::HttpForbidden),
        (18, bootstrap::Failure::HttpForbidden),
    ] {
        let fixture = Fixture::new();
        fixture.mode.set(mode);
        let failure =
            BrowserSession::connect_with(fixture.global.clone(), fixture.scope.clone(), 500)
                .await
                .err()
                .expect("fixture bootstrap must fail");
        assert_eq!(failure, expected);
        assert!(!format!("{failure:?}").contains("sensitive-bootstrap-fixture"));
        assert_eq!(fixture.calls.get(), 1);
        assert_eq!(fixture.last.get(), 0);
    }
    let fixture = Fixture::new();
    assert_eq!(
        BrowserSession::connect_with(fixture.global.clone(), fixture.scope.clone(), 0)
            .await
            .err(),
        Some(bootstrap::Failure::TimeoutProfile)
    );
    let mut wrong_scope = fixture.scope.clone();
    wrong_scope.authority.push_str(".invalid");
    assert_eq!(
        BrowserSession::connect_with(fixture.global.clone(), wrong_scope, 500)
            .await
            .err(),
        Some(bootstrap::Failure::Origin)
    );
    set(&fixture.global, "Request", &JsValue::UNDEFINED).unwrap();
    assert_eq!(
        BrowserSession::connect_with(fixture.global.clone(), fixture.scope.clone(), 500)
            .await
            .err(),
        Some(bootstrap::Failure::Request)
    );
    assert_eq!(fixture.calls.get(), 0);
}

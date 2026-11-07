//! Same-origin buffered Fetch owner; no cookie/stream or qualified-producer claim.
mod bootstrap;
mod io;
mod request;
#[cfg(all(test, feature = "runtime-pilot"))]
mod runtime_pilot;
#[cfg(test)]
mod tests;
use super::{
    reflection::{call, get},
    resources::{Abort, Timer},
};
use crate::session::{Request, Scope, Session};
use humanymous_core::integrity::request::browser::PROTOCOL;
use js_sys::{Array, Function, Promise};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use zeroize::Zeroizing;

fn error(value: &str) -> JsValue {
    JsValue::from_str(value)
}
fn failure(_: JsValue) -> JsValue {
    error("browser transport failed")
}

/// Explicit same-origin cookie-free buffered session transport.
#[wasm_bindgen]
pub struct BrowserSession {
    global: JsValue,
    origin: String,
    state: Rc<RefCell<Session>>,
    abort: Rc<RefCell<Option<Rc<Abort>>>>,
    timeout: u32,
}
/// Buffered application response; headers are browser-normalized, not raw capture.
#[wasm_bindgen]
pub struct BrowserResponse {
    status: u16,
    headers: String,
    body: Zeroizing<Vec<u8>>,
}
#[wasm_bindgen]
impl BrowserResponse {
    /// Actual received HTTP status after ACK and complete bounded body validation.
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> u16 {
        self.status
    }
    /// Exposed normalized header pairs, excluding reserved protocol headers.
    pub fn headers_json(&self) -> String {
        self.headers.clone()
    }
    /// Application bytes copied to the caller; not a streaming Response object.
    pub fn bytes(&self) -> Vec<u8> {
        self.body.to_vec()
    }
}
#[wasm_bindgen]
impl BrowserSession {
    /// Bootstrap exact caller-provisioned scope over secure/loopback same origin.
    pub async fn connect(scope_json: &str, timeout_ms: u32) -> Result<BrowserSession, JsValue> {
        if scope_json.len() > 4096 {
            return Err(error("scope exceeds limit"));
        }
        let scope =
            serde_json::from_str(scope_json).map_err(|_| error("invalid provisioned scope"))?;
        Self::connect_with(js_sys::global().into(), scope, timeout_ms)
            .await
            .map_err(|_| error("browser transport failed"))
    }
    /// Sign and send one bounded request; no redirect, cookie or automatic retry.
    pub async fn fetch(
        &self,
        method: &str,
        target: &str,
        headers_json: &str,
        body: Vec<u8>,
    ) -> Result<BrowserResponse, JsValue> {
        self.fetch_inner(method, target, headers_json, Zeroizing::new(body))
            .await
            .map_err(failure)
    }
    /// Invalidate the key epoch and explicitly abort the outstanding request.
    /// Ignoring/GC'ing a JavaScript Promise alone is not cancellation.
    pub fn close(&self) -> Result<(), JsValue> {
        self.state
            .try_borrow_mut()
            .map_err(|_| error("session busy"))?
            .invalidate();
        let abort = self
            .abort
            .try_borrow_mut()
            .map_err(|_| error("abort owner busy"))?
            .take();
        if let Some(abort) = abort {
            abort.abort().map_err(failure)?;
        }
        Ok(())
    }
    /// Local key-epoch status, not evidence of current server acceptance.
    pub fn is_closed(&self) -> bool {
        self.state
            .try_borrow()
            .map_or(true, |state| state.is_closed())
    }
}
impl BrowserSession {
    async fn connect_with(
        global: JsValue,
        scope: Scope,
        timeout: u32,
    ) -> Result<Self, bootstrap::Failure> {
        bootstrap::connect(global, scope, timeout).await
    }
    async fn fetch_inner(
        &self,
        method: &str,
        target: &str,
        headers_json: &str,
        body: Zeroizing<Vec<u8>>,
    ) -> Result<BrowserResponse, JsValue> {
        let scope = self
            .state
            .try_borrow()
            .map_err(|_| error("session busy"))?
            .scope()
            .clone();
        if request::origin(&self.global, &scope)? != self.origin {
            return Err(error("browser origin changed"));
        }
        let abort = Rc::new(Abort(io::construct(&self.global, "AbortController", &[])?));
        let req = request::build(
            &self.global,
            &self.origin,
            method,
            target,
            headers_json,
            &body,
            &abort,
        )?;
        let method = io::text(&get(&req, "method")?, 32)?;
        let target = request::final_target(&self.global, &self.origin, &req)?;
        let headers = get(&req, "headers")?;
        let content_type = io::header(&headers, "content-type")?;
        let authorization = Zeroizing::new(io::header(&headers, "authorization")?);
        let began = io::now(&self.global)?;
        let signed = self
            .state
            .try_borrow_mut()
            .map_err(|_| error("session busy"))?
            .begin(
                &Request {
                    method: &method,
                    target: &target,
                    content_type: &content_type,
                    authorization: &authorization,
                    body: &body,
                },
                began,
            )
            .map_err(|_| error("session cannot begin request"))?;
        let mut flight = Flight {
            state: self.state.clone(),
            slot: self.abort.clone(),
            abort: abort.clone(),
            complete: false,
        };
        *self
            .abort
            .try_borrow_mut()
            .map_err(|_| error("abort owner busy"))? = Some(abort);
        for (name, value) in [
            ("x-humanymous-protocol", PROTOCOL.to_string()),
            ("x-humanymous-session", signed.session().to_string()),
            ("x-humanymous-counter", signed.counter.to_string()),
            ("x-humanymous-bucket", signed.bucket.to_string()),
            ("x-humanymous-deadline", signed.deadline.to_string()),
            ("x-humanymous-token", signed.token().to_string()),
        ] {
            let value = Zeroizing::new(value);
            call(&headers, "set", &[name.into(), JsValue::from_str(&value)])?;
        }
        let now = io::now(&self.global)?;
        if now < began || now >= signed.deadline {
            return Err(error("request clock expired"));
        }
        let remaining = (signed.deadline - now).min(u64::from(self.timeout)) as u32;
        let mut timer = Timer::start(&self.global, remaining)?;
        let response = io::wait(&timer, call(&self.global, "fetch", &[req])?).await?;
        let status = io::status(&response)?;
        let response_headers = get(&response, "headers")?;
        let content_type = io::header(&response_headers, "content-type")?.to_ascii_lowercase();
        if content_type.starts_with("text/event-stream")
            || content_type.starts_with("application/grpc")
        {
            return Err(error("streaming response profile unsupported"));
        }
        let bytes = io::body(&response, &timer, 1_048_576).await?;
        let protocol = io::header(&response_headers, "x-humanymous-protocol")?;
        let counter = io::header(&response_headers, "x-humanymous-counter")?;
        let counter = counter
            .parse::<u64>()
            .ok()
            .filter(|n| n.to_string() == counter)
            .ok_or_else(|| error("invalid acknowledgement counter"))?;
        let ack = Zeroizing::new(io::header(&response_headers, "x-humanymous-ack")?);
        let exposed = io::exposed_headers(&response_headers)?;
        timer.cancel()?;
        flight.abort.abort()?;
        let now = io::now(&self.global)?;
        let completion = {
            let mut state = self
                .state
                .try_borrow_mut()
                .map_err(|_| error("session busy"))?;
            state.complete(&protocol, counter, status, &ack, now)
        };
        completion.map_err(|_| error("session acknowledgement rejected"))?;
        flight.complete = true;
        Ok(BrowserResponse {
            status,
            headers: exposed,
            body: bytes,
        })
    }
}
impl Drop for BrowserSession {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
struct Flight {
    state: Rc<RefCell<Session>>,
    slot: Rc<RefCell<Option<Rc<Abort>>>>,
    abort: Rc<Abort>,
    complete: bool,
}
impl Drop for Flight {
    fn drop(&mut self) {
        if !self.complete
            && let Ok(mut state) = self.state.try_borrow_mut()
        {
            state.invalidate();
        }
        if let Ok(mut slot) = self.slot.try_borrow_mut()
            && slot
                .as_ref()
                .is_some_and(|value| Rc::ptr_eq(value, &self.abort))
        {
            slot.take();
        }
        let _ = self.abort.abort();
    }
}

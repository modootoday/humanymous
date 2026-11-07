//! Field-free bootstrap diagnostics; never retain or format foreign error data.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Failure {
    TimeoutProfile,
    Origin,
    AbortController,
    Request,
    Timer,
    FetchCall,
    FetchAwait,
    ResponseStatus,
    HttpForbidden,
    HttpUnsupported,
    HttpClient,
    HttpServer,
    HttpOther,
    Body,
    TimerCancel,
    Abort,
    Clock,
    SessionScope,
    SessionReply,
}

fn rejected(status: u16) -> Failure {
    match status {
        403 => Failure::HttpForbidden,
        422 => Failure::HttpUnsupported,
        400..=499 => Failure::HttpClient,
        500..=599 => Failure::HttpServer,
        _ => Failure::HttpOther,
    }
}

pub(super) async fn connect(
    global: JsValue,
    scope: Scope,
    timeout: u32,
) -> Result<BrowserSession, Failure> {
    if !(1..=30000).contains(&timeout) {
        return Err(Failure::TimeoutProfile);
    }
    let origin = request::origin(&global, &scope).map_err(|_| Failure::Origin)?;
    let abort = Abort(
        io::construct(&global, "AbortController", &[]).map_err(|_| Failure::AbortController)?,
    );
    let req = request::build(
        &global,
        &origin,
        "POST",
        "/_humanymous/browser/session",
        r#"[["content-type","application/json"]]"#,
        b"{}",
        &abort,
    )
    .map_err(|_| Failure::Request)?;
    let mut timer = Timer::start(&global, timeout).map_err(|_| Failure::Timer)?;
    let promise = call(&global, "fetch", &[req]).map_err(|_| Failure::FetchCall)?;
    let response = io::wait(&timer, promise)
        .await
        .map_err(|_| Failure::FetchAwait)?;
    let status = io::status(&response).map_err(|_| Failure::ResponseStatus)?;
    if !(200..=299).contains(&status) {
        return Err(rejected(status));
    }
    let bytes = io::body(&response, &timer, 8192)
        .await
        .map_err(|_| Failure::Body)?;
    timer.cancel().map_err(|_| Failure::TimerCancel)?;
    abort.abort().map_err(|_| Failure::Abort)?;
    let now = io::now(&global).map_err(|_| Failure::Clock)?;
    let state = Session::from_json(scope, &bytes, now, u64::from(timeout)).map_err(|failure| {
        if failure == crate::session::SessionError::Scope {
            Failure::SessionScope
        } else {
            Failure::SessionReply
        }
    })?;
    Ok(BrowserSession {
        global,
        origin,
        state: Rc::new(RefCell::new(state)),
        abort: Rc::new(RefCell::new(None)),
        timeout,
    })
}

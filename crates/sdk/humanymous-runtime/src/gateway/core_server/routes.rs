//! Route table, middleware chain and the non-collect handlers of the legacy
//! Core (Go `routes`, `withSecurityHeaders`, `withTrafficLog`,
//! `withObservability`, handlers.go, resource_handlers.go, ops.go `healthz`,
//! and the `ServeMux`/`FileServer`/`ServeFile` behaviour they relied on).
use super::http::{Request, Response, clean, clean_mux_path, http_date, unescape};
use super::{Connection, CoreServer, Now, PassRequest, State, timestamp};
use crate::gateway::demo;
use crate::gateway::logging::{Completion, HttpVersion, observe};
use crate::gateway::operations::mlcorrect::operator_authorized;
use crate::gateway::request::{day_bucket, traffic_ip_hash};
use crate::gateway::resource::{
    self, Issuer, MAX_TRACE_BODY, ResourceRequest, SessionView, Tracer,
};
use crate::gateway::session::pow;
use crate::network::http::{
    canonical_name, client_ip, cookie_value, first_value, parse_ip, project_request,
};
use crate::network::tls::{engine_from_client_hello, extension_order_hash, ja3, ja4};
use crate::network::traffic::{Record, hash_order, hash_ua};
use humanymous_core::signals::observation::{Signal, Source, Verdict};
use serde::Serialize;
use serde_json::{Value, json};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::path::Path;
use std::sync::MutexGuard;

/// Go `withSecurityHeaders` (SoT-11): set on every response, before the
/// handler, so a handler header of the same name replaces it.
pub const SECURITY_HEADERS: [(&str, &str); 3] = [
    (
        "Content-Security-Policy-Report-Only",
        "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; object-src 'none'; base-uri 'none'; report-uri /api/csp-report",
    ),
    ("X-Content-Type-Options", "nosniff"),
    ("Referrer-Policy", "strict-origin-when-cross-origin"),
];

const ACCEPT_CH: &str = "Sec-CH-UA-Full-Version-List, Sec-CH-UA-Platform-Version, Sec-CH-UA-Arch";

/// Handler identity selected by the Go `ServeMux` patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Route {
    Page,
    Demo,
    Session,
    Collect,
    ReportOne,
    ReportList,
    Csp,
    Pow,
    Pass,
    Trace,
    Traffic,
    Resource,
    Static,
    Healthz,
}

fn segments(escaped: &str) -> Vec<String> {
    escaped
        .trim_start_matches('/')
        .split('/')
        .map(|segment| unescape(segment, false).unwrap_or_else(|| segment.to_owned()))
        .collect()
}

/// Match a cleaned escaped path; segments are compared unescaped, as Go
/// 1.22+ patterns are. `None` is the `/` catch-all.
fn matched(escaped: &str) -> Option<Route> {
    let segs = segments(escaped);
    let segs: Vec<&str> = segs.iter().map(String::as_str).collect();
    let exact = match segs.as_slice() {
        ["demo"] => Some(Route::Demo),
        ["api", "session"] => Some(Route::Session),
        ["api", "collect"] => Some(Route::Collect),
        ["api", "report"] => Some(Route::ReportList),
        ["api", "csp-report"] => Some(Route::Csp),
        ["api", "pow"] => Some(Route::Pow),
        ["pass"] | ["api", "pass", "new" | "pow" | "attest" | "solve" | "kpi"] => Some(Route::Pass),
        ["api", "trace"] => Some(Route::Trace),
        ["healthz"] => Some(Route::Healthz),
        _ => None,
    };
    exact.or(match segs.as_slice() {
        ["api", "report", _, ..] => Some(Route::ReportOne),
        ["api", "traffic", _, ..] => Some(Route::Traffic),
        ["res", _, ..] => Some(Route::Resource),
        ["static", _, ..] => Some(Route::Static),
        _ => None,
    })
}

/// Go `ServeMux.Handler` outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Dispatch {
    Handle(Route),
    Redirect(String),
}

/// Go 1.26 `ServeMux`: a subtree pattern without its trailing slash redirects
/// to it, then an unclean escaped path redirects to its cleaned form (both
/// 307, query preserved); otherwise the most specific pattern handles.
pub(super) fn dispatch(request: &Request) -> Option<Dispatch> {
    let (escaped, query) = request.split_target()?;
    let cleaned = clean_mux_path(escaped);
    let with_query = |path: String| {
        if query.is_empty() {
            path
        } else {
            format!("{path}?{query}")
        }
    };
    let found = matched(&cleaned);
    if found.is_none() && !cleaned.ends_with('/') {
        let slashed = format!("{cleaned}/");
        if matched(&slashed).is_some() {
            return Some(Dispatch::Redirect(with_query(slashed)));
        }
    }
    if cleaned != escaped {
        return Some(Dispatch::Redirect(with_query(cleaned)));
    }
    Some(Dispatch::Handle(found.unwrap_or(Route::Page)))
}

/// Go `htmlEscape` used by `http.Redirect`.
fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&#34;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// Go `http.Redirect(w, r, url, 307)`.
fn redirect(location: &str, method: &str) -> resource::Response {
    let mut headers = vec![("Location", location.to_owned())];
    let mut body = Vec::new();
    if method == "GET" || method == "HEAD" {
        headers.push(("Content-Type", "text/html; charset=utf-8".to_owned()));
    }
    if method == "GET" {
        body = format!(
            "<a href=\"{}\">Temporary Redirect</a>.\n\n",
            html_escape(location)
        )
        .into_bytes();
    }
    resource::Response {
        status: 307,
        headers,
        body,
    }
}

fn not_found() -> resource::Response {
    resource::not_found()
}

fn unavailable(reason: &str) -> resource::Response {
    resource::Response::text(503, Vec::new(), reason)
}

fn empty(status: u16) -> resource::Response {
    resource::Response {
        status,
        headers: Vec::new(),
        body: Vec::new(),
    }
}

/// Go `localRedirect`: 301 with a relative Location and no body.
fn local_redirect(location: &str, query: &str) -> resource::Response {
    let location = if query.is_empty() {
        location.to_owned()
    } else {
        format!("{location}?{query}")
    };
    resource::Response {
        status: 301,
        headers: vec![("Location", location)],
        body: Vec::new(),
    }
}

/// Go `toHTTPError` for a failed open or stat.
fn file_error(error: &std::io::Error) -> resource::Response {
    match error.kind() {
        std::io::ErrorKind::NotFound => not_found(),
        std::io::ErrorKind::PermissionDenied => {
            resource::Response::text(403, Vec::new(), "403 Forbidden")
        }
        _ => resource::Response::text(500, Vec::new(), "500 Internal Server Error"),
    }
}

/// Serve one regular file's bytes typed by extension (the subset of Go
/// `serveContent` used here; no conditional or range handling).
fn file_response(file: &Path) -> resource::Response {
    match std::fs::metadata(file) {
        Ok(meta) if meta.is_dir() => return not_found(),
        Ok(_) => {}
        Err(error) => return file_error(&error),
    }
    match std::fs::read(file) {
        Ok(body) => resource::Response {
            status: 200,
            headers: vec![(
                "Content-Type",
                resource::mime_type(&file.to_string_lossy()).to_owned(),
            )],
            body,
        },
        Err(error) => file_error(&error),
    }
}

/// Go `http.FileServer(http.Dir(root))` behind `StripPrefix("/static/")`.
fn static_file(root: &str, path: &str, query: &str) -> resource::Response {
    let Some(stripped) = path.strip_prefix("/static/") else {
        return not_found();
    };
    let url = if stripped.starts_with('/') {
        stripped.to_owned()
    } else {
        format!("/{stripped}")
    };
    if url.ends_with("/index.html") {
        return local_redirect("./", query);
    }
    let name = clean(&url);
    let file = Path::new(root).join(name.trim_start_matches('/'));
    let meta = match std::fs::metadata(&file) {
        Ok(meta) => meta,
        Err(error) => return file_error(&error),
    };
    let base = |url: &str| {
        let trimmed = url.trim_end_matches('/');
        trimmed.rsplit('/').next().unwrap_or("").to_owned()
    };
    if meta.is_dir() {
        if !url.ends_with('/') {
            return local_redirect(&format!("{}/", base(&url)), query);
        }
        let index = file.join("index.html");
        return match std::fs::metadata(&index) {
            Ok(meta) if !meta.is_dir() => file_response(&index),
            _ => not_found(), // no directory listing (deviation)
        };
    }
    if url.ends_with('/') {
        return local_redirect(&format!("../{}", base(&url)), query);
    }
    file_response(&file)
}

/// Go `TrafficRecord` JSON (field order and `omitempty` as tagged).
#[derive(Serialize)]
struct TrafficView<'a> {
    ts: String,
    #[serde(rename = "sessionId")]
    session_id: &'a str,
    #[serde(rename = "remoteAddr")]
    remote_addr: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    ja3: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    ja4: &'a str,
    #[serde(rename = "ja4Engine", skip_serializing_if = "str::is_empty")]
    engine: &'a str,
    #[serde(rename = "extOrder", skip_serializing_if = "str::is_empty")]
    ext_order: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    alpn: &'a str,
    #[serde(rename = "tlsVersion", skip_serializing_if = "str::is_empty")]
    tls_version: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    proto: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    method: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    path: &'a str,
    #[serde(rename = "uaHash", skip_serializing_if = "str::is_empty")]
    ua_hash: &'a str,
    #[serde(rename = "headerOrderHash", skip_serializing_if = "str::is_empty")]
    header_order: &'a str,
    #[serde(rename = "hasSecFetch")]
    has_sec_fetch: bool,
}

fn rfc3339(tick: std::time::Duration) -> String {
    timestamp(tick)
        .0
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// Go `writeJSON(w, a.tlog.Session(sid))`: `null` when no history.
pub(super) fn traffic_json(records: &[Record]) -> resource::Response {
    if records.is_empty() {
        return resource::Response::json(&Value::Null);
    }
    let views: Vec<TrafficView<'_>> = records
        .iter()
        .map(|record| TrafficView {
            ts: rfc3339(record.time),
            session_id: &record.session_id,
            remote_addr: &record.remote_addr,
            ja3: &record.ja3,
            ja4: &record.ja4,
            engine: &record.engine,
            ext_order: &record.ext_order,
            alpn: &record.alpn,
            tls_version: &record.tls_version,
            proto: &record.proto,
            method: &record.method,
            path: &record.path,
            ua_hash: &record.ua_hash,
            header_order: &record.header_order,
            has_sec_fetch: record.has_sec_fetch,
        })
        .collect();
    resource::Response::json(&views)
}

/// Request facts every handler shares.
pub(super) struct Ctx<'a> {
    pub request: &'a Request,
    pub connection: &'a Connection,
    pub path: String,
    pub query: String,
    pub fields: Vec<(String, String)>,
    pub cookie: String,
    pub client_ip: String,
}

impl Ctx<'_> {
    pub fn header(&self, name: &str) -> &str {
        first_value(&self.fields, name)
    }
    pub fn authorization(&self) -> Option<&str> {
        Some(self.header("Authorization")).filter(|value| !value.is_empty())
    }
}

/// Headers a handler set before its reply (`Set-Cookie`, RIT rotation).
pub(super) type Early = Vec<(&'static str, String)>;

impl CoreServer {
    /// Bytes of body the route reads (Go's per-handler `MaxBytesReader`);
    /// zero for handlers that never read the body. The trace upload is read
    /// only once the operator is authorized and the method is POST.
    pub fn body_limit(&self, request: &Request) -> usize {
        match dispatch(request) {
            Some(Dispatch::Handle(Route::Collect | Route::Pass)) => super::COLLECT_BODY,
            Some(Dispatch::Handle(Route::Pow)) => pow::MAX_BODY,
            Some(Dispatch::Handle(Route::Trace))
                if request.method == "POST"
                    && operator_authorized(
                        &self.config.ops_token,
                        Some(first_value(&request.handler_fields(), "Authorization")),
                    ) =>
            {
                MAX_TRACE_BODY
            }
            _ => 0,
        }
    }

    /// Serve one request through Go's handler chain: security headers,
    /// traffic log, observability, then the mux.
    pub fn handle(&self, request: &Request, connection: &Connection) -> Response {
        let (Some(path), Some((_, query))) = (request.path(), request.split_target()) else {
            return finish(
                resource::Response::text(400, Vec::new(), "Bad Request"),
                Vec::new(),
            );
        };
        let fields = request.handler_fields();
        let ctx = Ctx {
            request,
            connection,
            query: query.to_owned(),
            cookie: cookie_value(&fields, demo::SESSION_COOKIE),
            client_ip: client_ip(&request.remote_addr, &fields, &self.config.trusted_proxies),
            path,
            fields,
        };
        if let Err(refused) = self.record_traffic(&ctx) {
            return finish(refused, Vec::new());
        }
        let outcome = catch_unwind(AssertUnwindSafe(|| self.route(&ctx)));
        let observed = match &self.logger {
            Some(logger) => match logger.lock() {
                Ok(mut logger) if logger.is_on() => {
                    let at = self.wall();
                    match observe(
                        &mut logger,
                        at,
                        &request.method,
                        HttpVersion::Http1,
                        move || outcome.unwrap_or_else(|panic| resume_unwind(panic)),
                    ) {
                        Completion::Completed(reply) => Some(reply),
                        Completion::PanicRecovered => Some((empty(500), Vec::new())),
                    }
                }
                _ => outcome.ok(),
            },
            None => outcome.ok(),
        };
        match observed {
            Some((reply, early)) => finish(reply, early),
            // Logging off: Go's server dropped the connection on a panic.
            None => Response {
                status: 0,
                headers: BTreeMap::new(),
                body: Vec::new(),
            },
        }
    }

    /// Go `withTrafficLog`.
    fn record_traffic(&self, ctx: &Ctx<'_>) -> Result<(), resource::Response> {
        let hello = ctx.connection.hello();
        if ctx.cookie.is_empty() && hello.is_none() {
            return Ok(());
        }
        let info = project_request(&ctx.request.method, 1, ctx.request.minor, &ctx.fields);
        let order = info.order();
        let order: Vec<&str> = order.iter().map(String::as_str).collect();
        let Some((mut state, now)) = self.lock() else {
            return Err(unavailable("history unavailable"));
        };
        let refused = |_| unavailable("history unavailable");
        if let Some(hello) = hello {
            state
                .traffic
                .record_tls(Record {
                    time: now.tick,
                    remote_addr: ctx.client_ip.clone(),
                    ja3: ja3(hello).0,
                    ja4: ja4(hello).0,
                    engine: engine_from_client_hello(Some(hello)).to_owned(),
                    ext_order: extension_order_hash(Some(hello)),
                    ..Record::default()
                })
                .map_err(refused)?;
        }
        if !ctx.cookie.is_empty() {
            state
                .traffic
                .record_http(Record {
                    time: now.tick,
                    session_id: ctx.cookie.clone(),
                    remote_addr: ctx.client_ip.clone(),
                    proto: "1.1".to_owned(),
                    method: ctx.request.method.clone(),
                    path: ctx.path.clone(),
                    ua_hash: hash_ua(&info.user_agent),
                    header_order: hash_order(&order),
                    has_sec_fetch: info.sec_fetch_present(),
                    ..Record::default()
                })
                .map_err(refused)?;
        }
        Ok(())
    }

    fn route(&self, ctx: &Ctx<'_>) -> (resource::Response, Early) {
        let method = ctx.request.method.as_str();
        let route = match dispatch(ctx.request) {
            Some(Dispatch::Handle(route)) => route,
            Some(Dispatch::Redirect(location)) => return (redirect(&location, method), Vec::new()),
            None => return (not_found(), Vec::new()),
        };
        match route {
            Route::Page if ctx.path != "/" => (not_found(), Vec::new()),
            Route::Page => self.page(ctx, "index.html"),
            Route::Demo => self.page(ctx, "demo.html"),
            Route::Session => self.session(ctx),
            Route::Collect => self.collect(ctx),
            Route::ReportOne => (self.report_one(ctx), Vec::new()),
            Route::ReportList => (self.report_list(ctx), Vec::new()),
            Route::Csp => (self.csp(ctx), Vec::new()),
            Route::Pow => (self.pow(ctx), Vec::new()),
            Route::Pass => (self.pass(ctx), Vec::new()),
            Route::Trace => (self.trace(ctx), Vec::new()),
            Route::Traffic => (self.traffic(ctx), Vec::new()),
            Route::Resource => self.resource(ctx),
            Route::Static => (
                static_file(&self.config.web_dir, &ctx.path, &ctx.query),
                Vec::new(),
            ),
            Route::Healthz => (self.healthz(), Vec::new()),
        }
    }

    /// Go `ensureSession`: the cookie session, or a fresh one with its
    /// `Set-Cookie`. `None` when entropy is unavailable (Go panicked).
    pub(super) fn ensure_session(&self, ctx: &Ctx<'_>, early: &mut Early) -> Option<String> {
        if !ctx.cookie.is_empty() {
            return Some(ctx.cookie.clone());
        }
        let sid = demo::new_session().ok()?;
        early.push(("Set-Cookie", demo::session_cookie(&sid)));
        Some(sid)
    }

    pub(super) fn locked(&self) -> Result<(MutexGuard<'_, State>, Now), resource::Response> {
        self.lock()
            .ok_or_else(|| unavailable("history unavailable"))
    }

    /// Go `handlePage` / `handleDemo`.
    fn page(&self, ctx: &Ctx<'_>, file: &str) -> (resource::Response, Early) {
        let mut early = Vec::new();
        let Some(sid) = self.ensure_session(ctx, &mut early) else {
            return (empty(500), Vec::new());
        };
        match self.locked() {
            Ok((mut state, now)) => state.store.ensure(&sid, now.ts),
            Err(refused) => return (refused, early),
        }
        let reply = file_response(&Path::new(&self.config.web_dir).join(file));
        (reply, early)
    }

    /// Go `handleSession`.
    fn session(&self, ctx: &Ctx<'_>) -> (resource::Response, Early) {
        let mut early = Vec::new();
        let Some(sid) = self.ensure_session(ctx, &mut early) else {
            return (empty(500), Vec::new());
        };
        let (mut state, now) = match self.locked() {
            Ok(locked) => locked,
            Err(refused) => return (refused, early),
        };
        state.store.ensure(&sid, now.ts);
        let mut body = json!({"sessionId": sid, "ritOn": self.config.rit_enabled});
        if self.config.rit_enabled {
            let (seed, n, window) =
                super::rit::issue_seed(&state.store, &self.config.master_key, &sid);
            body["ritSeed"] = Value::from(seed);
            body["ritN"] = Value::from(n);
            body["ritW"] = Value::from(window);
        }
        drop(state);
        early.push(("Accept-CH", ACCEPT_CH.to_owned()));
        (resource::Response::json(&body), early)
    }

    /// Go `handleReportOne`.
    fn report_one(&self, ctx: &Ctx<'_>) -> resource::Response {
        let id = ctx.path.strip_prefix("/api/report/").unwrap_or(&ctx.path);
        if !resource::can_read_session(
            Some(&ctx.cookie),
            id,
            &self.config.ops_token,
            ctx.authorization(),
        ) {
            return not_found();
        }
        let Ok((state, _)) = self.locked() else {
            return unavailable("history unavailable");
        };
        match state.store.get(id) {
            Some(report) => resource::Response::json(&report),
            None => not_found(),
        }
    }

    /// Go `handleReportList` (operator only).
    fn report_list(&self, ctx: &Ctx<'_>) -> resource::Response {
        if !operator_authorized(&self.config.ops_token, ctx.authorization()) {
            return not_found();
        }
        match self.locked() {
            Ok((state, _)) => resource::Response::json(&state.store.list(100)),
            Err(refused) => refused,
        }
    }

    /// Go `handleCSPReport`.
    fn csp(&self, ctx: &Ctx<'_>) -> resource::Response {
        if !ctx.cookie.is_empty() {
            let signal = Signal::new(
                "l3.guard.csp_violation",
                Value::Bool(true),
                Verdict::Suspicious,
                0.7,
                Source::Server,
                "CSP violation reported",
            );
            match self.locked() {
                Ok((mut state, now)) => {
                    state
                        .store
                        .append_network_signals(&ctx.cookie, vec![signal], now.ts);
                }
                Err(refused) => return refused,
            }
        }
        empty(204)
    }

    /// Go `handlePoW`.
    fn pow(&self, ctx: &Ctx<'_>) -> resource::Response {
        let (mut state, now) = match self.locked() {
            Ok(locked) => locked,
            Err(refused) => return refused,
        };
        let failed = Cell::new(false);
        let reply = pow::handle(
            &mut state.store,
            &self.config.master_key,
            &ctx.request.method,
            &ctx.cookie,
            &ctx.request.body,
            now.ts,
            |report| match self.engine.score(report) {
                Ok(result) => pow::Scored {
                    verdict: result.verdict,
                    risk_score: result.risk_score,
                },
                Err(_) => {
                    failed.set(true);
                    pow::Scored {
                        verdict: String::new(),
                        risk_score: f64::NAN,
                    }
                }
            },
        );
        if failed.get() {
            return unavailable("scoring unavailable");
        }
        reply
    }

    /// The Pass mount point; 404 when the Pass owner has not mounted it.
    fn pass(&self, ctx: &Ctx<'_>) -> resource::Response {
        match &self.pass {
            Some(pass) => pass.handle(&PassRequest {
                method: &ctx.request.method,
                path: &ctx.path,
                request: ctx.request,
                session: &ctx.cookie,
                client_ip: &ctx.client_ip,
            }),
            None => not_found(),
        }
    }

    /// Go `handleTrace` (operator only).
    fn trace(&self, ctx: &Ctx<'_>) -> resource::Response {
        let (_state, now) = match self.locked() {
            Ok(locked) => locked,
            Err(refused) => return refused,
        };
        resource::trace(
            &Tracer {
                ops_token: &self.config.ops_token,
                key: &self.watermark_key,
                ledger: &self.ledger,
                limits: self.config.limits.watermark_codec,
                now: now.tick,
                epoch_unix: 0,
            },
            ctx.authorization(),
            &ctx.request.method,
            ctx.header("Content-Type"),
            |_| Ok(ctx.request.body.clone()),
        )
    }

    /// Go `handleTraffic` (owner or operator).
    fn traffic(&self, ctx: &Ctx<'_>) -> resource::Response {
        let Some(sid) = resource::traffic_session(
            &ctx.path,
            Some(&ctx.cookie),
            &self.config.ops_token,
            ctx.authorization(),
        ) else {
            return not_found();
        };
        match self.locked() {
            Ok((state, _)) => traffic_json(&state.traffic.session(sid)),
            Err(refused) => refused,
        }
    }

    /// Go `handleResource`: gate, media/embed signals, watermark.
    fn resource(&self, ctx: &Ctx<'_>) -> (resource::Response, Early) {
        let mut early = Vec::new();
        let Some(sid) = self.ensure_session(ctx, &mut early) else {
            return (empty(500), Vec::new());
        };
        let Some(asset) = resource::asset_path(&ctx.path) else {
            return (resource::Response::text(400, Vec::new(), "bad path"), early);
        };
        let source = match resource::load_asset(&self.config.web_dir, &asset) {
            Ok(source) => source,
            Err(missing) => return (missing, early),
        };
        let mime = resource::mime_type(&asset);
        let request = ResourceRequest {
            session_id: &sid,
            headers: &ctx.fields,
            host: ctx.request.host(),
            rit_enforced: self.config.rit_enabled,
        };
        let (mut state, now) = match self.locked() {
            Ok(locked) => locked,
            Err(refused) => return (refused, early),
        };
        let stored = state.store.get(&sid);
        let view = stored.as_ref().map(|report| SessionView {
            verdict: &report.scoring.verdict,
            behavior: &report.client.behavior,
        });
        let gated = resource::gate_resource(&request, &asset, mime, source.len(), view);
        let signals = resource::resource_signals(&request, gated.tier, &self.media, now.ts.0);
        if !signals.is_empty() && !sid.is_empty() {
            state.store.append_network_signals(&sid, signals, now.ts);
        }
        if let Some(withheld) = resource::withheld_response(gated) {
            return (withheld, early);
        }
        let reply = resource::issue(
            &Issuer {
                key: &self.watermark_key,
                ledger: &self.ledger,
                limits: self.config.limits.watermark_codec,
                fallback: self.config.watermark_fallback,
                now: now.tick,
                bucket: day_bucket(now.unix()),
                ip_pseudonym: parse_ip(&ctx.client_ip)
                    .and_then(|ip| traffic_ip_hash(&self.config.master_key, ip)),
            },
            gated,
            &sid,
            &asset,
            mime,
            &source,
        );
        (reply, early)
    }

    /// Go `handleHealthz`.
    fn healthz(&self) -> resource::Response {
        let uptime = self.wall().saturating_sub(self.started).as_secs();
        resource::Response::json(&json!({
            "status": "ok",
            "uptimeSec": uptime,
            "version": self.config.version,
        }))
    }
}

/// Apply the security headers, the handler's early headers and its reply
/// headers with Go `Header.Set` semantics (`Set-Cookie` is added).
pub(super) fn finish(reply: resource::Response, early: Early) -> Response {
    let mut headers: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut put = |name: &str, value: String| {
        let name = canonical_name(name);
        if name == "Set-Cookie" {
            headers.entry(name).or_default().push(value);
        } else {
            headers.insert(name, vec![value]);
        }
    };
    for (name, value) in SECURITY_HEADERS {
        put(name, value.to_owned());
    }
    for (name, value) in early.into_iter().chain(reply.headers) {
        put(name, value);
    }
    Response {
        status: reply.status,
        headers,
        body: reply.body,
    }
}

/// `Date` value for a response written now.
pub(super) fn date_now(server: &CoreServer) -> String {
    http_date(i64::try_from(server.wall().as_secs()).unwrap_or(i64::MAX))
}

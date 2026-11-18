//! Legacy Core page/API router for the live gateway listener (legacy
//! cmd/server/server.go `routes()` at baseline 981d25c).
//!
//! Mounted only from an explicit [`LegacyPolicy`]; absent, every path keeps
//! its prior gateway behaviour. Paths and methods follow the Go mux: `/pass`
//! and `/api/pass/{new,pow,attest,solve,kpi}` dispatch every method to
//! [`PassFlow::handle`] (which enforces its own POST-only routes), `/demo` is
//! [`demo::handle`], and `/static/` serves files from the configured web
//! directory. Every response carries Go's `withSecurityHeaders` set.
//!
//! The application runs this after its exact `Host` authority check and
//! before RIT session authorization: like Go, these surfaces are public and
//! sessionless, so they do not consume gateway sessions or durable audit.
//!
//! Not mounted here, by design:
//! - The operator console (`/__hmn/admin/console`): Go served it only on the
//!   separate authenticated admin listener backed by the `/__hmn/admin/*` API
//!   ("NEVER mounted on the public proxied listener"). The Rust management
//!   listener is a native-peer API without that browser admin API or login,
//!   so the page stays unmounted rather than being exposed publicly.
//! - The detection observatory (`/playground*`): its SSE feed is a long-lived
//!   blocking stream that this serialized one-request-per-connection loop
//!   cannot serve without stalling all ingress, and its launcher needs a
//!   profile runner that Go implemented as a foreign-language shell-out.
//!
//! Intentional differences from Go:
//! - The client IP is the accepted loopback peer. The live gateway treats
//!   forwarding headers as observed claims, never transport identity, so no
//!   trusted-proxy rewrite exists to reuse.
//! - Without a captured TLS ClientHello the stable JA4 is the empty string,
//!   exactly Go's `ja4Stable(nil)`; nothing is synthesized.
//! - `/static/` never lists directories, refuses files outside the canonical
//!   web directory and files above [`MAX_FILE`].
//! - Pass reports live in a bounded in-process store; they do not yet feed the
//!   gateway's own decision path.
use crate::gateway::{
    Rejection,
    configuration::LegacyPolicy,
    demo, http,
    observatory::{Body, Request, Response},
    pass_flow::{
        ATTEST_PATH, Dependencies, KPI_PATH, NEW_PATH, PAGE_PATH, POW_PATH, PassContext, PassFlow,
        Reports, SOLVE_PATH, SystemClock, no_feed, scoring,
    },
};
use crate::network::tls::{ClientHello, ja4};
use humanymous_core::scoring::engine::Engine;
use humanymous_core::signals::report::SessionReport;
use std::{
    collections::HashMap,
    io::Read,
    net::IpAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// Static asset prefix (Go `http.StripPrefix("/static/", ...)`).
pub const STATIC_PREFIX: &str = "/static/";
/// Largest file a legacy response serves; it fits one complete TLS frame.
pub const MAX_FILE: usize = http::MAX_BODY * 2 - 4096;
/// Pass API paths, in Go mux order.
pub const PASS_API: [&str; 5] = [NEW_PATH, POW_PATH, ATTEST_PATH, SOLVE_PATH, KPI_PATH];

/// Go `withSecurityHeaders`.
const SECURITY_HEADERS: [(&str, &str); 3] = [
    (
        "Content-Security-Policy-Report-Only",
        "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; object-src 'none'; base-uri 'none'; report-uri /api/csp-report",
    ),
    ("X-Content-Type-Options", "nosniff"),
    ("Referrer-Policy", "strict-origin-when-cross-origin"),
];

/// Whether `path` belongs to any legacy surface, enabled or not.
pub fn surface_path(path: &str) -> bool {
    path == PAGE_PATH
        || PASS_API.contains(&path)
        || path == demo::PATH
        || path.starts_with(STATIC_PREFIX)
}

/// Whether `policy` mounts a surface at `path`.
pub fn claims(policy: &LegacyPolicy, path: &str) -> bool {
    (policy.pass && (path == PAGE_PATH || PASS_API.contains(&path)))
        || (policy.demo && path == demo::PATH)
        || (policy.static_assets && path.starts_with(STATIC_PREFIX))
}

/// Stable JA4 prefix (`a_b`) of a captured hello, or `""` when none was
/// captured (Go `ja4Stable`).
pub fn ja4_stable(hello: Option<&ClientHello>) -> String {
    let Some(hello) = hello else {
        return String::new();
    };
    let (hashed, _) = ja4(hello);
    match hashed.match_indices('_').nth(1) {
        Some((index, _)) => hashed[..index].to_owned(),
        None => hashed,
    }
}

/// Bounded in-process Pass report store (Go `a.store` for these surfaces).
/// New sessions beyond `capacity` are not retained and read as empty.
pub struct MemoryReports {
    capacity: usize,
    reports: Mutex<HashMap<String, SessionReport>>,
}

impl MemoryReports {
    /// Store holding at most `capacity` sessions.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            reports: Mutex::new(HashMap::new()),
        }
    }
    fn with<T>(&self, f: impl FnOnce(&mut HashMap<String, SessionReport>) -> T) -> T {
        f(&mut self.reports.lock().unwrap_or_else(|p| p.into_inner()))
    }
}

impl Reports for MemoryReports {
    fn ensure(&self, session: &str) {
        let capacity = self.capacity;
        self.with(|reports| {
            if !reports.contains_key(session) && reports.len() < capacity {
                reports.insert(session.to_owned(), SessionReport::default());
            }
        });
    }
    fn get(&self, session: &str) -> SessionReport {
        self.with(|reports| reports.get(session).cloned().unwrap_or_default())
    }
    fn store_scored(&self, session: &str, report: SessionReport) {
        let capacity = self.capacity;
        self.with(|reports| {
            if reports.contains_key(session) || reports.len() < capacity {
                reports.insert(session.to_owned(), report);
            }
        });
    }
}

/// The mounted surfaces.
pub struct Surfaces {
    root: PathBuf,
    pass: Option<PassFlow>,
    reports: Arc<MemoryReports>,
    demo: bool,
    static_assets: bool,
}

impl Surfaces {
    /// Validate the web directory and build the enabled surfaces. A missing
    /// page or directory refuses startup instead of serving 404s later.
    pub fn new(policy: &LegacyPolicy, engine: &Engine, capacity: usize) -> Result<Self, Rejection> {
        let bad = Rejection::new(503, "legacy_configuration");
        let root = policy.web_dir.canonicalize().map_err(|_| bad)?;
        if !root.is_dir()
            || policy.pass && !root.join("pass.html").is_file()
            || policy.demo && !root.join("demo.html").is_file()
        {
            return Err(bad);
        }
        let reports = Arc::new(MemoryReports::new(capacity));
        let pass = if policy.pass {
            // Go generated a fresh random 32-byte master key per process.
            let mut key = vec![0u8; 32];
            getrandom::fill(&mut key).map_err(|_| Rejection::new(503, "entropy"))?;
            Some(PassFlow::new(
                key,
                root.clone(),
                Dependencies {
                    engine: Arc::new(engine.clone()),
                    reports: reports.clone(),
                    clock: Arc::new(SystemClock::default()),
                    feed: no_feed(),
                    hub: None,
                    step_up: None,
                },
            ))
        } else {
            None
        };
        Ok(Self {
            root,
            pass,
            reports,
            demo: policy.demo,
            static_assets: policy.static_assets,
        })
    }

    /// Whether an enabled surface answers `path`.
    pub fn mounted(&self, path: &str) -> bool {
        (self.pass.is_some() && (path == PAGE_PATH || PASS_API.contains(&path)))
            || (self.demo && path == demo::PATH)
            || (self.static_assets && path.starts_with(STATIC_PREFIX))
    }

    /// Answer one request on a mounted path as complete HTTP/1.1 bytes;
    /// `None` leaves the path to the gateway.
    pub fn handle(
        &self,
        req: &http::Request,
        peer: Option<IpAddr>,
        hello: Option<&ClientHello>,
    ) -> Option<Result<Vec<u8>, Rejection>> {
        let (path, query) = match req.target.split_once('?') {
            Some((path, query)) => (path, Some(query)),
            None => (req.target.as_str(), None),
        };
        if !self.mounted(path) {
            return None;
        }
        let request = Request {
            method: &req.method,
            path,
            query,
            host: req.header("host").unwrap_or(""),
            origin: req.header("origin"),
            last_event_id: req.header("last-event-id"),
            cookie: req.header("cookie"),
            body: &req.body,
        };
        let response = if let Some(rel) = path.strip_prefix(STATIC_PREFIX) {
            Ok(self.static_file(rel))
        } else if path == demo::PATH {
            demo::handle(&request, &self.root, &|session| {
                self.reports.ensure(session);
            })
            .unwrap_or_else(|| Ok(Response::text(404, "404 page not found")))
            .or_else(|_| Ok(Response::text(500, "session unavailable")))
        } else {
            let pass = self.pass.as_ref()?;
            let client_ip = peer.map(|ip| ip.to_string()).unwrap_or_default();
            let fingerprint = scoring::fingerprint(&ja4_stable(hello), &client_ip);
            let context = PassContext {
                fingerprint: &fingerprint,
                redteam: req.header(scoring::REDTEAM_HEADER),
            };
            pass.handle(&request, &context)
                .ok_or(Rejection::new(503, "legacy_dispatch"))
        };
        Some(response.and_then(|response| encode(response, req.method == "HEAD", &self.root)))
    }

    fn static_file(&self, rel: &str) -> Response {
        if rel.is_empty() || rel.split('/').any(str::is_empty) {
            return Response::text(404, "404 page not found");
        }
        Response {
            status: 200,
            headers: Vec::new(),
            body: Body::File(self.root.join(rel)),
        }
    }

    /// The Pass surface, when mounted.
    pub fn pass(&self) -> Option<&PassFlow> {
        self.pass.as_ref()
    }
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "wasm" => "application/wasm",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Bounded read of a regular file under the canonical web directory.
fn read_file(root: &Path, path: &Path) -> Result<Option<Vec<u8>>, Rejection> {
    let Ok(canonical) = path.canonicalize() else {
        return Ok(None);
    };
    let Ok(metadata) = canonical.metadata() else {
        return Ok(None);
    };
    if !canonical.starts_with(root) || !metadata.is_file() {
        return Ok(None);
    }
    if metadata.len() > MAX_FILE as u64 {
        return Err(Rejection::new(503, "legacy_file_budget"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&canonical)
        .and_then(|file| file.take(MAX_FILE as u64 + 1).read_to_end(&mut bytes))
        .map_err(|_| Rejection::new(503, "legacy_file_read"))?;
    if bytes.len() > MAX_FILE {
        return Err(Rejection::new(503, "legacy_file_budget"));
    }
    Ok(Some(bytes))
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Content Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "Service Unavailable",
    }
}

/// Complete close-delimited HTTP/1.1 response bytes.
pub fn encode(response: Response, head: bool, root: &Path) -> Result<Vec<u8>, Rejection> {
    let Response {
        mut status,
        mut headers,
        body,
    } = response;
    let body = match body {
        Body::Bytes(bytes) => bytes,
        Body::File(path) => match read_file(root, &path)? {
            Some(bytes) => {
                if !headers
                    .iter()
                    .any(|(name, _)| name.eq_ignore_ascii_case("content-type"))
                {
                    headers.push(("Content-Type", content_type(&path).into()));
                }
                bytes
            }
            None => {
                let missing = Response::text(404, "404 page not found");
                let bytes = missing.bytes().map(<[u8]>::to_vec).unwrap_or_default();
                status = missing.status;
                headers = missing.headers;
                bytes
            }
        },
        Body::Events(_) => return Err(Rejection::new(503, "legacy_stream")),
    };
    let mut out = format!("HTTP/1.1 {status} {}\r\n", reason(status));
    for (name, value) in headers
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .chain(SECURITY_HEADERS)
    {
        if value.bytes().any(|b| !(32..127).contains(&b)) {
            return Err(Rejection::new(503, "legacy_header"));
        }
        out.push_str(&format!("{name}: {value}\r\n"));
    }
    out.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    let mut bytes = out.into_bytes();
    if !head {
        bytes.extend_from_slice(&body);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;

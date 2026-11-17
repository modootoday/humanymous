//! Lab-only Detection Observatory (legacy cmd/server playground.go and
//! launch.go at baseline 981d25c, SoT-30): a read-only live feed, the policy
//! constants a client renders bands from, a score explanation, and a local
//! launcher that fires one bundled catalog profile at the local engine.
//!
//! The surface exists only when `HMN_PLAYGROUND=1`, only on a loopback listen
//! address, and answers only loopback `Host`s (DNS-rebinding defence) with
//! anti-embed headers. It is transport-agnostic: an HTTP owner hands it a
//! parsed [`Request`] and writes the [`Response`]; nothing here opens sockets.
//!
//! Launcher safety is structural: the target is hard-coded loopback with no
//! override key, every launch needs a single-use two-minute nonce and a
//! loopback `Origin`, at most one launch is in flight, stateful detectors are
//! reset first, and abort cancels the run. The profile runner is injected: Go
//! shelled a foreign-language wrapper, which this port does not reproduce.
pub mod hub;

use hub::{Event, Hub, Subscription};
use humanymous_core::scoring::{engine::Engine, policy::Policy};
use humanymous_core::signals::report::SessionReport;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{self, Write},
    net::IpAddr,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

/// Environment switch (Go `playgroundEnabled`).
pub const ENABLE_ENV: &str = "HMN_PLAYGROUND";
/// The only target a launch may hit.
pub const LAUNCH_BASE: &str = "https://127.0.0.1:8443";
/// Nonce lifetime.
pub const NONCE_TTL: Duration = Duration::from_secs(120);
/// Per-launch deadline.
pub const LAUNCH_TIMEOUT: Duration = Duration::from_secs(90);
/// Largest launch request body.
pub const MAX_LAUNCH_BODY: usize = 1 << 12;
/// Most runs per launch.
pub const MAX_RUNS: u32 = 5;
/// SSE heartbeat cadence.
pub const HEARTBEAT: Duration = Duration::from_secs(15);

/// Bundled catalog profiles a launch may name (Go `launchProfiles`), kept in
/// lock-step with the red-team catalog by the parity test.
pub const LAUNCH_PROFILES: &[&str] = &[
    "human.mjs",
    "http_client.mjs",
    "tls_parrot.mjs",
    "curl_impersonate_chrome.mjs",
    "curl_impersonate_chrome99_android.mjs",
    "selenium.mjs",
    "puppeteer.mjs",
    "puppeteer_stealth.mjs",
    "playwright_plain.mjs",
    "playwright_stealth.mjs",
    "undetected.mjs",
    "patchright.mjs",
    "rebrowser_cdp_stripped.mjs",
    "mobile_ua_desktop_profile.mjs",
    "near_ceiling_audio_24k.mjs",
    "near_ceiling_no_widevine.mjs",
    "browser_use_cdp.mjs",
    "direct_cdp.mjs",
    "nodriver.mjs",
    "xvfb_headful.mjs",
    "antidetect.mjs",
    "camoufox.mjs",
    "tls_static.mjs",
    "pq_absent.mjs",
    "alps_absent.mjs",
    "cert_compression_absent.mjs",
    "ech_absent.mjs",
    "tls_rotate.mjs",
    "ua_rotate.mjs",
    "rit_replay.mjs",
    "rit_tamper.mjs",
    "video_scrape.mjs",
    "watermark_strip.mjs",
    "ai_agent.mjs",
    "distributed.mjs",
    "xff_spoof.mjs",
    "flood.mjs",
    "rapid_reset.mjs",
    "h2_flow_control.mjs",
    "h2_priority_absent.mjs",
    "signal_forgery.mjs",
    "privacy_evasion.mjs",
    "browser_tls_h1.mjs",
    "squid_forward.mjs",
    "openvpn_exit.mjs",
    "wireguard_hop.mjs",
    "tor_exit.mjs",
    "anon_proxy_chain.mjs",
    "elite_anon_proxy.mjs",
    "cdn_ip_spoof.mjs",
    "proxy_ua_rotate.mjs",
    "fp_churn_proxy.mjs",
    "stacked_proxy_vpn.mjs",
    "socks_exit_hop.mjs",
    "nonbrowser_ua.mjs",
    "sec_chua_absent.mjs",
    "sec_fetch_absent.mjs",
    "rit_absent.mjs",
    "ja4_churn.mjs",
    "multi_axis_rotate.mjs",
    "grease_absent_js.mjs",
    "behavior_untrusted.mjs",
    "behavior_no_interaction.mjs",
    "behavior_machine_keystroke.mjs",
    "behavior_teleport_click.mjs",
    "behavior_bezier_mouse.mjs",
    "behavior_fixed_typing.mjs",
    "ai_burst_silence.mjs",
    "ai_full_cadence.mjs",
    "adv_webgpu_mismatch.mjs",
    "headless_ua_token.mjs",
    "native_coherent_ceiling.mjs",
];

/// Catalog entries that need the isolated multi-container topology and are
/// never run by the local launcher (Go `dockerOnlyLaunchProfiles`).
pub const DOCKER_ONLY_PROFILES: &[&str] = &[
    "external_input_virtual",
    "external_input_dom_virtual",
    "external_input_usb",
    "external_input_dom_usb",
    "external_input_vusb",
    "external_input_dom_vusb",
];

/// Verdict bands in render order.
pub const STAGES: [&str; 7] = [
    "static client inspection",
    "client fingerprinting",
    "client integrity checks",
    "interaction analysis",
    "network and protocol inspection",
    "consistency checks",
    "risk aggregation and verdict selection",
];
/// Number of ordered Core hard rules the explanation evaluates.
pub const RULE_COUNT: usize = 24;

/// Whether the environment enables the observatory (exactly `"1"`).
pub fn enabled(value: Option<&str>) -> bool {
    value == Some("1")
}

/// Whether an HTTP `Host` names a loopback target: `localhost`, or a loopback
/// IP literal, with or without a port.
pub fn host_is_loopback(host: &str) -> bool {
    let name = split_host(host);
    name == "localhost" || name.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

fn split_host(host: &str) -> &str {
    if let Some(rest) = host.strip_prefix('[') {
        return rest.split_once(']').map_or(rest, |(inner, _)| inner);
    }
    match host.rsplit_once(':') {
        Some((name, port)) if !name.contains(':') && port.bytes().all(|b| b.is_ascii_digit()) => {
            name
        }
        _ => host,
    }
}

/// Whether a listen address binds only loopback. An empty host (`:8443`)
/// means every interface and is refused.
pub fn listen_addr_is_loopback(address: &str) -> bool {
    let name = split_host(address);
    !name.is_empty()
        && (name == "localhost" || name.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback()))
}

/// Whether an `Origin` header names a loopback host.
pub fn origin_is_loopback(origin: &str) -> bool {
    let Some((_, rest)) = origin.split_once("://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    !authority.is_empty() && host_is_loopback(authority)
}

/// Classify a run failure as a missing environment dependency rather than a
/// real error, so a client can grey the card honestly.
pub fn looks_like_missing_dependency(output: &str, error: &str) -> bool {
    let text = format!("{output} {error}").to_lowercase();
    [
        "cannot find",
        "not found",
        "err_module",
        "no such file",
        "executable file",
        "playwright",
        "browsertype",
        "econnrefused",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

/// Feed source label: a launched profile, or organic browsing.
pub fn live_source(label: Option<&str>) -> String {
    match label {
        Some(label) if !label.is_empty() => format!("profile:{label}"),
        _ => "organic".into(),
    }
}

/// Live-feed payload for one scored session (SoT-30 §4.2). A report carries
/// no key material, so nothing is redacted.
pub fn live_event(session: &str, source: &str, report: &SessionReport) -> Value {
    json!({
        "sessionId": session,
        "source": source,
        "ts": report.timestamp,
        "report": report,
    })
}

/// One SSE frame.
pub fn sse_frame(event: &Event) -> String {
    format!(
        "id: {}\nevent: {}\ndata: {}\n\n",
        event.id, event.kind, event.data
    )
}

/// Policy constants and bands a client renders from (`/playground/meta`).
pub fn meta(policy: &Policy) -> Value {
    json!({
        "policy": {
            "version": policy.version,
            "layerCap": policy.layer_cap,
            "challengeAt": policy.challenge_at,
            "denyAt": policy.deny_at,
        },
        "bands": [
            {"verdict": "ALLOW", "min": 0.0, "max": policy.challenge_at},
            {"verdict": "CHALLENGE", "min": policy.challenge_at, "max": policy.deny_at},
            {"verdict": "DENY", "min": policy.deny_at, "max": 100.0},
        ],
        "ruleCount": RULE_COUNT,
        "stages": STAGES,
    })
}

/// Single-use, short-lived launch nonces.
pub struct NonceStore {
    issued: Mutex<BTreeMap<String, Instant>>,
}

impl Default for NonceStore {
    fn default() -> Self {
        Self {
            issued: Mutex::new(BTreeMap::new()),
        }
    }
}

impl NonceStore {
    /// Issue a fresh 128-bit hex nonce valid for [`NONCE_TTL`].
    pub fn issue(&self) -> io::Result<String> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| io::Error::other("nonce entropy unavailable"))?;
        let token = hex::encode(bytes);
        let now = Instant::now();
        let mut issued = self.issued.lock().unwrap_or_else(|p| p.into_inner());
        issued.retain(|_, expiry| now <= *expiry);
        issued.insert(token.clone(), now + NONCE_TTL);
        Ok(token)
    }

    /// Consume a nonce once; false when empty, unknown, reused or expired.
    pub fn consume(&self, token: &str) -> bool {
        if token.is_empty() {
            return false;
        }
        let mut issued = self.issued.lock().unwrap_or_else(|p| p.into_inner());
        issued
            .remove(token)
            .is_some_and(|expiry| Instant::now() < expiry)
    }
}

/// Cancellation and deadline handed to a profile runner.
#[derive(Clone)]
pub struct RunControl {
    aborted: Arc<AtomicBool>,
    deadline: Instant,
}

impl RunControl {
    /// Whether the run was aborted or its deadline passed.
    pub fn cancelled(&self) -> bool {
        self.aborted.load(Ordering::SeqCst) || Instant::now() >= self.deadline
    }
}

/// Runs one profile once against `base`; `Err` carries the combined output and
/// error text.
pub type ProfileRunner =
    Arc<dyn Fn(&str, &str, &RunControl) -> Result<String, (String, String)> + Send + Sync>;

/// Supplies stored sessions for the explanation endpoint.
pub trait Sessions: Send + Sync {
    /// A stored report by session id.
    fn report(&self, session: &str) -> Option<SessionReport>;
}

/// Parsed request the HTTP owner hands over.
#[derive(Debug, Clone, Default)]
pub struct Request<'a> {
    /// HTTP method.
    pub method: &'a str,
    /// Path without query.
    pub path: &'a str,
    /// Raw query string, if any.
    pub query: Option<&'a str>,
    /// `Host` header.
    pub host: &'a str,
    /// `Origin` header.
    pub origin: Option<&'a str>,
    /// `Last-Event-ID` header.
    pub last_event_id: Option<&'a str>,
    /// Request body.
    pub body: &'a [u8],
}

/// What the HTTP owner writes back.
pub enum Body {
    /// Complete body bytes.
    Bytes(Vec<u8>),
    /// A file under the web directory to serve.
    File(PathBuf),
    /// A live event stream; see [`EventStream::serve`].
    Events(EventStream),
}

/// One observatory response.
pub struct Response {
    /// HTTP status.
    pub status: u16,
    /// Response headers.
    pub headers: Vec<(&'static str, String)>,
    /// Response body.
    pub body: Body,
}

impl Response {
    fn text(status: u16, message: &str) -> Self {
        Self {
            status,
            headers: vec![("Content-Type", "text/plain; charset=utf-8".into())],
            body: Body::Bytes(format!("{message}\n").into_bytes()),
        }
    }
    fn json(value: &impl Serialize) -> Self {
        Self {
            status: 200,
            headers: vec![("Content-Type", "application/json".into())],
            body: Body::Bytes(serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec())),
        }
    }
    fn guarded(mut self) -> Self {
        self.headers.extend([
            ("X-Frame-Options", "DENY".into()),
            ("Content-Security-Policy", "frame-ancestors 'none'".into()),
            ("Cache-Control", "no-store".into()),
        ]);
        self
    }
    /// Header value by name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
    /// Body bytes, when complete.
    pub fn bytes(&self) -> Option<&[u8]> {
        match &self.body {
            Body::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }
}

/// Live SSE stream: a retry directive, the replay backlog, then live events
/// and heartbeats until the reader goes away or the subscription ends.
pub struct EventStream {
    backlog: Vec<Event>,
    subscription: Subscription,
}

impl EventStream {
    /// Write the stream to `out`, flushing after each frame. Returns when a
    /// write fails (client gone) or the subscription is cancelled.
    pub fn serve(self, out: &mut impl Write, heartbeat: Duration) -> io::Result<()> {
        out.write_all(b"retry: 3000\n\n")?;
        for event in &self.backlog {
            out.write_all(sse_frame(event).as_bytes())?;
        }
        out.flush()?;
        loop {
            match self.subscription.recv_timeout(heartbeat) {
                Ok(event) => out.write_all(sse_frame(&event).as_bytes())?,
                Err(false) => out.write_all(b": ping\n\n")?,
                Err(true) => return Ok(()),
            }
            out.flush()?;
        }
    }
    /// The replayed backlog.
    pub fn backlog(&self) -> &[Event] {
        &self.backlog
    }
}

#[derive(Deserialize)]
struct LaunchRequest {
    #[serde(default, rename = "profileId")]
    profile: String,
    #[serde(default)]
    runs: i64,
    #[serde(default)]
    nonce: String,
    #[serde(default)]
    host: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    upstream: String,
}

/// The observatory surface; present only when enabled.
pub struct Observatory {
    hub: Arc<Hub>,
    nonces: NonceStore,
    web_dir: PathBuf,
    engine: Arc<Engine>,
    sessions: Arc<dyn Sessions>,
    runner: ProfileRunner,
    reset: Arc<dyn Fn() + Send + Sync>,
    in_flight: Arc<AtomicBool>,
    abort: Mutex<Option<Arc<AtomicBool>>>,
}

impl Observatory {
    /// Build the surface when `enable` is `"1"` and `listen` is loopback-only.
    /// Enabled off loopback is an error: the server must refuse to start.
    pub fn new(
        enable: Option<&str>,
        listen: &str,
        web_dir: PathBuf,
        engine: Arc<Engine>,
        sessions: Arc<dyn Sessions>,
        runner: ProfileRunner,
        reset: Arc<dyn Fn() + Send + Sync>,
    ) -> io::Result<Option<Self>> {
        if !enabled(enable) {
            return Ok(None);
        }
        if !listen_addr_is_loopback(listen) {
            return Err(io::Error::other(
                "the detection observatory requires a loopback-only listen address",
            ));
        }
        Ok(Some(Self {
            hub: Hub::new(0, 0),
            nonces: NonceStore::default(),
            web_dir,
            engine,
            sessions,
            runner,
            reset,
            in_flight: Arc::new(AtomicBool::new(false)),
            abort: Mutex::new(None),
        }))
    }

    /// The live-feed hub.
    pub fn hub(&self) -> &Arc<Hub> {
        &self.hub
    }

    /// Launch nonces.
    pub fn nonces(&self) -> &NonceStore {
        &self.nonces
    }

    /// Tap A: publish a scored session to the feed.
    pub fn publish_scored(&self, session: &str, label: Option<&str>, report: &SessionReport) {
        self.hub.publish(
            "session.scored",
            &live_event(session, &live_source(label), report),
        );
    }

    /// Answer a `/playground` request; `None` for paths outside the surface.
    pub fn handle(&self, request: &Request<'_>) -> Option<Response> {
        let path = request.path;
        if path != "/playground" && !path.starts_with("/playground/") {
            return None;
        }
        if !host_is_loopback(request.host) {
            return Some(Response::text(
                403,
                "detection observatory is loopback-only",
            ));
        }
        let response = match path {
            "/playground" => Response {
                status: 200,
                headers: Vec::new(),
                body: Body::File(self.web_dir.join("playground.html")),
            },
            "/playground/events" => self.events(request),
            "/playground/meta" => Response::json(&meta(&self.engine.policy)),
            "/playground/nonce" => match self.nonces.issue() {
                Ok(nonce) => Response::json(&json!({"nonce": nonce})),
                Err(_) => Response::text(503, "nonce unavailable"),
            },
            "/playground/launch" => self.launch(request),
            "/playground/abort" => {
                if let Some(abort) = self
                    .abort
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .as_ref()
                {
                    abort.store(true, Ordering::SeqCst);
                }
                Response::json(&json!({"aborted": true}))
            }
            _ => match path.strip_prefix("/playground/explain/") {
                Some(session) => self.explain(session),
                None => Response::text(404, "404 page not found"),
            },
        };
        Some(response.guarded())
    }

    fn events(&self, request: &Request<'_>) -> Response {
        let query_last = request.query.and_then(|query| {
            query
                .split('&')
                .find_map(|pair| pair.strip_prefix("lastId="))
        });
        let last_id = request
            .last_event_id
            .filter(|value| !value.is_empty())
            .or(query_last)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let (backlog, subscription) = self.hub.subscribe(last_id);
        Response {
            status: 200,
            headers: vec![
                ("Content-Type", "text/event-stream".into()),
                ("X-Accel-Buffering", "no".into()),
            ],
            body: Body::Events(EventStream {
                backlog,
                subscription,
            }),
        }
    }

    fn explain(&self, session: &str) -> Response {
        let Some(mut report) = self.sessions.report(session) else {
            return Response::text(404, "404 page not found");
        };
        // Scores a copy; the stored verdict is untouched.
        match self.engine.score_with_trace(&mut report) {
            Ok((_, trace)) => Response::json(&trace),
            Err(_) => Response::text(500, "score explanation unavailable"),
        }
    }

    fn launch(&self, request: &Request<'_>) -> Response {
        if request.method != "POST" {
            return Response::text(405, "POST only");
        }
        if request
            .origin
            .is_some_and(|origin| !origin.is_empty() && !origin_is_loopback(origin))
        {
            return Response::text(403, "cross-origin launch refused");
        }
        if request.body.len() > MAX_LAUNCH_BODY {
            return Response::text(400, "bad request");
        }
        let Ok(launch) = serde_json::from_slice::<LaunchRequest>(request.body) else {
            return Response::text(400, "bad request");
        };
        if !launch.host.is_empty() || !launch.url.is_empty() || !launch.upstream.is_empty() {
            return Response::text(
                400,
                "no target override permitted — the observatory targets local Blue only",
            );
        }
        if !self.nonces.consume(&launch.nonce) {
            return Response::text(403, "invalid or expired launch nonce");
        }
        if DOCKER_ONLY_PROFILES.contains(&launch.profile.as_str()) {
            return Response::text(
                409,
                "Docker-only profile — run it through the isolated external-input harness",
            );
        }
        if !LAUNCH_PROFILES.contains(&launch.profile.as_str()) {
            return Response::text(400, "unknown profile");
        }
        let runs = u32::try_from(launch.runs.clamp(1, i64::from(MAX_RUNS))).unwrap_or(1);
        if self
            .in_flight
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Response::text(429, "a launch is already in flight");
        }
        (self.reset)();
        let aborted = Arc::new(AtomicBool::new(false));
        *self.abort.lock().unwrap_or_else(|p| p.into_inner()) = Some(aborted.clone());
        let control = RunControl {
            aborted,
            deadline: Instant::now() + LAUNCH_TIMEOUT,
        };
        let (hub, runner, in_flight, profile) = (
            self.hub.clone(),
            self.runner.clone(),
            self.in_flight.clone(),
            launch.profile.clone(),
        );
        let spawned = thread::Builder::new()
            .name("observatory-launch".into())
            .spawn(move || {
                run_launch(&hub, &runner, &control, &profile, runs);
                in_flight.store(false, Ordering::SeqCst);
            });
        if spawned.is_err() {
            self.in_flight.store(false, Ordering::SeqCst);
            return Response::text(503, "launch worker unavailable");
        }
        Response::json(&json!({"accepted": true, "profileId": launch.profile, "runs": runs}))
    }
}

fn run_launch(hub: &Hub, runner: &ProfileRunner, control: &RunControl, profile: &str, runs: u32) {
    hub.publish(
        "attack.launched",
        &json!({"profile": profile, "runs": runs}),
    );
    for run in 0..runs {
        if control.cancelled() {
            hub.publish("attack.aborted", &json!({"profile": profile, "run": run}));
            break;
        }
        match runner(profile, LAUNCH_BASE, control) {
            Ok(output) => {
                hub.publish(
                    "attack.ran",
                    &json!({"profile": profile, "run": run, "out": output.trim()}),
                );
            }
            Err((output, error)) => {
                hub.publish(
                    "attack.error",
                    &json!({
                        "profile": profile,
                        "run": run,
                        "reason": format!("{} {error}", output.trim()),
                        "skipped": looks_like_missing_dependency(&output, &error),
                    }),
                );
            }
        }
    }
    hub.publish("attack.completed", &json!({"profile": profile}));
}

#[cfg(test)]
mod tests;

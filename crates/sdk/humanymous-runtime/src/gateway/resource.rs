//! Gated, watermarked resource serving and the forensic trace view (port of
//! legacy cmd/server/resource_handlers.go `handleResource`, `handleTrace`,
//! `handleTraffic` authorization, `recordResourceSignals`, `sessionVerdict`,
//! `sessionLiveness` and `externalReferer` at baseline 981d25c; SoT
//! resource-watermark W07/W09).
//!
//! These are the handler decisions over owner-supplied inputs: the asset bytes,
//! the session's stored verdict and behavior, the watermark key and ledger, the
//! media tracker and the clock. Mounting `/res/`, `/api/trace` and
//! `/api/traffic/` in a Core server, reading the asset root, appending the
//! returned signals to the session store and rendering the traffic log stay
//! with their runtime owners (plan B05/B07); nothing here changes a score,
//! verdict, threshold or signal identity, and the signals returned are the
//! frozen L5 media/embed signals of `integrity::resource`.
//!
//! Deliberate fail-closed differences from Go:
//! - W07: a watermark or ledger failure answers 503 with no body; Go served
//!   the ORIGINAL unmarked bytes with `X-HM-Watermark: none`. The ledger entry
//!   is committed before the marked bytes are returned, and a failed transform
//!   cancels its reservation.
//! - W09: only a verified carrier is reported as `traced: true`; a forged
//!   carrier is `traced: false` with reason `forged` (Go returned the record
//!   with `forged: true`). Unknown, expired, ambiguous and malformed carriers
//!   are `traced: false` with a stable reason code instead of Go's error text.
//! - The trace body bound is checked on the bytes actually read; Go's
//!   `MaxBytesReader` failure was a 400 `read error`, preserved here.
//! - MIME by extension uses Go's built-in `mime` table only (lowercased
//!   extension); Go additionally consulted host `mime.types` files, which made
//!   the served type host-dependent.
use super::operations::mlcorrect::{NOT_FOUND_BODY, operator_authorized};
use humanymous_core::integrity::resource::{
    Decision, EmbedContext, MediaEvent, MediaTracker, Tier, classify, gate, inspect,
};
use humanymous_core::model::go_json;
use humanymous_core::signals::observation::Signal;
use humanymous_core::signals::report::BehaviorSummary;
use humanymous_core::watermark::frame::Limits;
use humanymous_core::watermark::issuance::{Fallback, apply};
use humanymous_core::watermark::ledger::{Issuance, Ledger, Trace};
use humanymous_core::watermark::payload::{Payload, WatermarkKey};
use serde::Serialize;
use std::time::Duration;

/// Largest trace upload read (Go `MaxBytesReader(.., 32<<20)`).
pub const MAX_TRACE_BODY: usize = 32 << 20;

/// Legacy `resource.VerdictChallenge`: unknown trust is conservative on heavy tiers.
pub const UNKNOWN_VERDICT: &str = "CHALLENGE";

/// Handler outcome for the transport owner to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// HTTP status.
    pub status: u16,
    /// Response headers in the order Go set them.
    pub headers: Vec<(&'static str, String)>,
    /// Exact response body.
    pub body: Vec<u8>,
}

impl Response {
    fn text(status: u16, headers: Vec<(&'static str, String)>, body: &str) -> Self {
        let mut all = headers;
        all.push(("Content-Type", "text/plain; charset=utf-8".to_owned()));
        all.push(("X-Content-Type-Options", "nosniff".to_owned()));
        Self {
            status,
            headers: all,
            body: format!("{body}\n").into_bytes(),
        }
    }

    fn not_found() -> Self {
        Self {
            status: 404,
            headers: vec![
                ("Content-Type", "text/plain; charset=utf-8".to_owned()),
                ("X-Content-Type-Options", "nosniff".to_owned()),
            ],
            body: NOT_FOUND_BODY.to_vec(),
        }
    }

    fn json<T: Serialize>(value: &T) -> Self {
        let mut body = go_json::to_vec(value).unwrap_or_default();
        body.push(b'\n');
        Self {
            status: 200,
            headers: vec![("Content-Type", "application/json".to_owned())],
            body,
        }
    }

    /// First value of a header set by the handler.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

fn first<'a>(headers: &'a [(String, String)], name: &str) -> &'a str {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map_or("", |(_, value)| value.as_str())
}

/// Go `path.Clean`: collapse slashes, drop `.`, resolve `..` lexically.
fn clean(path: &str) -> String {
    let rooted = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|last| *last != "..") {
                    parts.pop();
                } else if !rooted {
                    parts.push("..");
                }
            }
            _ => parts.push(part),
        }
    }
    let joined = parts.join("/");
    match (rooted, joined.is_empty()) {
        (true, _) => format!("/{joined}"),
        (false, true) => ".".to_owned(),
        (false, false) => joined,
    }
}

/// Asset identity of a `/res/` request path (Go
/// `path.Clean("/" + strings.TrimPrefix(p, "/res/"))`), or `None` for Go's
/// `400 bad path` (any remaining `..`).
pub fn asset_path(url_path: &str) -> Option<String> {
    let rest = url_path.strip_prefix("/res/").unwrap_or(url_path);
    let asset = clean(&format!("/{rest}"));
    (!asset.contains("..")).then_some(asset)
}

/// Go `mime.TypeByExtension` over its built-in table, defaulting to
/// `application/octet-stream` as the legacy handler did.
pub fn mime_type(asset: &str) -> &'static str {
    let name = asset.rsplit('/').next().unwrap_or(asset);
    let ext = name
        .rfind('.')
        .map_or(String::new(), |dot| name[dot..].to_ascii_lowercase());
    match ext.as_str() {
        ".avif" => "image/avif",
        ".css" => "text/css; charset=utf-8",
        ".gif" => "image/gif",
        ".htm" | ".html" => "text/html; charset=utf-8",
        ".jpeg" | ".jpg" => "image/jpeg",
        ".js" | ".mjs" => "text/javascript; charset=utf-8",
        ".json" => "application/json",
        ".pdf" => "application/pdf",
        ".png" => "image/png",
        ".svg" => "image/svg+xml",
        ".wasm" => "application/wasm",
        ".webp" => "image/webp",
        ".xml" => "text/xml; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// The stored session report fields the resource gate reads.
#[derive(Debug, Clone, Copy)]
pub struct SessionView<'a> {
    /// Engine verdict wire label (`ALLOW`/`CHALLENGE`/`DENY`); empty when unscored.
    pub verdict: &'a str,
    /// Collected behavior summary.
    pub behavior: &'a BehaviorSummary,
}

/// Legacy `sessionVerdict`: the stored verdict, else conservative CHALLENGE.
pub fn session_verdict<'a>(session: Option<SessionView<'a>>) -> &'a str {
    match session {
        Some(view) if !view.verdict.is_empty() => view.verdict,
        _ => UNKNOWN_VERDICT,
    }
}

/// Legacy `sessionLiveness`: more than three pointer samples or any keystroke.
pub fn session_liveness(session: Option<SessionView<'_>>) -> bool {
    session.is_some_and(|view| view.behavior.mouse.samples > 3 || view.behavior.key.keystrokes > 0)
}

/// Legacy `externalReferer`: a non-empty Referer that does not contain the
/// request host (substring match, as in Go).
pub fn external_referer(referer: &str, host: &str) -> bool {
    !referer.is_empty() && !referer.contains(host)
}

/// One `/res/` request as seen by the handler.
#[derive(Debug, Clone, Copy)]
pub struct ResourceRequest<'a> {
    /// Session identity (legacy `ensureSession`); empty when none.
    pub session_id: &'a str,
    /// Request headers, first value per name wins.
    pub headers: &'a [(String, String)],
    /// Request `Host`.
    pub host: &'a str,
    /// Whether request-integrity tokens are enforced (legacy `a.ritOn`).
    pub rit_enforced: bool,
}

impl ResourceRequest<'_> {
    fn rit_present(&self) -> bool {
        !first(self.headers, "X-HM-Token").is_empty()
    }
}

/// Gate outcome for one asset (SoT-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gated {
    /// Classified tier.
    pub tier: Tier,
    /// Serving decision.
    pub decision: Decision,
}

impl Gated {
    /// Legacy `X-HM-Gate` value: `<decision>:<tier>`.
    pub fn header(self) -> String {
        format!("{}:{}", self.decision.as_str(), self.tier.as_str())
    }
}

/// Classify and gate an asset by session trust (legacy `handleResource`
/// gating block).
pub fn gate_resource(
    request: &ResourceRequest<'_>,
    asset: &str,
    mime: &str,
    size: usize,
    session: Option<SessionView<'_>>,
) -> Gated {
    let tier = classify(
        asset,
        mime,
        i64::try_from(size).unwrap_or(i64::MAX),
        first(request.headers, "Sec-Fetch-Dest"),
    );
    let rit_ok = !request.rit_enforced || request.rit_present();
    let decision = gate(
        tier,
        session_verdict(session),
        rit_ok,
        session_liveness(session),
    );
    Gated { tier, decision }
}

/// Legacy `recordResourceSignals`: embed inspection for every request plus
/// media observations for heavy and media tiers. The caller appends the
/// result to the session's network signals only when the session id is
/// non-empty and the result is non-empty (Go `AppendNetworkSignals`).
pub fn resource_signals(
    request: &ResourceRequest<'_>,
    tier: Tier,
    media: &MediaTracker,
    now: time::OffsetDateTime,
) -> Vec<Signal> {
    let rit_present = request.rit_present();
    let mut signals = inspect(&EmbedContext {
        sec_fetch_dest: first(request.headers, "Sec-Fetch-Dest"),
        origin: first(request.headers, "Origin"),
        allowed_hosts: &[], // demo: permit all origins
        rit_present,
    });
    if matches!(tier, Tier::Heavy | Tier::Media) {
        signals.extend(media.observe(&MediaEvent {
            session_id: request.session_id,
            is_range: !first(request.headers, "Range").is_empty(),
            external_ref: external_referer(first(request.headers, "Referer"), request.host),
            has_rit: rit_present,
            at: now,
        }));
    }
    signals
}

/// The response for a withheld decision, or `None` to serve the resource.
pub fn withheld_response(gated: Gated) -> Option<Response> {
    let gate_header = vec![("X-HM-Gate", gated.header())];
    match gated.decision {
        Decision::Serve => None,
        Decision::DenyServe => Some(Response::text(
            403,
            gate_header,
            "resource blocked for this session",
        )),
        Decision::Challenge => Some(Response::text(
            451,
            gate_header,
            "verification required for heavy resource",
        )),
        Decision::Downgrade => {
            let mut headers = gate_header;
            headers.push(("Content-Type", "text/plain; charset=utf-8".to_owned()));
            headers.push(("X-HM-Downgrade", "preview".to_owned()));
            Some(Response {
                status: 200,
                headers,
                body: b"[preview] full resource withheld until this session reaches the required trust level".to_vec(),
            })
        }
    }
}

/// Owner-supplied issuance context for a served resource.
pub struct Issuer<'a> {
    /// Server watermark key.
    pub key: &'a WatermarkKey,
    /// Bounded volatile issuance ledger.
    pub ledger: &'a Ledger,
    /// Codec admission limits.
    pub limits: Limits,
    /// Whether an opaque trailer may replace a failed/unsupported transform.
    pub fallback: Fallback,
    /// Monotonic instant for the ledger.
    pub now: Duration,
    /// Legacy `dayBucket()` (see `request::day_bucket`).
    pub bucket: u64,
    /// Keyed peer pseudonym (see `request::traffic_ip_hash`).
    pub ip_pseudonym: Option<String>,
}

/// Legacy `X-HM-Watermark` label for the channels actually written: one
/// channel is its name; PNG metadata plus pixels is Go's `png-multi`.
fn channel_label(channels: &[String]) -> String {
    match channels {
        [one] => one.clone(),
        [a, b] if a == "png-lsb" && b == "png-text" || a == "png-text" && b == "png-lsb" => {
            "png-multi".to_owned()
        }
        many => many.join("+"),
    }
}

/// Watermark and issue a gated-in resource (legacy `handleResource` after the
/// gate). The issuance is committed to the ledger before the marked bytes are
/// returned; any failure is 503 with no resource bytes (W07).
pub fn issue(
    issuer: &Issuer<'_>,
    gated: Gated,
    session_id: &str,
    asset: &str,
    mime: &str,
    source: &[u8],
) -> Response {
    let unavailable = || Response {
        status: 503,
        headers: vec![
            ("X-HM-Gate", gated.header()),
            ("Cache-Control", "no-store".to_owned()),
        ],
        body: Vec::new(),
    };
    let Ok(reservation) = issuer.ledger.reserve(issuer.now) else {
        return unavailable();
    };
    let payload = Payload {
        tag_id: reservation.tag_id(),
        session_id: session_id.to_owned(),
        asset_id: asset.to_owned(),
        bucket: issuer.bucket,
    };
    let applied = match apply(
        issuer.key,
        mime,
        source,
        &payload,
        issuer.fallback,
        issuer.limits,
    ) {
        Ok(applied) => applied,
        Err(_) => {
            let _ = reservation.cancel();
            return unavailable();
        }
    };
    let channels = applied.channel_names();
    let label = channel_label(&channels);
    let committed = reservation.commit(
        Issuance {
            payload,
            mime: mime.to_owned(),
            channels,
            ip_pseudonym: issuer.ip_pseudonym.clone(),
        },
        issuer.now,
    );
    if committed.is_err() {
        return unavailable();
    }
    Response {
        status: 200,
        headers: vec![
            ("X-HM-Gate", gated.header()),
            ("Content-Type", mime.to_owned()),
            ("Cache-Control", "no-store".to_owned()), // per-session unique
            ("X-HM-Watermark", label),
        ],
        body: applied.bytes,
    }
}

/// Owner-supplied trace context.
pub struct Tracer<'a> {
    /// Configured operator token; empty disables the route (404).
    pub ops_token: &'a str,
    /// Server watermark key.
    pub key: &'a WatermarkKey,
    /// Issuance ledger.
    pub ledger: &'a Ledger,
    /// Recovery limits.
    pub limits: Limits,
    /// Monotonic instant for the ledger.
    pub now: Duration,
    /// Wall-clock time (Unix seconds) of the ledger's monotonic zero, used only
    /// to display `issuedAt`.
    pub epoch_unix: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TraceResult {
    session_id: String,
    asset_id: String,
    mime: String,
    channel: String,
    issued_at: String,
    forged: bool,
}

#[derive(Serialize)]
struct Traced {
    result: TraceResult,
    traced: bool,
}

#[derive(Serialize)]
struct NotTraced {
    reason: &'static str,
    traced: bool,
}

fn not_traced(reason: &'static str) -> Response {
    Response::json(&NotTraced {
        reason,
        traced: false,
    })
}

fn rfc3339(epoch_unix: i64, offset: Duration) -> String {
    let nanos = i128::from(epoch_unix) * 1_000_000_000 + offset.as_nanos() as i128;
    time::OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .ok()
        .and_then(|at| {
            at.format(&time::format_description::well_known::Rfc3339)
                .ok()
        })
        .unwrap_or_default()
}

/// Legacy `handleTrace`: operator gate BEFORE the body is read, POST only,
/// bounded upload, then recover, resolve and verify. `read_body` reads at
/// most `limit + 1` bytes and is called only once authorization and method
/// have passed.
pub fn trace(
    tracer: &Tracer<'_>,
    authorization: Option<&str>,
    method: &str,
    content_type: &str,
    read_body: impl FnOnce(usize) -> std::io::Result<Vec<u8>>,
) -> Response {
    if !operator_authorized(tracer.ops_token, authorization) {
        return Response::not_found();
    }
    if method != "POST" {
        return Response::text(405, Vec::new(), "POST only");
    }
    let body = match read_body(MAX_TRACE_BODY) {
        Ok(body) if body.len() <= MAX_TRACE_BODY => body,
        _ => return Response::text(400, Vec::new(), "read error"),
    };
    let outcome = tracer
        .ledger
        .trace(tracer.key, content_type, &body, tracer.limits, tracer.now);
    match outcome {
        Ok(Trace::Verified(attribution)) => {
            let record = attribution.record;
            Response::json(&Traced {
                result: TraceResult {
                    session_id: record.payload().session_id.clone(),
                    asset_id: record.payload().asset_id.clone(),
                    mime: record.mime().to_owned(),
                    channel: attribution
                        .channels
                        .first()
                        .map_or_else(String::new, |channel| channel.name().to_owned()),
                    issued_at: rfc3339(tracer.epoch_unix, record.issued_at()),
                    forged: false,
                },
                traced: true,
            })
        }
        Ok(Trace::Forged(_)) => not_traced("forged"),
        Ok(Trace::Unknown { .. }) => not_traced("unknown"),
        Ok(Trace::Expired { .. }) => not_traced("expired"),
        Err(error) => not_traced(trace_reason(error)),
    }
}

fn trace_reason(error: humanymous_core::watermark::Error) -> &'static str {
    use humanymous_core::watermark::Error;
    match error {
        Error::NoFrame => "no_frame",
        Error::Ambiguous => "ambiguous",
        Error::ResourceLimit => "resource_limit",
        Error::InvalidCarrier => "invalid_carrier",
        Error::InvalidMime | Error::InvalidFormat | Error::UnsupportedFormat => "invalid_format",
        _ => "unavailable",
    }
}

/// Legacy `canReadSession` (cmd/server/ops.go), used by `/api/traffic/{sid}`:
/// the owner (its session cookie equals the non-empty path sid) or an
/// authenticated operator; anything else is a non-discoverable 404.
pub fn can_read_session(
    owner_session: Option<&str>,
    sid: &str,
    ops_token: &str,
    authorization: Option<&str>,
) -> bool {
    (!sid.is_empty() && owner_session == Some(sid)) || operator_authorized(ops_token, authorization)
}

/// Legacy `handleTraffic` gate: the session id from `/api/traffic/{sid}` and
/// whether the caller may read it. `None` means answer [`not_found`]; the
/// traffic-log projection itself belongs to the traffic-log owner.
pub fn traffic_session<'a>(
    url_path: &'a str,
    owner_session: Option<&str>,
    ops_token: &str,
    authorization: Option<&str>,
) -> Option<&'a str> {
    let sid = url_path.strip_prefix("/api/traffic/").unwrap_or(url_path);
    can_read_session(owner_session, sid, ops_token, authorization).then_some(sid)
}

/// Go `http.NotFound` response.
pub fn not_found() -> Response {
    Response::not_found()
}

#[cfg(test)]
mod tests;

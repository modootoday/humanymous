//! `/api/mlcorrect` operator snapshot (port of legacy cmd/server/mlcorrect_ops.go
//! `handleMLCorrect` and the cmd/server/ops.go `operatorAuthorized` gate it
//! uses, baseline 981d25c).
//!
//! Read-only and free of PII: it projects the lab correction controller's
//! calibration proposal, drift gate, Pass solve-rate guard, shadow divergence
//! and canary state plus the active bundle identity. It reports a disabled
//! marker (not 404) when no controller is loaded, so an operator can tell
//! "off" from "mis-authenticated"; a missing or wrong bearer token is a
//! non-discoverable 404, never 401. Nothing here mutates a controller, a
//! bundle lifecycle, a threshold or a verdict.
//!
//! The body is byte-identical to Go's `writeJSON` output (`encoding/json`
//! key order and number text, trailing newline). `fireThreshold` is the
//! controller's lab-only calibration proposal, shown for observability only.
//!
//! Mounting the route, transport and audit emission stay with the runtime
//! owner; this kernel is the handler decision and document.
use humanymous_core::model::correction::{
    BundleManager, CalibrationStats, CanaryStats, Controller, DriftEvent, ShadowStats,
};
use humanymous_core::model::go_json;
use serde::Serialize;
use subtle::ConstantTimeEq;

/// Disabled-marker reason, verbatim from Go.
pub const DISABLED_REASON: &str = "no -ml-bundle loaded; l4.ml.behavioral abstains";

/// Go `http.NotFound` body.
pub const NOT_FOUND_BODY: &[u8] = b"404 page not found\n";

/// Handler outcome for the transport owner to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// HTTP status: 200 or 404.
    pub status: u16,
    /// `Content-Type` header value.
    pub content_type: &'static str,
    /// Exact response body.
    pub body: Vec<u8>,
}

/// Legacy `operatorAuthorized`: the configured operator token must be
/// non-empty, and the `Authorization` value with one leading `Bearer `
/// stripped must equal it in constant time. As in Go, a header that is the bare
/// token (no `Bearer ` prefix) also matches.
pub fn operator_authorized(configured_token: &str, authorization: Option<&str>) -> bool {
    if configured_token.is_empty() {
        return false; // defense in depth: never authorize when unconfigured
    }
    let header = authorization.unwrap_or("");
    let presented = header.strip_prefix("Bearer ").unwrap_or(header);
    presented
        .as_bytes()
        .ct_eq(configured_token.as_bytes())
        .into()
}

/// Go `CalibrationStats` (no JSON tags: Go field names).
#[derive(Serialize)]
struct Calibration {
    #[serde(rename = "Theta")]
    theta: f32,
    #[serde(rename = "Budget")]
    budget: f32,
    #[serde(rename = "HumanFPEst")]
    human_fp_est: f32,
    #[serde(rename = "HumansSeen")]
    humans_seen: u64,
    #[serde(rename = "BotsSeen")]
    bots_seen: u64,
}

impl From<CalibrationStats> for Calibration {
    fn from(stats: CalibrationStats) -> Self {
        Self {
            theta: stats.theta,
            budget: stats.budget,
            human_fp_est: stats.human_fp_est,
            humans_seen: stats.humans_seen,
            bots_seen: stats.bots_seen,
        }
    }
}

/// Go `DriftEvent` (no JSON tags: Go field names).
#[derive(Serialize)]
struct Drift {
    #[serde(rename = "Fired")]
    fired: bool,
    #[serde(rename = "MimicAlarm")]
    mimic_alarm: bool,
    #[serde(rename = "OracleAlarm")]
    oracle_alarm: bool,
    #[serde(rename = "CovariateShift")]
    covariate_shift: bool,
    #[serde(rename = "MaxPSI")]
    max_psi: f64,
}

impl From<DriftEvent> for Drift {
    fn from(event: DriftEvent) -> Self {
        Self {
            fired: event.fired,
            mimic_alarm: event.mimic_alarm,
            oracle_alarm: event.oracle_alarm,
            covariate_shift: event.covariate_shift,
            max_psi: event.max_psi,
        }
    }
}

/// Go `ShadowStats`; the Rust-only `activeAbstained`/`rejected` counters are
/// not part of the legacy document.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Shadow {
    n: u64,
    both_scored: u64,
    mean_abs_delta: f64,
    fire_agreement: f64,
    shadow_fires_more: u64,
    active_fires_more: u64,
    shadow_abstained: u64,
}

impl From<ShadowStats> for Shadow {
    fn from(stats: ShadowStats) -> Self {
        Self {
            n: stats.n,
            both_scored: stats.both_scored,
            mean_abs_delta: stats.mean_abs_delta,
            fire_agreement: stats.fire_agreement,
            shadow_fires_more: stats.shadow_fires_more,
            active_fires_more: stats.active_fires_more,
            shadow_abstained: stats.shadow_abstained,
        }
    }
}

#[derive(Serialize)]
struct ActiveBundle {
    digest: String,
    version: String,
}

/// Go `map[string]any` document: keys in Go's sorted order.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Enabled {
    active_bundle: ActiveBundle,
    calibration: Calibration,
    canary: CanaryStats,
    drift: Drift,
    enabled: bool,
    fire_threshold: f32,
    pass_anomalous: bool,
    pass_solve_rate: f32,
    shadow: Shadow,
}

#[derive(Serialize)]
struct Disabled {
    enabled: bool,
    reason: &'static str,
}

fn json_response<T: Serialize>(value: &T) -> Response {
    // Every field is a finite number, bool or string: encoding cannot fail.
    let mut body = go_json::to_vec(value).unwrap_or_default();
    body.push(b'\n');
    Response {
        status: 200,
        content_type: "application/json",
        body,
    }
}

/// The snapshot document for an authorized operator (Go `writeJSON` bytes).
/// `bundles` is the bundle manager when one is configured: without one the
/// active identity is empty, with one but nothing active it is `none`.
pub fn snapshot(controller: Option<&Controller>, bundles: Option<&BundleManager>) -> Vec<u8> {
    let Some(controller) = controller else {
        return json_response(&Disabled {
            enabled: false,
            reason: DISABLED_REASON,
        })
        .body;
    };
    let snapshot = controller.snapshot();
    let active_bundle = match bundles.map(BundleManager::active) {
        None => ActiveBundle {
            digest: String::new(),
            version: String::new(),
        },
        Some(None) => ActiveBundle {
            digest: String::new(),
            version: "none".to_owned(),
        },
        Some(Some(identity)) => ActiveBundle {
            digest: identity.digest,
            version: identity.version,
        },
    };
    json_response(&Enabled {
        active_bundle,
        calibration: snapshot.calibration.into(),
        canary: snapshot.canary,
        drift: snapshot.drift.into(),
        enabled: true,
        fire_threshold: controller.proposed_fire_threshold(),
        pass_anomalous: snapshot.pass_anomalous,
        pass_solve_rate: snapshot.pass_solve_rate,
        shadow: snapshot.shadow.into(),
    })
    .body
}

/// Legacy `handleMLCorrect`: operator gate first, then the snapshot.
pub fn handle(
    configured_token: &str,
    authorization: Option<&str>,
    controller: Option<&Controller>,
    bundles: Option<&BundleManager>,
) -> Response {
    if !operator_authorized(configured_token, authorization) {
        return Response {
            status: 404,
            content_type: "text/plain; charset=utf-8",
            body: NOT_FOUND_BODY.to_vec(),
        };
    }
    Response {
        status: 200,
        content_type: "application/json",
        body: snapshot(controller, bundles),
    }
}

#[cfg(test)]
mod tests;

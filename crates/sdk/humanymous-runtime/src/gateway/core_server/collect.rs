//! The `/api/collect` pipeline in Go's load-bearing order (port of
//! cmd/server/handlers.go `handleCollect` and collect_pipeline.go at baseline
//! 981d25c; SoT-00 §5): RIT verify-and-rotate before the body is parsed,
//! network merge (pinned on the first collect), server-signal enrichment,
//! scoring, the live-feed publish, the external-input receipt and the PoW
//! challenge header.
use super::routes::{Ctx, Early};
use super::signals::webrtc_leak;
use super::{COLLECT_BODY, CoreServer, rit};
use crate::gateway::logging::{Level, Value as LogValue};
use crate::gateway::observatory::{live_event, live_source};
use crate::gateway::request::ja4_stable;
use crate::gateway::resource;
use crate::gateway::session::pow;
use crate::network::correlation::FleetInput;
use crate::network::history::rate::{Level as RateLevel, Observation as RateObservation};
use crate::network::http::{client_subnet, forwarded_ip, is_datacenter_ip, project_request};
use crate::network::report::{self, Observation};
use crate::network::residual;
use crate::network::traffic;
use humanymous_core::signals::observation::{Signal, Source, Verdict};
use humanymous_core::signals::report::ClientReport;
use serde_json::{Value, json};

/// Go `parseClientReport`: decode the report and bind the session id. A
/// JSON `null` body is an empty report, as in Go.
pub(super) fn parse_client(body: &[u8], sid: &str) -> Option<ClientReport> {
    let mut client = if body.trim_ascii() == b"null" {
        ClientReport::default()
    } else {
        serde_json::from_slice::<ClientReport>(body).ok()?
    };
    client.session_id = sid.to_owned();
    Some(client)
}

fn history_unavailable() -> resource::Response {
    resource::Response::text(503, Vec::new(), "history unavailable")
}

impl CoreServer {
    /// Go `handleCollect`.
    pub(super) fn collect(&self, ctx: &Ctx<'_>) -> (resource::Response, Early) {
        if ctx.request.method != "POST" {
            return (
                resource::Response::text(405, Vec::new(), "POST only"),
                Vec::new(),
            );
        }
        let mut early = Vec::new();
        let Some(sid) = self.ensure_session(ctx, &mut early) else {
            return (
                resource::Response {
                    status: 500,
                    headers: Vec::new(),
                    body: Vec::new(),
                },
                Vec::new(),
            );
        };
        let body = &ctx.request.body;
        if body.len() > COLLECT_BODY {
            return (
                resource::Response::text(400, Vec::new(), "read error"),
                early,
            );
        }
        let (mut state, now) = match self.locked() {
            Ok(locked) => locked,
            Err(refused) => return (refused, early),
        };
        let master = self.config.master_key.as_slice();
        // RIT before the body parse: a malformed body still consumes the grace.
        let rit_signal = self.config.rit_enabled.then(|| {
            let signal = rit::verify(
                &mut state.store,
                master,
                &sid,
                rit::Presented {
                    token: ctx.header("X-HM-Token"),
                    n: ctx.header("X-HM-N"),
                    tb: ctx.header("X-HM-TB"),
                },
                body,
                now.ts,
            );
            let (seed, n, window) = rit::issue_seed(&state.store, master, &sid);
            early.push(("X-HM-Seed", seed));
            early.push(("X-HM-N", n.to_string()));
            early.push(("X-HM-W", window.to_string()));
            signal
        });
        let Some(client) = parse_client(body, &sid) else {
            return (resource::Response::text(400, Vec::new(), "bad body"), early);
        };
        let label = ctx.request.query("label");
        if !label.is_empty() {
            state.store.set_label(&sid, &label, now.ts);
        }

        // mergeObservation
        let hello = ctx.connection.hello();
        let mut header = project_request(&ctx.request.method, 1, ctx.request.minor, &ctx.fields);
        if let Some(order) = ctx.connection.wire_order.clone() {
            order.apply(&mut header);
        }
        let datacenter = is_datacenter_ip(&ctx.client_ip, |ip| {
            self.intel.as_ref().is_some_and(|intel| {
                intel
                    .classify(Some(ip), u64::try_from(now.unix()).unwrap_or(0))
                    .datacenter
            })
        });
        let network = report::build(&Observation {
            hello: hello.cloned(),
            h2: None,
            header,
            is_datacenter_ip: datacenter,
            is_proxy: false,
            is_tor_exit: false,
            client_forwarded_ip: forwarded_ip(&ctx.fields),
            ..Observation::default()
        });
        state.store.merge_network(&sid, network, now.ts);
        let fingerprint = client.fingerprint_id.clone();
        let webrtc = client
            .advanced
            .web_rtc_public_addrs
            .clone()
            .unwrap_or_default();
        state.store.merge_client(&sid, client, now.ts);

        // enrichServerSignals
        let mut extra: Vec<Vec<Signal>> = Vec::new();
        if let Some(signal) = rit_signal {
            extra.push(vec![signal]);
        }
        match traffic::check(&state.traffic.session(&sid), state.traffic_limits) {
            Ok(outcome) if !outcome.signals.is_empty() => extra.push(outcome.signals),
            Ok(_) => {}
            Err(_) => return (history_unavailable(), early),
        }
        if let Some(abuse) = ctx.connection.abuse {
            extra.push(vec![Signal::new(
                abuse,
                Value::Bool(true),
                Verdict::Bot,
                1.0,
                Source::Server,
                "HTTP/2 frame-abuse DoS",
            )]);
        }
        let ja4 = ja4_stable(hello);
        let subnet = client_subnet(
            &ctx.request.remote_addr,
            &ctx.fields,
            &self.config.trusted_proxies,
        );
        let level = match state.limiter.observe(&format!("{ja4}|{subnet}"), now.tick) {
            Ok(RateObservation::Counted { level, .. }) => level,
            Ok(RateObservation::IgnoredEmptyKey) => RateLevel::Normal,
            Err(_) => return (history_unavailable(), early),
        };
        let rate = match level {
            RateLevel::Normal => None,
            RateLevel::Soft => Some(("l5.abuse.rate_exceeded", Verdict::Suspicious)),
            RateLevel::Hard => Some(("l5.abuse.flood", Verdict::Bot)),
        };
        if let Some((id, verdict)) = rate {
            extra.push(vec![Signal::new(
                id,
                Value::Null,
                verdict,
                1.0,
                Source::Server,
                "fingerprint-keyed request rate exceeded",
            )]);
        }
        if !fingerprint.is_empty() {
            let key = format!("{fingerprint}|{ja4}");
            match state
                .correlation
                .observe(&key, &subnet, &sid, now.tick, FleetInput::LocalOnly)
            {
                Ok(outcome) if !outcome.signals.is_empty() => extra.push(outcome.signals),
                Ok(_) => {}
                Err(_) => return (history_unavailable(), early),
            }
        }
        match state.fingerprints.observe(&sid, &fingerprint, now.tick) {
            Ok(Some(signal)) => extra.push(vec![signal]),
            Ok(None) => {}
            Err(_) => return (history_unavailable(), early),
        }
        if let Some(signal) = webrtc_leak(&webrtc, &ctx.client_ip) {
            extra.push(vec![signal]);
        }
        if state.store.pow_solved(&sid) {
            extra.push(vec![pow::solved_signal()]);
        }
        for signals in extra {
            state.store.append_network_signals(&sid, signals, now.ts);
        }

        // scoreAndStore
        let mut report = state.store.get(&sid).unwrap_or_default();
        let Ok(result) = self.engine.score(&mut report) else {
            return (
                resource::Response::text(503, Vec::new(), "scoring unavailable"),
                early,
            );
        };
        state.store.store_scored(&sid, report.clone(), now.ts);

        // publishScored (SoT-30 tap A), residuals never move risk.
        if let Some(hub) = &self.hub {
            hub.publish(
                "session.scored",
                &live_event(&sid, &live_source(Some(&label)), &report),
            );
            let short: String = sid.chars().filter(|c| !c.is_control()).take(8).collect();
            for record in residual::audit_records(report.network.signals.as_deref().unwrap_or(&[]))
            {
                hub.publish(
                    "network.residual",
                    &json!({
                        "sid": short,
                        "eventType": record.event_type,
                        "signalId": record.signal_id,
                        "verdict": record.verdict,
                        "notes": record.notes,
                        "scoreExempt": true,
                    }),
                );
            }
        }
        if let Some(receipts) = &self.receipts
            && receipts.write(&label, &sid, &result, &report).is_err()
        {
            if let Some(logger) = &self.logger
                && let Ok(mut logger) = logger.lock()
            {
                logger.emit(
                    self.wall(),
                    Level::Error,
                    "External-input evidence receipt failed.",
                    &[
                        ("component", LogValue::Str("core.measurement")),
                        ("event", LogValue::Str("measurement.receipt_failed")),
                        ("error_class", LogValue::Str("receipt_write")),
                    ],
                );
            }
            return (
                resource::Response::text(500, Vec::new(), "measurement receipt failed"),
                early,
            );
        }
        if let Some(challenge) = pow::issue_header(
            &mut state.store,
            master,
            &sid,
            &result.verdict,
            result.risk_score,
            now.ts,
        ) {
            early.push((pow::HEADER, challenge));
        }
        drop(state);
        if !result.risk_score.is_finite() {
            // Go's encoder failed after the header: content type only.
            return (
                resource::Response {
                    status: 200,
                    headers: vec![("Content-Type", "application/json".to_owned())],
                    body: Vec::new(),
                },
                early,
            );
        }
        let reply = json!({
            "sessionId": sid,
            "riskScore": result.risk_score,
            "verdict": result.verdict,
            "hardRuleFired": result.hard_rule_fired,
            "topContributors": serde_json::to_value(&result.top_contributors).unwrap_or(Value::Null),
            "policyVersion": result.policy_version,
        });
        (resource::Response::json(&reply), early)
    }
}

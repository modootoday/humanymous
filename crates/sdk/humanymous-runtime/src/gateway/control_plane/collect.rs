//! The collect beacon, one step per domain (legacy `internal/gate`
//! `control_collect.go` at baseline 981d25c, SoT-19 step 5): body parse,
//! beacon-replay defense (HR-29), scoring, the sticky verdict, the verdict
//! trust token (HR-28) and the audit records.
//!
//! Deliberate differences, each pinned by tests:
//! - The body is decoded by `serde_json` into the frozen `ClientReport` wire
//!   type. Go `encoding/json` also accepted duplicate keys (last wins),
//!   case-insensitive field names and invalid UTF-8 (replaced); here a
//!   duplicate key or invalid UTF-8 is `400 bad body` (fail-closed) and a
//!   differently cased key is ignored. A JSON `null` body is an empty report,
//!   as in Go.
//! - A verdict token whose fields cannot be sealed (for example a session id
//!   containing `|`) is not issued; Go minted it anyway.
use super::{
    Backend, ControlPlane, MAX_BEACON, NONCE_HEADER, Now, Request, Response, VERDICT_COOKIE,
    control_record, cookie, headers, token_bind, unix_seconds,
};
use crate::audit::record::events::{
    EVENT_BEACON_REPLAY, EVENT_SCORING_EVALUATED, EVENT_TOKEN_ISSUED,
};
use crate::audit::record::{Actor, Record, Signal};
use crate::gateway::verdict_store::{Sticky, Verdict};
use crate::network::history::cohort::Cohort;
use crate::network::report::{self, Observation};
use crate::network::residual;
use humanymous_core::signals::report::{ClientReport, ScoreResult};
use humanymous_core::signals::wire::{json_float_go, quote_json_go};

/// Go `topSignals`: the first five contributors as BOT with confidence one.
pub(super) fn top_signals(result: &ScoreResult) -> Vec<Signal> {
    result
        .top_contributors
        .iter()
        .flatten()
        .take(5)
        .map(|contributor| Signal {
            id: contributor.id.clone(),
            verdict: "BOT".into(),
            conf: 1.0,
        })
        .collect()
}

/// Go `ruleList`.
fn rule_list(rule: &str) -> Vec<String> {
    if rule.is_empty() {
        Vec::new()
    } else {
        vec![rule.to_owned()]
    }
}

/// Go `parseControlClient`: decode the beacon and bind the session id.
pub(super) fn parse_client(body: &[u8], sid: &str) -> Option<ClientReport> {
    let mut client = if body.trim_ascii() == b"null" {
        ClientReport::default()
    } else {
        serde_json::from_slice::<ClientReport>(body).ok()?
    };
    client.session_id = sid.to_owned();
    Some(client)
}

/// Go `writeControlVerdict`. A non-finite score leaves the body empty, as Go
/// `json.Encoder` does after the header is written.
pub(super) fn verdict_body(sid: &str, result: &ScoreResult) -> String {
    let Some(risk) = json_float_go(result.risk_score) else {
        return String::new();
    };
    format!(
        "{{\"hardRuleFired\":{},\"riskScore\":{risk},\"sessionId\":{},\"verdict\":{}}}\n",
        quote_json_go(&result.hard_rule_fired),
        quote_json_go(sid),
        quote_json_go(&result.verdict),
    )
}

/// Go `updateStickyVerdict` input.
pub(super) fn sticky(result: &ScoreResult, now: Now) -> Sticky {
    Sticky {
        verdict: Verdict::from_engine(&result.verdict),
        risk: result.risk_score,
        rule: result.hard_rule_fired.clone(),
        top: top_signals(result)
            .into_iter()
            .map(|signal| signal.id)
            .collect(),
        updated: now.mono,
    }
}

impl<B: Backend> ControlPlane<B> {
    /// Go `handleCollect`.
    pub(super) fn collect(&mut self, request: &Request<'_>, now: Now) -> Response {
        if request.method != "POST" {
            return Response::error(405, "POST only");
        }
        if let Some(refusal) = self.metered(request, "collect", now) {
            return refusal;
        }
        let (sid, mut cookies) = match self.ensure_session(request) {
            Ok(found) => found,
            Err(refusal) => return refusal,
        };
        if request.body.len() > MAX_BEACON {
            return Response::error(400, "read error").with_cookies(cookies);
        }
        let Some(client) = parse_client(request.body, &sid) else {
            return Response::error(400, "bad body").with_cookies(cookies);
        };
        if let Some(refusal) = self.reject_beacon_replay(request, &sid, now) {
            return refusal.with_cookies(cookies);
        }
        let Some(unix) = unix_seconds(now.wall) else {
            return Response::error(503, "clock unavailable").with_cookies(cookies);
        };
        let mut records = Vec::new();
        let (result, cohort) = match self.score_beacon(request, &sid, client, now) {
            Some(scored) => scored,
            None => return Response::error(503, "scoring unavailable").with_cookies(cookies),
        };
        self.verdicts.set(&sid, sticky(&result, now));
        if let Some((set, record)) = self.verdict_token_on_allow(request, &sid, &result, unix) {
            cookies.push(set);
            records.push(record);
        }
        records.extend(self.link_and_audit_scoring(request, &sid, &result));
        let mut response = Response::json(200, verdict_body(&sid, &result)).with_cookies(cookies);
        response.records = records;
        response.cohort = cohort;
        response
    }

    /// Go `rejectBeaconReplay`: an absent nonce is not a replay; a consumed
    /// one is audited and refused with `409`.
    fn reject_beacon_replay(
        &mut self,
        request: &Request<'_>,
        sid: &str,
        now: Now,
    ) -> Option<Response> {
        let nonce = request.header(NONCE_HEADER);
        if nonce.is_empty() || self.nonces.use_once(nonce, now.mono) {
            return None;
        }
        let mut response = Response::error(409, "replay rejected");
        response.records.push(Record {
            event_type: EVENT_BEACON_REPLAY.into(),
            actor: self.subject(sid, sid),
            verdict: "deny".into(),
            triggered_rules: Some(vec!["HR-29".into()]),
            enforcement_action: "block".into(),
            enforcement_mode: "enforce".into(),
            fail_reason: "beacon nonce replay".into(),
            ..control_record()
        });
        Some(response)
    }

    /// Go `scoreBeacon`: merge the network observation and the client report,
    /// feed the cohort shadow, apply the label, score and persist.
    fn score_beacon(
        &mut self,
        request: &Request<'_>,
        sid: &str,
        client: ClientReport,
        now: Now,
    ) -> Option<(ScoreResult, Option<Cohort>)> {
        // Go: network.Build(Observation{Header: headerInfo(r), IsDatacenterIP: false}).
        let network = report::build(&Observation {
            header: headers::header_info(request),
            ..Observation::default()
        });
        self.store.merge_network(sid, network, now.wall);
        let cohort = match &mut self.cohort {
            // Log-only measurement; an observer refusal never affects the verdict.
            Some(observer) => observer
                .observe(&token_bind(request), &client.behavior, sid, now.mono)
                .ok(),
            None => None,
        };
        self.store.merge_client(sid, client, now.wall);
        if let Some(label) = request.label.filter(|label| !label.is_empty()) {
            self.store.set_label(sid, label, now.wall);
        }
        let mut report = self.store.get(sid)?;
        let result = self.backend.score(&mut report)?;
        self.store.store_scored(sid, report, now.wall);
        Some((result, cohort))
    }

    /// Go `issueVerdictTokenOnAllow`.
    fn verdict_token_on_allow(
        &mut self,
        request: &Request<'_>,
        sid: &str,
        result: &ScoreResult,
        unix: u64,
    ) -> Option<(String, Record)> {
        if result.verdict != "ALLOW" {
            return None;
        }
        let epoch = self.epoch(unix);
        let key = self.token_key.as_ref()?;
        let issued = key
            .issue_verdict(
                sid,
                &token_bind(request),
                &epoch,
                unix.saturating_add(super::TOKEN_TTL),
            )
            .ok()?;
        let record = Record {
            event_type: EVENT_TOKEN_ISSUED.into(),
            actor: self.subject(sid, sid),
            verdict: "allow".into(),
            enforcement_mode: "enforce".into(),
            ..control_record()
        };
        Some((cookie(VERDICT_COOKIE, &issued), record))
    }

    /// Go `linkAndAuditScoring` and `emitNetworkResidualAudit`.
    fn link_and_audit_scoring(
        &mut self,
        request: &Request<'_>,
        sid: &str,
        result: &ScoreResult,
    ) -> Vec<Record> {
        let handle = self.pseudo(sid, sid);
        self.backend.link(&handle, sid);
        let psn = self.pseudo(sid, &request.peer.to_string());
        let mut records = vec![Record {
            event_type: EVENT_SCORING_EVALUATED.into(),
            actor: Actor {
                kind: "subject".into(),
                id_pseudonym: psn.clone(),
            },
            session_pseudonym: handle,
            verdict: Verdict::from_engine(&result.verdict).as_str().into(),
            // Go int(float64) truncates toward zero.
            risk_score: result.risk_score as i64,
            triggered_rules: Some(rule_list(&result.hard_rule_fired)),
            top_signals: Some(top_signals(result)),
            enforcement_mode: "enforce".into(),
            ..control_record()
        }];
        if let Some(report) = self.store.get(sid) {
            let signals = report.network.signals.unwrap_or_default();
            records.extend(self.network_residual_records(sid, &psn, &signals));
        }
        records
    }

    /// Go `emitNetworkResidualAudit`: one observational record per mapped
    /// score-exempt residual.
    pub(super) fn network_residual_records(
        &mut self,
        sid: &str,
        psn: &str,
        signals: &[humanymous_core::signals::observation::Signal],
    ) -> Vec<Record> {
        residual::audit_records(signals)
            .into_iter()
            .map(|residual| Record {
                event_type: residual.event_type,
                actor: Actor {
                    kind: "subject".into(),
                    id_pseudonym: psn.to_owned(),
                },
                session_pseudonym: self.pseudo(sid, sid),
                verdict: "none".into(),
                risk_score: 0,
                triggered_rules: Some(vec![residual.signal_id.clone()]),
                top_signals: Some(vec![Signal {
                    id: residual.signal_id,
                    verdict: residual.verdict,
                    conf: 1.0,
                }]),
                fail_reason: residual.notes,
                enforcement_mode: "monitor".into(),
                ..control_record()
            })
            .collect()
    }
}

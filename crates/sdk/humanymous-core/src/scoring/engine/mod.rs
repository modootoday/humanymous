//! One orchestration path for observation derivation, combination and decisions.
use crate::model::inference::Runtime;
use crate::scoring::{
    combination::{Contribution, combine, top_contributors},
    consistency::{cross_checks, user_agent::is_browser_ua},
    derivation::{advanced_signals, behavior_signals, environment_signals, guard_signals},
    policy::{Policy, SessionVerdict},
    promotion::{self, RuleContext},
    trace::{ContributionTrace, PolicyView, ScoreTrace},
};
use crate::signals::{
    observation::{Layer, group_of},
    report::{ScoreResult, SessionReport},
};
use std::collections::BTreeMap;

/// Invalid arithmetic cannot become a numeric trust verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringError {
    /// A configured numeric policy value is not finite.
    NonFinitePolicy,
    /// A contribution is not finite before or after an overlay.
    NonFiniteContribution,
}
impl std::fmt::Display for ScoringError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::NonFinitePolicy => "non-finite scoring policy",
            Self::NonFiniteContribution => "non-finite scoring contribution",
        })
    }
}
impl std::error::Error for ScoringError {}

/// Shared L6/L7 authority. Consumers configure one instance rather than copying
/// verdict logic into enforcement or HTTP handlers.
#[derive(Clone, Default)]
pub struct Engine {
    /// Versioned numeric policy.
    pub policy: Policy,
    /// Hard-rule enforce/monitor overlays; absent and empty values enforce.
    pub rule_modes: BTreeMap<String, String>,
    /// Per-signal multipliers, defensively clamped to zero through two.
    pub weight_multipliers: BTreeMap<String, f64>,
    /// None preserves code defaults. An explicit map preserves partial-overlay semantics.
    pub net_policy: Option<BTreeMap<String, String>>,
    /// Shared active/shadow model owner.
    pub model: Runtime,
}

impl Engine {
    /// Score a merged report, replacing its cross-check and scoring fields.
    pub fn score(&self, report: &mut SessionReport) -> Result<ScoreResult, ScoringError> {
        self.evaluate(report, false).map(|(result, _)| result)
    }
    /// Score through the same path, retaining the additive explanation.
    pub fn score_with_trace(
        &self,
        report: &mut SessionReport,
    ) -> Result<(ScoreResult, ScoreTrace), ScoringError> {
        self.evaluate(report, true)
            .map(|(result, trace)| (result, trace.expect("trace requested")))
    }
    fn evaluate(
        &self,
        report: &mut SessionReport,
        traced: bool,
    ) -> Result<(ScoreResult, Option<ScoreTrace>), ScoringError> {
        if ![
            self.policy.challenge_at,
            self.policy.deny_at,
            self.policy.layer_cap,
        ]
        .iter()
        .all(|value| value.is_finite())
            || !self
                .weight_multipliers
                .values()
                .all(|value| value.is_finite())
        {
            return Err(ScoringError::NonFinitePolicy);
        }
        let (contributions, context) = self.assemble(report)?;
        let combined = combine(&contributions, &self.policy, traced);
        if !combined.risk.is_finite() {
            return Err(ScoringError::NonFiniteContribution);
        }
        let mut verdict = self.policy.verdict_for(combined.risk);
        let mut fired = "";
        if let Some((override_verdict, rule)) = promotion::apply(&context) {
            verdict = override_verdict;
            fired = rule;
        }
        if verdict == SessionVerdict::Challenge
            && fired.is_empty()
            && server_signal(report, "l7.pow.solved")
        {
            verdict = SessionVerdict::Allow;
            fired = "PoW-upgrade";
        }
        if verdict == SessionVerdict::Challenge
            && fired.is_empty()
            && server_signal(report, "l7.pass.solved")
        {
            verdict = SessionVerdict::Allow;
            fired = "Pass-upgrade";
        }
        let result = ScoreResult {
            risk_score: ((combined.risk * 10.0 + 0.5) as i64) as f64 / 10.0,
            verdict: verdict.as_str().into(),
            hard_rule_fired: fired.into(),
            top_contributors: Some(top_contributors(&contributions, 5)),
            policy_version: self.policy.version.clone(),
        };
        report.scoring = result.clone();
        let trace = traced.then(|| ScoreTrace {
            contributions: contributions
                .iter()
                .filter(|contribution| contribution.score > 0.0)
                .map(|contribution| ContributionTrace {
                    id: contribution.id.clone(),
                    layer: contribution.layer.as_str().into(),
                    group: contribution.group.into(),
                    score: contribution.score,
                })
                .collect(),
            dedup_dropped: combined.drops,
            per_layer: combined.layers,
            hard_rules: promotion::evaluations(&context),
            score: result.risk_score,
            verdict: result.verdict.clone(),
            hard_rule_fired: result.hard_rule_fired.clone(),
            policy: PolicyView {
                version: self.policy.version.clone(),
                layer_cap: self.policy.layer_cap,
                challenge_at: self.policy.challenge_at,
                deny_at: self.policy.deny_at,
            },
        });
        Ok((result, trace))
    }
    fn assemble(
        &self,
        report: &mut SessionReport,
    ) -> Result<(Vec<Contribution>, RuleContext<'_>), ScoringError> {
        let has_client = !report.client.user_agent.is_empty()
            || report
                .client
                .signals
                .as_ref()
                .is_some_and(|signals| !signals.is_empty())
            || report.client.environment.probed;
        let behavior = behavior_signals(&report.client.behavior, &self.model);
        let environment = environment_signals(&report.client.environment, has_client);
        let guard = guard_signals(&report.client.guard);
        let advanced = advanced_signals(&report.client.advanced, &report.client.user_agent);
        let cross = cross_checks(report);
        report.cross_checks = (!cross.is_empty()).then(|| cross.clone());
        let mut contributions = Vec::new();
        for signal in report
            .client
            .signals
            .iter()
            .flatten()
            .chain(report.network.signals.iter().flatten())
            .chain(&behavior)
            .chain(&environment.signals)
            .chain(&guard)
            .chain(&advanced)
        {
            contributions.push(Contribution {
                id: signal.id.clone(),
                layer: signal.layer.clone(),
                group: group_of(&signal.id),
                score: signal.score,
            });
        }
        for check in &cross {
            if check.score > 0.0 {
                contributions.push(Contribution {
                    id: check.id.clone(),
                    layer: Layer::Consistency,
                    group: "",
                    score: check.score,
                });
            }
        }
        let privacy_noise = contributions.iter().any(|contribution| {
            matches!(
                contribution.id.as_str(),
                "l2.canvas.tampered" | "l2.webgl.param_consistency"
            )
        });
        for contribution in &mut contributions {
            if !contribution.score.is_finite() {
                return Err(ScoringError::NonFiniteContribution);
            }
            if let Some(multiplier) = self.weight_multipliers.get(&contribution.id) {
                contribution.score *= multiplier.clamp(0.0, 2.0);
            }
            if privacy_noise && matches!(contribution.group, "canvas" | "webgl") {
                contribution.score *= 0.5;
            }
            if environment.privacy
                && matches!(contribution.group, "canvas" | "webgl" | "audio" | "font")
            {
                contribution.score *= 0.5;
            }
            if !contribution.score.is_finite() {
                return Err(ScoringError::NonFiniteContribution);
            }
        }
        let mut signals = report.all_signals();
        signals.extend(behavior);
        signals.extend(environment.signals);
        signals.extend(guard);
        signals.extend(advanced);
        Ok((
            contributions,
            RuleContext {
                signals,
                cross,
                has_client,
                browser_claim: is_browser_ua(&report.client.user_agent),
                modes: &self.rule_modes,
                net_policy: self.net_policy.as_ref(),
            },
        ))
    }
}
fn server_signal(report: &SessionReport, id: &str) -> bool {
    report.network.signals.iter().flatten().any(|signal| {
        signal.id == id && signal.collected.as_str() == "server" && signal.verdict.as_str() == "OK"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::observation::{Signal, Source, Verdict};
    #[test]
    fn no_client_is_challenged_and_artifacts_win_before_missing_client() {
        let engine = Engine::default();
        let mut report = SessionReport::default();
        assert_eq!(engine.score(&mut report).unwrap().hard_rule_fired, "HR-10");
        report.network.signals = Some(vec![Signal::new(
            "l1.artifact.selenium",
            serde_json::json!(true),
            Verdict::Bot,
            1.0,
            Source::Server,
            "",
        )]);
        let (result, trace) = engine.score_with_trace(&mut report).unwrap();
        assert_eq!(result.hard_rule_fired, "HR-1");
        assert_eq!(trace.hard_rules.iter().filter(|rule| rule.won).count(), 1);
        assert_eq!(result, engine.score(&mut report).unwrap());
    }

    /// Cap0 honesty: frozen SoT hard rules must evaluate known automation fixtures
    /// as Deny/HR-1 without changing weights, thresholds, or predicates.
    #[test]
    fn known_automation_fixtures_evaluate_honestly_under_frozen_hard_rules() {
        let engine = Engine::default();
        for artifact in [
            "l1.artifact.selenium",
            "l1.artifact.playwright",
            "l1.artifact.phantom",
        ] {
            let mut report = SessionReport::default();
            report.network.signals = Some(vec![Signal::new(
                artifact,
                serde_json::json!(true),
                Verdict::Bot,
                1.0,
                Source::Server,
                "known-automation-fixture",
            )]);
            let result = engine.score(&mut report).unwrap();
            assert_eq!(
                (result.hard_rule_fired.as_str(), result.verdict.as_str()),
                ("HR-1", "DENY"),
                "frozen HR-1 must Deny known automation fixture {artifact}"
            );
        }

        // Non-BOT artifact evidence must not invent HR-1 Deny.
        let mut ok_report = SessionReport::default();
        ok_report.network.signals = Some(vec![Signal::new(
            "l1.artifact.selenium",
            serde_json::json!(false),
            Verdict::Ok,
            1.0,
            Source::Server,
            "non-bot-fixture",
        )]);
        let ok_result = engine.score(&mut ok_report).unwrap();
        assert_ne!(
            ok_result.hard_rule_fired.as_str(),
            "HR-1",
            "OK selenium fixture must not invent HR-1"
        );

        // Webdriver alone is not the hard automation-artifact rule.
        let mut wd_report = SessionReport::default();
        wd_report.client.user_agent = "Mozilla/5.0".into();
        wd_report.client.signals = Some(vec![Signal::new(
            "l1.navigator.webdriver",
            serde_json::json!(true),
            Verdict::Bot,
            1.0,
            Source::Js,
            "webdriver-only-fixture",
        )]);
        let wd_result = engine.score(&mut wd_report).unwrap();
        assert_ne!(
            wd_result.hard_rule_fired.as_str(),
            "HR-1",
            "webdriver-only must not be scored as HR-1 artifact"
        );
    }

    fn bot_signal(id: &str, network: bool, notes: &str) -> Signal {
        Signal::new(
            id,
            serde_json::json!(true),
            Verdict::Bot,
            1.0,
            if network { Source::Server } else { Source::Js },
            notes,
        )
    }

    fn score_fixture(engine: &Engine, signals: &[(&str, bool)]) -> ScoreResult {
        let mut report = SessionReport::default();
        // Browser UA + a non-BOT client tell so HR-10/HR-18 do not preempt Deny fixtures.
        report.client.user_agent = "Mozilla/5.0".into();
        let mut client = vec![Signal::new(
            "l1.navigator.webdriver",
            serde_json::json!(false),
            Verdict::Ok,
            1.0,
            Source::Js,
            "client-presence-anchor",
        )];
        let mut network = Vec::new();
        for &(id, on_network) in signals {
            let signal = bot_signal(id, on_network, "multi-signal-deny-fixture");
            if on_network {
                network.push(signal);
            } else {
                client.push(signal);
            }
        }
        report.client.signals = Some(client);
        if !network.is_empty() {
            report.network.signals = Some(network);
        }
        engine.score(&mut report).unwrap()
    }

    /// Cap0 honesty: multi-signal Deny hard rules beyond HR-1 must fire on
    /// complete frozen fixtures and must not invent Deny on incomplete pairs.
    #[test]
    fn multi_signal_deny_fixtures_evaluate_honestly_under_frozen_hard_rules() {
        let engine = Engine::default();
        let cases: &[(&str, &[(&str, bool)])] = &[
            (
                "HR-3",
                &[
                    ("l4.event.untrusted", false),
                    ("l1.navigator.webdriver", false),
                ],
            ),
            (
                "HR-4",
                &[
                    ("l1.navigator.webdriver", false),
                    ("l3.integrity.native_tostring", false),
                ],
            ),
            (
                "HR-7",
                &[
                    ("l1.ua.headless_token", true),
                    ("l1.navigator.webdriver", false),
                ],
            ),
            (
                "HR-9",
                &[
                    ("l1.cdp.runtime_enable", false),
                    ("l1.navigator.webdriver", false),
                ],
            ),
            ("HR-16", &[("l5.rit.header_tampered", true)]),
            ("HR-14", &[("l5.traffic.engine_rotation", true)]),
            ("HR-21", &[("l5.h2dos.rapid_reset", true)]),
        ];

        for &(rule, signals) in cases {
            let result = score_fixture(&engine, signals);
            assert_eq!(
                (result.hard_rule_fired.as_str(), result.verdict.as_str()),
                (rule, "DENY"),
                "frozen {rule} must Deny complete multi-signal fixture"
            );
        }

        // Incomplete pairs must not invent the paired Deny rule.
        let incomplete: &[(&str, &[(&str, bool)])] = &[
            ("HR-3", &[("l4.event.untrusted", false)]),
            ("HR-4", &[("l3.integrity.native_tostring", false)]),
            ("HR-7", &[("l1.ua.headless_token", true)]),
            ("HR-9", &[("l1.cdp.runtime_enable", false)]),
        ];
        for &(rule, signals) in incomplete {
            let result = score_fixture(&engine, signals);
            assert_ne!(
                result.hard_rule_fired.as_str(),
                rule,
                "incomplete fixture must not invent {rule}"
            );
        }
    }

    fn score_chrome_fixture(
        engine: &Engine,
        signals: &[(&str, bool)],
        ja4_engine: &str,
        h2_engine: &str,
    ) -> ScoreResult {
        let mut report = SessionReport::default();
        report.client.user_agent =
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0.0.0".into();
        report.network.ja4_engine = ja4_engine.into();
        report.network.h2_engine = h2_engine.into();
        let mut client = vec![Signal::new(
            "l1.navigator.webdriver",
            serde_json::json!(false),
            Verdict::Ok,
            1.0,
            Source::Js,
            "client-presence-anchor",
        )];
        let mut network = Vec::new();
        for &(id, on_network) in signals {
            let signal = bot_signal(id, on_network, "remaining-deny-fixture");
            if on_network {
                network.push(signal);
            } else {
                client.push(signal);
            }
        }
        report.client.signals = Some(client);
        if !network.is_empty() {
            report.network.signals = Some(network);
        }
        engine.score(&mut report).unwrap()
    }

    /// Cap0 honesty: remaining frozen Deny hard rules (beyond HR-1 and the
    /// cycle34 multi-signal set) must fire on complete fixtures and must not
    /// invent Deny on incomplete pairs. Detection freeze: no weight/threshold/
    /// predicate change.
    #[test]
    fn remaining_deny_fixtures_evaluate_honestly_under_frozen_hard_rules() {
        let engine = Engine::default();

        // HR-2: UA claims Chrome but TLS + HTTP/2 both resolve elsewhere.
        let hr2 = score_chrome_fixture(&engine, &[], "firefox", "firefox");
        assert_eq!(
            (hr2.hard_rule_fired.as_str(), hr2.verdict.as_str()),
            ("HR-2", "DENY"),
            "frozen HR-2 must Deny dual TLS/H2 engine mismatch"
        );
        let hr2_incomplete = score_chrome_fixture(&engine, &[], "firefox", "chrome");
        assert_ne!(
            hr2_incomplete.hard_rule_fired.as_str(),
            "HR-2",
            "single-axis TLS mismatch must not invent HR-2"
        );

        // HR-5: RIT tamper plus TLS engine mismatch (wins before lone HR-16).
        let hr5 = score_chrome_fixture(
            &engine,
            &[("l5.rit.header_tampered", true)],
            "firefox",
            "chrome",
        );
        assert_eq!(
            (hr5.hard_rule_fired.as_str(), hr5.verdict.as_str()),
            ("HR-5", "DENY"),
            "frozen HR-5 must Deny RIT+TLS mismatch"
        );
        let hr5_incomplete = score_chrome_fixture(
            &engine,
            &[("l5.rit.header_tampered", true)],
            "chrome",
            "chrome",
        );
        assert_ne!(
            hr5_incomplete.hard_rule_fired.as_str(),
            "HR-5",
            "RIT alone must not invent HR-5"
        );

        let cases: &[(&str, &[(&str, bool)])] = &[
            (
                "HR-6",
                &[
                    ("l3.guard.native_hooked", false),
                    ("l1.navigator.webdriver", false),
                ],
            ),
            (
                "HR-8",
                &[
                    ("l3.integrity.native_tostring", false),
                    ("l1.window.outer_eq_inner", false),
                ],
            ),
            (
                "HR-13",
                &[
                    ("l3.guard.console_disabled", false),
                    ("l1.window.outer_eq_inner", false),
                ],
            ),
            (
                "HR-15",
                &[
                    ("l5.traffic.ua_rotation", true),
                    ("l5.traffic.ip_hop", true),
                ],
            ),
            ("HR-19", &[("l7.pow.too_fast", true)]),
            (
                "HR-20",
                &[
                    ("l4.mouse.click_no_trajectory", false),
                    ("l4.agent.burst_silence", false),
                ],
            ),
        ];
        for &(rule, signals) in cases {
            let result = score_fixture(&engine, signals);
            assert_eq!(
                (result.hard_rule_fired.as_str(), result.verdict.as_str()),
                (rule, "DENY"),
                "frozen {rule} must Deny complete remaining fixture"
            );
        }

        // HR-18: browser UA with zero client execution evidence (HTTP parrot).
        let mut parrot = SessionReport::default();
        parrot.client.user_agent =
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0".into();
        let hr18 = engine.score(&mut parrot).unwrap();
        assert_eq!(
            (hr18.hard_rule_fired.as_str(), hr18.verdict.as_str()),
            ("HR-18", "DENY"),
            "frozen HR-18 must Deny browser UA with no JS evidence"
        );

        let incomplete: &[(&str, &[(&str, bool)])] = &[
            ("HR-6", &[("l3.guard.native_hooked", false)]),
            ("HR-8", &[("l3.integrity.native_tostring", false)]),
            ("HR-13", &[("l3.guard.console_disabled", false)]),
            ("HR-15", &[("l5.traffic.ua_rotation", true)]),
            ("HR-20", &[("l4.mouse.click_no_trajectory", false)]),
        ];
        for &(rule, signals) in incomplete {
            let result = score_fixture(&engine, signals);
            assert_ne!(
                result.hard_rule_fired.as_str(),
                rule,
                "incomplete fixture must not invent {rule}"
            );
        }
    }

    /// Cap0 honesty: frozen Challenge hard rules (HR-10/11/12/17/22/23/24)
    /// must Challenge on complete fixtures and must not invent Challenge on
    /// incomplete multi-signal sets. Detection freeze: no weight/threshold/
    /// predicate change. Does not redo HR-1 or Deny Cap0 units.
    #[test]
    fn challenge_fixtures_evaluate_honestly_under_frozen_hard_rules() {
        let engine = Engine::default();

        // HR-10: missing client-side evidence challenges.
        let hr10 = engine.score(&mut SessionReport::default()).unwrap();
        assert_eq!(
            (hr10.hard_rule_fired.as_str(), hr10.verdict.as_str()),
            ("HR-10", "CHALLENGE"),
            "frozen HR-10 must Challenge empty client fixture"
        );

        let cases: &[(&str, &[(&str, bool)])] = &[
            ("HR-17", &[("l5.rit.stale_replay", true)]),
            ("HR-12", &[("l4.event.no_interaction", false)]),
            ("HR-22", &[("l2.adv.audio_24k", false)]),
            ("HR-24", &[("l5.header.proxy_hop", true)]),
        ];
        for &(rule, signals) in cases {
            let result = score_fixture(&engine, signals);
            assert_eq!(
                (result.hard_rule_fired.as_str(), result.verdict.as_str()),
                (rule, "CHALLENGE"),
                "frozen {rule} must Challenge complete fixture"
            );
        }

        // HR-11: consistent browser from a datacenter ASN.
        let hr11 = score_chrome_fixture(
            &engine,
            &[("l5.ip.datacenter_asn", true)],
            "chrome",
            "chrome",
        );
        assert_eq!(
            (hr11.hard_rule_fired.as_str(), hr11.verdict.as_str()),
            ("HR-11", "CHALLENGE"),
            "frozen HR-11 must Challenge datacenter ASN under consistent browser"
        );
        let hr11_incomplete = score_chrome_fixture(&engine, &[], "chrome", "chrome");
        assert_ne!(
            hr11_incomplete.hard_rule_fired.as_str(),
            "HR-11",
            "consistent browser without datacenter ASN must not invent HR-11"
        );
        let hr11_mismatch = score_chrome_fixture(
            &engine,
            &[("l5.ip.datacenter_asn", true)],
            "firefox",
            "chrome",
        );
        assert_ne!(
            hr11_mismatch.hard_rule_fired.as_str(),
            "HR-11",
            "TLS mismatch must not invent HR-11"
        );

        // HR-23: Chrome UA with all three Chromium-container residuals.
        let hr23 = score_chrome_fixture(
            &engine,
            &[
                ("l2.adv.no_widevine", false),
                ("l2.adv.no_media_devices", false),
                ("l2.adv.no_voices", false),
            ],
            "chrome",
            "chrome",
        );
        assert_eq!(
            (hr23.hard_rule_fired.as_str(), hr23.verdict.as_str()),
            ("HR-23", "CHALLENGE"),
            "frozen HR-23 must Challenge complete container residual set"
        );
        let hr23_incomplete = score_chrome_fixture(
            &engine,
            &[("l2.adv.no_widevine", false)],
            "chrome",
            "chrome",
        );
        assert_ne!(
            hr23_incomplete.hard_rule_fired.as_str(),
            "HR-23",
            "partial container residual must not invent HR-23"
        );
    }

    /// Cap0 honesty: frozen numeric scoring bands (ALLOW / CHALLENGE / DENY)
    /// must map risk without inventing a hard-rule override. Soft multi-layer
    /// fixtures only — does not redo Deny/Challenge HR Cap0 units. Detection
    /// freeze: no weight/threshold/predicate change.
    #[test]
    fn scoring_band_fixtures_evaluate_honestly_under_frozen_thresholds() {
        let engine = Engine::default();

        // ALLOW band: client present, soft OK evidence, risk below ChallengeAt.
        let allow = score_fixture(&engine, &[]);
        assert_eq!(
            (allow.verdict.as_str(), allow.hard_rule_fired.as_str()),
            ("ALLOW", ""),
            "frozen ALLOW band must not invent a hard rule on soft client fixture"
        );
        assert!(
            allow.risk_score < engine.policy.challenge_at,
            "ALLOW fixture risk must stay below ChallengeAt"
        );

        // CHALLENGE band: single soft L5 residual (~30) crosses ChallengeAt without HR.
        let challenge = score_fixture(&engine, &[("l5.traffic.tls_no_permutation", true)]);
        assert_eq!(
            (challenge.verdict.as_str(), challenge.hard_rule_fired.as_str()),
            ("CHALLENGE", ""),
            "frozen CHALLENGE band must Challenge soft residual without hard rule"
        );
        assert!(
            challenge.risk_score >= engine.policy.challenge_at
                && challenge.risk_score < engine.policy.deny_at,
            "CHALLENGE fixture risk must sit in [ChallengeAt, DenyAt)"
        );

        // DENY band: multi-layer soft residuals combine above DenyAt without HR.
        let deny = score_fixture(
            &engine,
            &[
                ("l5.abuse.flood", true),
                ("l2.webgl.swiftshader", false),
                ("l7.pass.flood", true),
            ],
        );
        assert_eq!(
            (deny.verdict.as_str(), deny.hard_rule_fired.as_str()),
            ("DENY", ""),
            "frozen DENY band must Deny soft multi-layer residual without hard rule"
        );
        assert!(
            deny.risk_score >= engine.policy.deny_at,
            "DENY fixture risk must reach DenyAt"
        );

        // Sub-ChallengeAt soft residual must not invent CHALLENGE/DENY bands.
        let mut soft = SessionReport::default();
        soft.client.user_agent = "Mozilla/5.0".into();
        soft.client.signals = Some(vec![Signal::new(
            "l1.navigator.webdriver",
            serde_json::json!(true),
            Verdict::Suspicious,
            1.0,
            Source::Js,
            "below-challenge-soft-fixture",
        )]);
        let below_band = engine.score(&mut soft).unwrap();
        assert_eq!(
            (
                below_band.verdict.as_str(),
                below_band.hard_rule_fired.as_str()
            ),
            ("ALLOW", ""),
            "sub-ChallengeAt soft residual must stay ALLOW without hard rule"
        );
        assert!(
            below_band.risk_score < engine.policy.challenge_at,
            "sub-ChallengeAt soft residual must not invent Challenge band"
        );
    }
    #[test]
    fn invalid_numeric_configuration_and_contributions_fail_closed() {
        let engine = Engine {
            policy: Policy {
                layer_cap: f64::NAN,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            engine.score(&mut SessionReport::default()),
            Err(ScoringError::NonFinitePolicy)
        );
        let mut report = SessionReport::default();
        let signal = Signal {
            score: f64::INFINITY,
            ..Default::default()
        };
        report.client.signals = Some(vec![signal]);
        assert_eq!(
            Engine::default().score(&mut report),
            Err(ScoringError::NonFiniteContribution)
        );
    }
}

//! One orchestration path for observation derivation, combination and decisions.
use crate::{
    combination::{Contribution, combine, top_contributors},
    consistency::{cross_checks, user_agent::is_browser_ua},
    derivation::{advanced_signals, behavior_signals, environment_signals, guard_signals},
    policy::{Policy, SessionVerdict},
    promotion::{self, RuleContext},
    trace::{ContributionTrace, PolicyView, ScoreTrace},
};
use humanymous_model::inference::Runtime;
use humanymous_signals::{
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
    use humanymous_signals::observation::{Signal, Source, Verdict};
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

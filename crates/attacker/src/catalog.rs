//! Versioned observation scenarios; frozen fixtures remain owned by scoring.
use humanymous_signals::report::SessionReport;
use serde_json::{Value, json};

/// A scheduled attempt, including unavailable runtime responsibilities.
pub struct Case {
    /// Stable catalog identity.
    pub id: String,
    /// Cost ladder or control tier.
    pub tier: &'static str,
    /// Attack, human, accessibility, or ceiling control.
    pub class: &'static str,
    /// Reproducible observation provenance; never claims browser execution.
    pub provenance: &'static str,
    /// Report and expected regression fields, or an unavailable-runtime reason.
    pub input: Result<(SessionReport, Value), String>,
    /// Distinguish missing runtime support from fixture parse errors.
    pub unsupported: bool,
}

const HUMAN: &str = include_str!("../../scoring/tests/fixtures/frozen/human-chrome-allow.json");

fn replay(id: &str, tier: &'static str, class: &'static str, raw: &str) -> Case {
    let input = (|| {
        let fixture: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        let report =
            serde_json::from_value(fixture["report"].clone()).map_err(|e| e.to_string())?;
        Ok((report, fixture["want"].clone()))
    })();
    Case {
        id: id.into(),
        tier,
        class,
        provenance: "scoring frozen Go-oracle fixture replay",
        input,
        unsupported: false,
    }
}

/// Catalog v1. Seed controls order only; input coverage is not probabilistic.
pub fn cases(seed: u64) -> Vec<Case> {
    let mut cases = vec![
        replay("human-zero-motor", "control", "human", HUMAN),
        replay(
            "go-http-client",
            "T0",
            "attack",
            include_str!("../../scoring/tests/fixtures/frozen/go-http-client.json"),
        ),
        replay(
            "missing-client",
            "T0",
            "attack",
            include_str!("../../scoring/tests/fixtures/frozen/no-client-hr10.json"),
        ),
        replay(
            "ua-tls-h2-mismatch",
            "T0",
            "attack",
            include_str!("../../scoring/tests/fixtures/frozen/ua-tls-h2-hr2.json"),
        ),
        replay(
            "selenium-artifact",
            "T1",
            "attack",
            include_str!("../../scoring/tests/fixtures/frozen/selenium-hr1.json"),
        ),
        replay(
            "webdriver-cdp",
            "T1",
            "attack",
            include_str!("../../scoring/tests/fixtures/frozen/bot-webdriver-cdp-hr9.json"),
        ),
        replay("coherent-observation-ceiling", "T4", "ceiling", HUMAN),
    ];
    let mut accessibility = replay(
        "accessibility-no-motor-long-window",
        "control",
        "accessibility",
        HUMAN,
    );
    accessibility.provenance = "synthetic zero-motor 60-second observation; not a human pilot";
    if let Ok((report, expected)) = &mut accessibility.input {
        report.client.behavior = Default::default();
        report.client.behavior.duration_s = 60.0;
        *expected = json!({"verdict":"CHALLENGE", "hardRuleFired":"HR-12"});
    }
    cases.push(accessibility);
    for (id, tier, class, reason) in [
        (
            "forged-pass-runtime",
            "T2",
            "attack",
            "Requires real collect trust-boundary and challenge credential verification",
        ),
        (
            "real-browser-agent",
            "T3",
            "attack",
            "Requires actual browser and Rust Gate enforcement runtime",
        ),
        (
            "installation-binding-egress",
            "runtime",
            "runtime",
            "Signed installation identity and internal-topology escape tests not implemented",
        ),
        (
            "accessible-pass-human",
            "control",
            "accessibility",
            "Requires actual accessible challenge and measured human completion; no motor requirement permitted",
        ),
    ] {
        cases.push(Case {
            id: id.into(),
            tier,
            class,
            provenance: "explicit missing runtime coverage",
            input: Err(reason.into()),
            unsupported: true,
        });
    }
    shuffle(&mut cases, seed);
    cases
}

/// One seed-order implementation shared by simulation and owned network cases.
pub(crate) fn shuffle<T>(cases: &mut [T], seed: u64) {
    // A stable integer mixer orders cases without platform RNG or clock state.
    let mut state = seed;
    for i in (1..cases.len()).rev() {
        state = state.wrapping_add(0x9e3779b97f4a7c15);
        let mut mixed = state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d049bb133111eb);
        mixed ^= mixed >> 31;
        cases.swap(i, (mixed % (i as u64 + 1)) as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_has_unique_ids_and_seed_keeps_every_denominator() {
        use std::collections::BTreeSet;
        let ids = |seed| {
            cases(seed)
                .into_iter()
                .map(|c| c.id)
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(ids(0).len(), 12);
        assert_eq!(ids(0), ids(u64::MAX));
    }
}

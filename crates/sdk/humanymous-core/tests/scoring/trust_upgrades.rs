//! Trust upgrades require server evidence and cannot override promotion rules.
use humanymous_core::scoring::engine::Engine;
use humanymous_core::signals::{
    observation::{Signal, Source, Verdict},
    report::SessionReport,
};

fn signal(id: &str, verdict: Verdict, source: Source) -> Signal {
    Signal::new(id, serde_json::json!(true), verdict, 1.0, source, "")
}

// Same moderate-suspicion report as the original pass_upgrade_test.go.
fn borderline() -> SessionReport {
    let mut report: SessionReport = serde_json::from_value(serde_json::json!({
        "client": {
            "userAgent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36",
            "uaClientHints": {"platform": "Windows", "brands": [{"brand":"Chromium","version":"126"}]},
            "environment": {"probed":true},
            "behavior": {"events":{"totalEvents":50,"untrustedFrac":0}}
        },
        "network": {"ja4Engine":"chrome","h2Engine":"chrome","secFetchPresent":true,"secChUaPresent":true}
    })).unwrap();
    report.client.signals = Some(vec![
        signal("l2.webgl.swiftshader", Verdict::Bot, Source::Wasm),
        signal("l2.webgl.param_consistency", Verdict::Bot, Source::Wasm),
    ]);
    report.network.signals = Some(vec![signal(
        "l5.header.h2_uppercase",
        Verdict::Bot,
        Source::Server,
    )]);
    report
}

#[test]
fn only_server_collected_ok_network_evidence_upgrades_score_challenges() {
    let engine = Engine::default();
    let baseline = engine.score(&mut borderline()).unwrap();
    assert_eq!(baseline.verdict, "CHALLENGE");
    assert!(baseline.hard_rule_fired.is_empty());
    for (id, upgrade) in [
        ("l7.pow.solved", "PoW-upgrade"),
        ("l7.pass.solved", "Pass-upgrade"),
    ] {
        for client_plane in [false, true] {
            for source in [Source::Server, Source::Js, Source::Wasm] {
                for verdict in [Verdict::Ok, Verdict::Suspicious, Verdict::Bot] {
                    let trusted =
                        !client_plane && source == Source::Server && verdict == Verdict::Ok;
                    let mut report = borderline();
                    let observation = signal(id, verdict, source.clone());
                    if client_plane {
                        report.client.signals.as_mut().unwrap().push(observation);
                    } else {
                        report.network.signals.as_mut().unwrap().push(observation);
                    }
                    let result = engine.score(&mut report).unwrap();
                    assert_eq!(result.risk_score, baseline.risk_score);
                    assert_eq!(result.verdict, if trusted { "ALLOW" } else { "CHALLENGE" });
                    assert_eq!(result.hard_rule_fired, if trusted { upgrade } else { "" });
                }
            }
        }
        for (hard_id, expected_rule, expected_verdict) in [
            ("l1.artifact.selenium", "HR-1", "DENY"),
            ("l5.rit.absent", "HR-17", "CHALLENGE"),
        ] {
            let mut report = borderline();
            report.network.signals.as_mut().unwrap().extend([
                signal(id, Verdict::Ok, Source::Server),
                signal(hard_id, Verdict::Bot, Source::Server),
            ]);
            let result = engine.score(&mut report).unwrap();
            assert_eq!(result.verdict, expected_verdict);
            assert_eq!(result.hard_rule_fired, expected_rule);
        }
    }
}

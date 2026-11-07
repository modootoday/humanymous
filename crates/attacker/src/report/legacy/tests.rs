use super::*;

fn rows(input: &str) -> Vec<Record> {
    decode(input.as_bytes()).ok().unwrap()
}

#[test]
fn frozen_go_four_metric_rows_and_fifth_skip_preserve_tpr_fpr() {
    let summary = aggregate(&rows(
        r#"[
        {"label":"human","verdict":"ALLOW","outcome":"TN"},
        {"label":"bot:selenium","verdict":"DENY","hardRuleFired":"HR-1","outcome":"TP"},
        {"label":"bot:puppeteer","verdict":"DENY","hardRuleFired":"HR-7","outcome":"TP"},
        {"label":"bot:nodriver","verdict":"CHALLENGE","hardRuleFired":"HR-12","outcome":"TP"},
        {"profile":"camoufox.mjs","skipped":true,"reason":"not installed"}
    ]"#,
    ))
    .unwrap();
    assert_eq!((summary.bot_tpr, summary.human_fpr), (1.0, 0.0));
    assert_eq!((summary.bot_runs, summary.human_runs), (3, 1));
    assert_eq!(
        (
            summary.accounting.scheduled,
            summary.accounting.eligible,
            summary.accounting.skipped
        ),
        (5, 4, 1)
    );
    assert!(summary.accounting.unsupported_count.is_none());
    assert_eq!(summary.unsupported_accounting, "not_encoded_in_legacy_v1");
}

#[test]
fn frozen_go_missed_bot_lowers_tpr_without_trusting_outcome() {
    let summary = aggregate(&rows(
        r#"[
        {"label":"bot:a","verdict":"DENY","outcome":"FN"},
        {"label":"bot:b","verdict":"ALLOW","outcome":"TP"}
    ]"#,
    ))
    .unwrap();
    assert_eq!(summary.bot_tpr, 0.5);
}

#[test]
fn first_eligible_order_mean_rules_and_repeated_top_hits_are_preserved() {
    let summary = aggregate(&rows(r#"[
        {"label":"bot:a","error":"omit from legacy rates"},
        {"label":"bot:z","verdict":"DENY","riskScore":80,"hardRuleFired":"HR-2","top":["z","b","a","z"]},
        {"label":"human","verdict":"ALLOW","riskScore":2},
        {"label":"bot:z","verdict":"ALLOW","riskScore":20,"hardRuleFired":"HR-1","top":["b"]},
        {"label":"bot:a","verdict":"CHALLENGE","riskScore":50}
    ]"#)).unwrap();
    assert_eq!(
        summary
            .profiles
            .iter()
            .map(|p| p.label.as_str())
            .collect::<Vec<_>>(),
        ["bot:z", "human", "bot:a"]
    );
    let p = &summary.profiles[0];
    assert_eq!(p.mean_risk, 50.0);
    assert_eq!(p.verdicts["ALLOW"], 1);
    assert_eq!(
        p.rules.keys().map(String::as_str).collect::<Vec<_>>(),
        ["HR-1", "HR-2"]
    );
    assert_eq!(p.sorted_top_hits(), [("b", 2), ("z", 2), ("a", 1)]);
}

#[test]
fn errors_skips_missing_labels_and_diagnostic_privacy_remain_explicit() {
    let summary = aggregate(&rows(
        r#"[
        {"label":"bot:a","verdict":"DENY","riskScore":100},
        {"label":"bot:a","verdict":"ALLOW","riskScore":20},
        {"label":"bot:a","error":"SECRET_CREDENTIAL"},
        {"profile":"skip","skipped":true,"error":"SECRET_CREDENTIAL","reason":"SECRET_CREDENTIAL"},
        {}
    ]"#,
    ))
    .unwrap();
    let a = &summary.accounting;
    assert_eq!(
        (
            a.scheduled,
            a.eligible,
            a.errors,
            a.skipped,
            a.missing_label,
            a.error_marked
        ),
        (5, 2, 1, 1, 1, 2)
    );
    assert_eq!(
        a.scheduled,
        a.eligible + a.errors + a.skipped + a.missing_label
    );
    assert_eq!(summary.bot_tpr, 0.5);
    let html = render(&summary, "fixed").unwrap();
    assert!(!html.contains("SECRET_CREDENTIAL"));
    assert!(
        !serde_json::to_string(&summary)
            .unwrap()
            .contains("SECRET_CREDENTIAL")
    );
    assert!(html.contains("not encoded (unknown, not zero)"));
    assert!(html.contains("N/A (no observations)"));
    assert!(html.contains("not a product pass"));
    assert!(!html.contains("class=good"));
}

#[test]
fn denied_only_human_fpr_does_not_hide_challenge_or_no_response() {
    let summary = aggregate(&rows(
        r#"[
        {"label":"human","verdict":"DENY"},
        {"label":"human","verdict":"CHALLENGE"},
        {"label":"human","verdict":"NO_RESPONSE"}
    ]"#,
    ))
    .unwrap();
    assert_eq!(summary.human_fpr, 1.0 / 3.0);
    assert_eq!((summary.human_challenges, summary.human_non_allow), (1, 3));
    assert!(
        render(&summary, "")
            .unwrap()
            .contains("CHALLENGE is not successful human access")
    );
}

#[test]
fn null_defaults_and_unknown_discard_match_go_without_duplicate_key_ambiguity() {
    let records = rows(
        r#"[null,{"label":"human","run":null,"riskScore":null,"top":[null],"skipped":null,"error":null,"extra":{"credential":"SECRET"}}]"#,
    );
    assert_eq!(records[1].run, 0);
    assert_eq!(records[1].risk_score, 0.0);
    assert_eq!(records[1].top, [""]);
    let summary = aggregate(&records).unwrap();
    assert_eq!(summary.accounting.missing_label, 1);
    assert!(!render(&summary, "").unwrap().contains("SECRET"));
    assert!(decode(b"null").ok().unwrap().is_empty());
    for invalid in [
        r#"[{"label":"human","\u006cabel":"bot:a"}]"#,
        r#"[{"ignored":{"a":1,"a":2}}]"#,
        r#"[{"label":1}]"#,
        r#"[{"run":1.0}]"#,
        r#"[{"skipped":"false"}]"#,
        r#"[{"top":[3]}]"#,
        r#"[{"riskScore":1e9999}]"#,
        "[] trailing",
    ] {
        assert!(decode(invalid.as_bytes()).is_err());
    }
}

#[test]
fn html_is_deterministic_and_escapes_every_dynamic_text_context() {
    let summary = aggregate(&rows(r#"[
        {"label":"bot:<script>&\"'","verdict":"<verdict>","hardRuleFired":"<img>","top":["<svg>","&"]},
        {"profile":"<skipped>","label":"<label>","skipped":true,"reason":"PRIVATE_REASON"}
    ]"#)).unwrap();
    let note = "</p><script>CAPTION</script>\"'&";
    let first = render(&summary, note).unwrap();
    assert_eq!(first, render(&summary, note).unwrap());
    for raw in [
        "<script>",
        "<verdict>",
        "<img>",
        "<svg>",
        "<skipped>",
        "<label>",
        "PRIVATE_REASON",
    ] {
        assert!(!first.contains(raw), "unescaped context");
    }
    for escaped in [
        "&lt;script&gt;",
        "&lt;verdict&gt;",
        "&lt;img&gt;",
        "&lt;svg&gt;",
        "&quot;",
        "&#39;",
        "&amp;",
    ] {
        assert!(first.contains(escaped));
    }
    assert!(first.contains("default-src 'none'"));
}

#[test]
fn input_field_vector_row_caption_output_and_numeric_limits_fail_closed() {
    assert!(matches!(
        decode(&vec![b' '; INPUT_BYTES + 1]),
        Err(Error::InputTooLarge)
    ));
    let oversized = serde_json::json!([{"ignored":"x".repeat(FIELD_BYTES + 1)}]);
    assert!(matches!(
        decode(&serde_json::to_vec(&oversized).unwrap()),
        Err(Error::Limit)
    ));
    let oversized = serde_json::json!([{"top":vec!["x"; MAX_TOP + 1]}]);
    assert!(matches!(
        decode(&serde_json::to_vec(&oversized).unwrap()),
        Err(Error::Limit)
    ));
    let oversized = format!("[{}]", vec!["null"; MAX_ROWS + 1].join(","));
    assert!(matches!(decode(oversized.as_bytes()), Err(Error::Limit)));
    let records = vec![
        Record {
            label: "bot:a".into(),
            risk_score: f64::MAX,
            ..Default::default()
        };
        2
    ];
    assert!(matches!(aggregate(&records), Err(Error::RiskOverflow)));
    let summary = aggregate(&[]).unwrap();
    assert!(matches!(
        render(&summary, &"x".repeat(FIELD_BYTES + 1)),
        Err(Error::Limit)
    ));
    let note = "&".repeat(100);
    let full = render(&summary, &note).unwrap();
    assert!(matches!(
        super::html::render_with_limit(&summary, &note, full.len() - 1),
        Err(Error::OutputTooLarge)
    ));
}

#[test]
#[ignore = "requires an explicitly allocated native filesystem publication pilot"]
fn native_publication_replaces_old_report_and_preserves_foreign_collision() {
    let directory =
        std::env::temp_dir().join(format!("humanymous-report-pilot-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let output = directory.join("report.html");
    fs::write(&output, "old report").unwrap();
    TEMP_SEQUENCE.store(0, Ordering::Relaxed);
    let foreign = directory.join(format!(".humanymous-report-{}-0.tmp", std::process::id()));
    fs::write(&foreign, "foreign sentinel").unwrap();
    write_atomic(&output, "new report").unwrap();
    assert_eq!(fs::read_to_string(&output).unwrap(), "new report");
    assert_eq!(fs::read_to_string(&foreign).unwrap(), "foreign sentinel");
    TEMP_SEQUENCE.store(1000, Ordering::Relaxed);
    let collisions: Vec<_> = (1000..1032)
        .map(|serial| {
            directory.join(format!(
                ".humanymous-report-{}-{serial}.tmp",
                std::process::id()
            ))
        })
        .collect();
    for collision in &collisions {
        fs::write(collision, "foreign collision").unwrap();
    }
    assert_eq!(
        write_atomic(&output, "must not replace"),
        Err(Error::Collision)
    );
    assert_eq!(fs::read_to_string(&output).unwrap(), "new report");
    for collision in &collisions {
        assert_eq!(fs::read_to_string(collision).unwrap(), "foreign collision");
        fs::remove_file(collision).unwrap();
    }
    fs::remove_file(&output).unwrap();
    fs::remove_file(&foreign).unwrap();
    fs::remove_dir(&directory).unwrap();
}

#[cfg(windows)]
#[test]
#[ignore = "requires an explicitly allocated Windows file-sharing failure pilot"]
fn native_windows_denied_replacement_retains_old_report() {
    use std::os::windows::fs::OpenOptionsExt;
    let directory = std::env::temp_dir().join(format!(
        "humanymous-report-lock-pilot-{}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    let output = directory.join("report.html");
    fs::write(&output, "retained old report").unwrap();
    let locked = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&output)
        .unwrap();
    assert!(write_atomic(&output, "replacement").is_err());
    drop(locked);
    assert_eq!(fs::read_to_string(&output).unwrap(), "retained old report");
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    fs::remove_file(&output).unwrap();
    fs::remove_dir(&directory).unwrap();
}

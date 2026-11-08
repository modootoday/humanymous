//! Bounded language indicators for review, not automatic language identification.
#[path = "language_source.rs"]
pub mod source;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preservation {
    pub path: String,
    pub line_sha256: String,
    pub category: String,
    pub reason: String,
}
#[derive(Debug, Serialize)]
pub struct Finding {
    pub line: usize,
    pub line_sha256: String,
    pub classification: &'static str,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub findings: Vec<Finding>,
    pub errors: Vec<String>,
    pub english_conformance_proven: bool,
}

fn indicator(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c as u32,
        0x1100..=0x11ff | 0x3130..=0x318f | 0xac00..=0xd7af |
        0x3040..=0x30ff | 0x4e00..=0x9fff | 0x0400..=0x04ff)
    })
}

/// Scan Markdown prose or Rust comment-only lines. Inline Rust comments require
/// separate review; this deliberately avoids treating string fixtures as prose.
pub fn inspect(path: &str, text: &str, preserved: &[Preservation]) -> Report {
    let mut report = Report {
        schema: "humanymous.authoring-language.v1",
        findings: Vec::new(),
        errors: Vec::new(),
        english_conformance_proven: false,
    };
    if text.len() > 2 * 1024 * 1024 || preserved.len() > 4096 {
        report.errors.push("language input exceeds bound".into());
        return report;
    }
    let markdown = path.ends_with(".md");
    let rust = path.ends_with(".rs");
    if !markdown && !rust {
        report.errors.push("unsupported prose source kind".into());
        return report;
    }
    for entry in preserved {
        if entry.path.is_empty()
            || entry.path.contains('*')
            || entry.line_sha256.len() != 64
            || !entry.line_sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || ![
                "raw-evidence",
                "third-party-quote",
                "literal-ui-title",
                "multilingual-fixture",
            ]
            .contains(&entry.category.as_str())
            || entry.reason.trim().is_empty()
            || entry.reason.len() > 1024
        {
            report
                .errors
                .push("invalid exact-line preservation record".into());
        }
    }
    let mut fence: Option<char> = None;
    let mut block = false;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if markdown {
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                let marker = trimmed.chars().next().unwrap();
                if fence == Some(marker) {
                    fence = None;
                } else if fence.is_none() {
                    fence = Some(marker);
                }
                continue;
            }
            if fence.is_some() {
                continue;
            }
        }
        if rust {
            let comment = block || trimmed.starts_with("//") || trimmed.starts_with("/*");
            if trimmed.starts_with("/*") {
                block = true;
            }
            if trimmed.contains("*/") {
                block = false;
            }
            if !comment {
                continue;
            }
        }
        if !indicator(line) {
            continue;
        }
        let hash = format!("{:x}", Sha256::digest(line.as_bytes()));
        let retained = preserved.iter().any(|entry| {
            entry.path == path
                && entry.line_sha256 == hash
                && [
                    "raw-evidence",
                    "third-party-quote",
                    "literal-ui-title",
                    "multilingual-fixture",
                ]
                .contains(&entry.category.as_str())
                && !entry.reason.trim().is_empty()
        });
        report.findings.push(Finding {
            line: index + 1,
            line_sha256: hash,
            classification: if retained {
                "preserved-exact-line"
            } else {
                "authored-prose-review-debt"
            },
        });
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_is_not_an_ascii_only_policy() {
        let report = inspect(
            "README.md",
            "Use café names, naïve examples and symbols → ✓.\n",
            &[],
        );
        assert!(report.findings.is_empty());
        assert!(!report.english_conformance_proven);
    }
    #[test]
    fn authored_comments_are_debt_but_literal_fixture_strings_are_not_prose() {
        let report = inspect("test.rs", "// 한국어 설명\nlet fixture = \"서울\";\n", &[]);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(
            report.findings[0].classification,
            "authored-prose-review-debt"
        );
    }
    #[test]
    fn preservation_is_exact_and_does_not_exempt_whole_scopes() {
        let title = "Humanymous · 개발도구 · 규칙";
        let entry = Preservation {
            path: "THREADS.md".into(),
            line_sha256: format!("{:x}", Sha256::digest(title.as_bytes())),
            category: "literal-ui-title".into(),
            reason: "Recorded application title".into(),
        };
        let report = inspect("THREADS.md", &format!("{title}\n추가 설명\n"), &[entry]);
        assert_eq!(report.findings[0].classification, "preserved-exact-line");
        assert_eq!(
            report.findings[1].classification,
            "authored-prose-review-debt"
        );
    }
    #[test]
    fn repository_title_records_preserve_exact_ui_lines() {
        // Invented title fixture; no mutable registry bytes compiled into tests.
        let path = ".agents/sessions/THREADS.md";
        let title = "Synthetic title · 서울";
        let records = vec![Preservation {
            path: path.into(),
            line_sha256: format!("{:x}", Sha256::digest(title.as_bytes())),
            category: "literal-ui-title".into(),
            reason: "Synthetic parser fixture, not a recorded application title".into(),
        }];
        let report = inspect(path, title, &records);
        assert!(report.errors.is_empty());
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].classification, "preserved-exact-line");
        let changed = inspect(path, &format!("{title}\n추가 설명\n"), &records);
        assert_eq!(changed.findings.len(), 2);
        assert_eq!(changed.findings[1].classification, "authored-prose-review-debt");
    }
}

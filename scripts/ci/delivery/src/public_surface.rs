//! `Public surface` check (common automation rules C4).
//!
//! Scans the text a change makes public: added lines, file names, commit
//! messages and the pull request title and body. Deny terms come from the
//! `HUMANYMOUS_DENY_TERMS` secret, one term per line, plus the optional
//! `HUMANYMOUS_DENY_TERMS_EXTRA` secret in the same format; the check uses the
//! union of both lists. It fails closed when the main secret is absent or
//! empty (an unset or empty extra secret adds nothing), and never prints a
//! term or a matching line: findings name the location and the term's index
//! only.
use crate::{Result, command};
use std::path::Path;

const DENY_ENV: &str = "HUMANYMOUS_DENY_TERMS";
const DENY_EXTRA_ENV: &str = "HUMANYMOUS_DENY_TERMS_EXTRA";

/// Parses the secret value: one term per line, blank lines ignored.
fn terms(value: &str) -> Vec<String> {
    value
        .lines()
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Builds the deny list from the main and the optional extra secret values:
/// the main terms keep their indexes, extra terms follow, and duplicates are
/// dropped. An empty main list fails closed whatever the extra list holds.
fn deny_terms(main: &str, extra: &str) -> Result<Vec<String>> {
    let mut list = terms(main);
    if list.is_empty() {
        return Err(format!(
            "{DENY_ENV} is absent or empty; Public surface fails closed (a fork pull request is reviewed by the owner)"
        ));
    }
    for term in terms(extra) {
        if !list.contains(&term) {
            list.push(term);
        }
    }
    Ok(list)
}

/// Returns the 1-based indexes of the terms found in `text`, case-insensitively.
fn hits(terms: &[String], text: &str) -> Vec<usize> {
    let text = text.to_lowercase();
    terms
        .iter()
        .enumerate()
        .filter(|(_, term)| text.contains(term.as_str()))
        .map(|(index, _)| index + 1)
        .collect()
}

/// Returns `(path, new line number, added text)` for every added line of a
/// zero-context unified diff.
fn added_lines(diff: &str) -> Vec<(String, usize, String)> {
    let mut result = Vec::new();
    let mut path = String::new();
    let mut line = 0usize;
    for raw in diff.lines() {
        if let Some(rest) = raw.strip_prefix("+++ ") {
            path = rest.strip_prefix("b/").unwrap_or(rest).to_owned();
        } else if raw.starts_with("--- ") {
        } else if let Some(hunk) = raw.strip_prefix("@@ ") {
            line = hunk
                .split_whitespace()
                .find_map(|part| part.strip_prefix('+'))
                .and_then(|range| range.split(',').next())
                .and_then(|start| start.parse().ok())
                .unwrap_or(0);
        } else if let Some(text) = raw.strip_prefix('+') {
            result.push((path.clone(), line, text.to_owned()));
            line += 1;
        } else if raw.starts_with(' ') {
            line += 1;
        }
    }
    result
}

fn null_base(base: &str) -> bool {
    !base.is_empty() && base.bytes().all(|b| b == b'0')
}

fn revision(value: &str) -> Result<&str> {
    if value.is_empty()
        || value.starts_with('-')
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'_' | b'~' | b'^'))
    {
        return Err("public-surface needs plain base and head revisions".into());
    }
    Ok(value)
}

/// `delivery public-surface <base> <head>`; on a push whose base is the null
/// object id, only the head commit is scanned.
pub fn check(root: &Path, base: &str, head: &str) -> Result<()> {
    let head = revision(head)?;
    let base = if null_base(base) {
        format!("{head}~1")
    } else {
        revision(base)?.to_owned()
    };
    let terms = deny_terms(
        &std::env::var(DENY_ENV).unwrap_or_default(),
        &std::env::var(DENY_EXTRA_ENV).unwrap_or_default(),
    )?;
    let range = format!("{base}..{head}");
    let triple = format!("{base}...{head}");
    let mut findings = Vec::new();
    let diff = command::text(
        root,
        "git",
        &[
            "diff",
            "--no-color",
            "--no-ext-diff",
            "--unified=0",
            "--no-renames",
            &triple,
        ],
    )?;
    for (path, line, text) in added_lines(&diff) {
        for index in hits(&terms, &text) {
            findings.push(format!("added line {path}:{line}: term #{index}"));
        }
    }
    let names = command::text(
        root,
        "git",
        &["diff", "--name-only", "--no-renames", &triple],
    )?;
    for name in names.lines() {
        for index in hits(&terms, name) {
            findings.push(format!("file name {name}: term #{index}"));
        }
    }
    let shas = command::text(root, "git", &["rev-list", &range])?;
    for sha in shas.lines().filter(|sha| !sha.is_empty()) {
        let message = command::text(root, "git", &["show", "-s", "--format=%B", sha])?;
        for index in hits(&terms, &message) {
            findings.push(format!("commit message {sha}: term #{index}"));
        }
    }
    for (label, key) in [
        ("pull request title", "PR_TITLE"),
        ("pull request body", "PR_BODY"),
    ] {
        for index in hits(&terms, &std::env::var(key).unwrap_or_default()) {
            findings.push(format!("{label}: term #{index}"));
        }
    }
    let attribution = crate::trailers::check(root, &range);
    if !findings.is_empty() {
        return Err(format!(
            "Public surface found {} deny-term match(es) in {range}:\n{}",
            findings.len(),
            findings.join("\n")
        ));
    }
    attribution?;
    println!(
        "Public surface passed for {range} ({} terms, {} commits)",
        terms.len(),
        shas.lines().filter(|sha| !sha.is_empty()).count()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terms_are_trimmed_lowercased_and_blank_lines_dropped() {
        assert_eq!(terms(" Alpha \n\n beta\n"), ["alpha", "beta"]);
        assert!(terms("\n  \n").is_empty());
    }
    #[test]
    fn extra_terms_join_the_main_list_without_duplicates() {
        let list = deny_terms("Alpha\nbeta", "BETA\n gamma \n").unwrap();
        assert_eq!(list, ["alpha", "beta", "gamma"]);
        assert_eq!(deny_terms("alpha", "").unwrap(), ["alpha"]);
    }
    #[test]
    fn an_empty_main_secret_fails_closed_even_with_extra_terms() {
        assert!(deny_terms("", "gamma").is_err());
        assert!(deny_terms(" \n", "").is_err());
    }
    #[test]
    fn hits_report_indexes_not_terms() {
        let list = terms("alpha\nbeta\ngamma");
        assert_eq!(hits(&list, "x BETA y alpha"), [1, 2]);
        assert!(hits(&list, "nothing").is_empty());
    }
    #[test]
    fn added_lines_follow_hunk_numbers_and_skip_headers() {
        let diff = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,0 +3,2 @@\n+one\n+two\n@@ -9 +20 @@\n-old\n+new\n";
        assert_eq!(
            added_lines(diff),
            [
                ("x".into(), 3, "one".into()),
                ("x".into(), 4, "two".into()),
                ("x".into(), 20, "new".into())
            ]
        );
    }
    #[test]
    fn revisions_are_plain_and_null_base_is_detected() {
        assert!(revision("--output=x").is_err());
        assert!(revision("a b").is_err());
        assert!(revision("abc123").is_ok());
        assert!(null_base("0000000000000000000000000000000000000000"));
        assert!(!null_base("abc"));
        assert!(!null_base(""));
    }
}

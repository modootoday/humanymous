//! Commit attribution policy.
//!
//! Co-author trailers may only use GitHub noreply addresses. Commits after the
//! identity cutoff must also carry a GitHub noreply author and committer; the
//! committer may additionally be GitHub's own web-flow address. Failures name
//! the commit and the field, never the address.
use crate::{Result, command};
use std::path::Path;

const ALLOWED_DOMAIN: &str = "@users.noreply.github.com";
/// GitHub's committer address for web-flow merges and edits.
const WEB_FLOW_COMMITTER: &str = "noreply@github.com";
/// Last commit made before the owner noreply identity rule (owner decision
/// 2026-11-16); it and its ancestors are exempt from the identity check.
pub const IDENTITY_CUTOFF: &str = "7e986a5";

/// Returns the offending trailer lines of one commit message.
fn offending(message: &str) -> Vec<String> {
    message
        .lines()
        .map(str::trim)
        .filter(|line| {
            line.split_once(':').is_some_and(|(key, value)| {
                key.trim().eq_ignore_ascii_case("co-authored-by")
                    && !value.trim().strip_suffix('>').is_some_and(|address| {
                        address.to_ascii_lowercase().ends_with(ALLOWED_DOMAIN)
                    })
            })
        })
        .map(str::to_owned)
        .collect()
}

fn noreply(address: &str) -> bool {
    let address = address.trim().to_ascii_lowercase();
    address.len() > ALLOWED_DOMAIN.len()
        && address.ends_with(ALLOWED_DOMAIN)
        && !address.contains(char::is_whitespace)
}

/// Returns the identity fields of one commit that break the rule.
fn identity_failures(author: &str, committer: &str) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if !noreply(author) {
        fields.push("author");
    }
    if !(noreply(committer) || committer.trim().eq_ignore_ascii_case(WEB_FLOW_COMMITTER)) {
        fields.push("committer");
    }
    fields
}

/// Checks every commit in `range` (for example `base..head`).
pub fn check(root: &Path, range: &str) -> Result<()> {
    if range.starts_with('-') || !range.contains("..") {
        return Err("commit-trailers needs one revision range such as base..head".into());
    }
    let shas = command::text(root, "git", &["rev-list", range])?;
    let mut failures = Vec::new();
    for sha in shas.lines().filter(|sha| !sha.is_empty()) {
        let message = command::text(root, "git", &["show", "-s", "--format=%B", sha])?;
        for line in offending(&message) {
            failures.push(format!("{sha}: {line}"));
        }
    }
    // Fails closed when the cutoff object is absent (for example a too-shallow clone).
    let recent = command::text(root, "git", &["rev-list", range, "--not", IDENTITY_CUTOFF])?;
    for sha in recent.lines().filter(|sha| !sha.is_empty()) {
        let fields = command::text(root, "git", &["show", "-s", "--format=%ae%n%ce", sha])?;
        let mut lines = fields.lines();
        let (author, committer) = (lines.next().unwrap_or(""), lines.next().unwrap_or(""));
        for field in identity_failures(author, committer) {
            failures.push(format!("{sha}: {field} is not a GitHub noreply address"));
        }
    }
    if failures.is_empty() {
        println!("commit attribution policy passed for {range}");
        Ok(())
    } else {
        Err(format!(
            "co-author trailers and (after {IDENTITY_CUTOFF}) author/committer must use a {ALLOWED_DOMAIN} address; put provider attribution in the pull request body:\n{}",
            failures.join("\n")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vendor_addresses_are_refused_and_noreply_is_allowed() {
        let message = "feat: x\n\nbody\n\nCo-Authored-By: Claude <noreply@anthropic.com>\nco-authored-by: Someone <1+someone@users.noreply.github.com>\nCO-AUTHORED-BY: codex <codex@openai.com>\n";
        assert_eq!(
            offending(message),
            [
                "Co-Authored-By: Claude <noreply@anthropic.com>",
                "CO-AUTHORED-BY: codex <codex@openai.com>"
            ]
        );
    }
    #[test]
    fn messages_without_coauthors_pass() {
        assert!(offending("fix: y\n\nX-Agent-Provider: claude\n").is_empty());
        assert!(offending("docs: mention Co-Authored-By rules in prose\n").is_empty());
    }
    #[test]
    fn spoofed_suffix_outside_the_address_is_refused() {
        assert_eq!(
            offending("Co-authored-by: a <a@evil.example> @users.noreply.github.com\n").len(),
            1
        );
    }
    #[test]
    fn noreply_identities_pass_and_web_flow_is_committer_only() {
        let owner = "1+owner@users.noreply.github.com";
        assert!(identity_failures(owner, owner).is_empty());
        assert!(identity_failures(owner, "noreply@github.com").is_empty());
        assert_eq!(identity_failures("noreply@github.com", owner), ["author"]);
        assert_eq!(
            identity_failures("noreply@anthropic.com", "dev@example.com"),
            ["author", "committer"]
        );
    }
    #[test]
    fn bare_domain_and_embedded_spaces_are_refused() {
        assert!(!noreply("@users.noreply.github.com"));
        assert!(!noreply("a@evil.example @users.noreply.github.com"));
        assert!(noreply("1+X@Users.Noreply.GitHub.com"));
    }
}

//! Commit co-author trailer policy: only GitHub noreply addresses are allowed.
use crate::{Result, command};
use std::path::Path;

const ALLOWED_DOMAIN: &str = "@users.noreply.github.com";

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
    if failures.is_empty() {
        println!("commit trailer policy passed for {range}");
        Ok(())
    } else {
        Err(format!(
            "co-author trailers must use a {ALLOWED_DOMAIN} address; put provider attribution in the pull request body:\n{}",
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
}

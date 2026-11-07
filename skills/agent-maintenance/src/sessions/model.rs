//! Pure legacy-compatible claim model and conservative glob intersection.
use super::catalog::{Catalog, valid_id};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub lane: String,
    pub provider: String,
    pub session: String,
    pub goal: String,
    pub paths: String,
    pub since: String,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub lane: String,
    pub provider: String,
    pub session: String,
    pub goal: String,
    pub paths: String,
}
impl Request {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 32 * 1024 {
            return Err("request exceeds32KiB".into());
        }
        serde_json::from_str(text).map_err(|e| e.to_string())
    }
    pub fn into_claim(self, since: String) -> Claim {
        Claim {
            lane: self.lane,
            provider: self.provider,
            session: self.session,
            goal: self.goal,
            paths: self.paths,
            since,
            status: "active".into(),
        }
    }
}

pub fn scope(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.len() > 256
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains(['\\', ':', ','])
        || path.chars().any(char::is_whitespace)
        || path.chars().any(char::is_control)
        || path
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return Err(format!("invalid repository-relative scope: {path}"));
    }
    Ok(())
}

impl Claim {
    pub fn parse(text: &str, catalog: &Catalog) -> Result<Self, String> {
        if text.len() > 64 * 1024 {
            return Err("claim exceeds64KiB".into());
        }
        // Accept a legacy UTF-8 BOM, but never last-wins duplicate JSON fields.
        let value: Self =
            serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())?;
        value.validate(catalog)?;
        Ok(value)
    }
    pub fn validate(&self, catalog: &Catalog) -> Result<(), String> {
        if !valid_id(&self.lane) || !catalog.contains(&self.lane) || self.status != "active" {
            return Err("unknown lane or non-active claim".into());
        }
        if !["claude", "grok", "codex", "gemini", "human"].contains(&self.provider.as_str()) {
            return Err("unknown provider".into());
        }
        for (label, text, limit) in [("session", &self.session, 128), ("goal", &self.goal, 4096)] {
            if text.trim().is_empty() || text.len() > limit || text.chars().any(char::is_control) {
                return Err(format!("invalid {label}"));
            }
        }
        time::OffsetDateTime::parse(&self.since, &time::format_description::well_known::Rfc3339)
            .map_err(|_| "invalid RFC3339 since")?;
        if self.paths.len() > 16 * 1024 {
            return Err("scope list exceeds16KiB".into());
        }
        let paths = self.scopes()?;
        if paths.len() > 128 {
            return Err("too many scope patterns".into());
        }
        Ok(())
    }
    pub fn scopes(&self) -> Result<Vec<&str>, String> {
        self.paths
            .split(',')
            .map(str::trim)
            .map(|p| {
                scope(p)?;
                Ok(p)
            })
            .collect()
    }
    pub fn owned_by(&self, provider: &str, session: &str) -> bool {
        self.provider == provider && self.session == session
    }
}

#[derive(Clone, Copy)]
enum Token {
    Literal(char),
    Star(bool),
}
fn tokens(pattern: &str) -> Vec<Token> {
    let mut chars = pattern.chars().peekable();
    let mut result = Vec::new();
    while let Some(c) = chars.next() {
        if c == '*' {
            let recursive = chars.peek() == Some(&'*');
            if recursive {
                chars.next();
            }
            result.push(Token::Star(recursive));
        } else {
            result.push(Token::Literal(c));
        }
    }
    result
}
fn closure(tokens: &[Token], states: impl IntoIterator<Item = usize>) -> BTreeSet<usize> {
    let mut result = states.into_iter().collect::<BTreeSet<_>>();
    loop {
        let additions = result
            .iter()
            .filter(|&&s| matches!(tokens.get(s), Some(Token::Star(_))))
            .map(|s| s + 1)
            .collect::<Vec<_>>();
        let before = result.len();
        result.extend(additions);
        if before == result.len() {
            return result;
        }
    }
}
fn step(tokens: &[Token], states: &BTreeSet<usize>, c: char) -> BTreeSet<usize> {
    closure(
        tokens,
        states.iter().filter_map(|&state| match tokens.get(state) {
            Some(Token::Literal(expected)) if *expected == c => Some(state + 1),
            Some(Token::Star(recursive)) if *recursive || c != '/' => Some(state),
            _ => None,
        }),
    )
}
/// True if glob languages intersect. `*` stays within a segment; `**` crosses `/`.
/// False positives for unusual filesystem names are conservative; budget exhaustion is an error.
pub fn overlap(a: &str, b: &str) -> Result<bool, String> {
    scope(a)?;
    scope(b)?;
    // Conservative case folding also prevents Windows path aliases on other hosts.
    let a = tokens(&a.to_lowercase());
    let b = tokens(&b.to_lowercase());
    let mut alphabet = BTreeSet::from(['/', '_']);
    for t in a.iter().chain(&b) {
        if let Token::Literal(c) = t {
            alphabet.insert(*c);
        }
    }
    let start = (closure(&a, [0]), closure(&b, [0]));
    let mut seen = BTreeSet::from([start.clone()]);
    let mut queue = VecDeque::from([start]);
    while let Some((left, right)) = queue.pop_front() {
        if left.contains(&a.len()) && right.contains(&b.len()) {
            return Ok(true);
        }
        for &c in &alphabet {
            let l = step(&a, &left, c);
            let r = step(&b, &right, c);
            if !l.is_empty() && !r.is_empty() && seen.insert((l.clone(), r.clone())) {
                if seen.len() > 4096 {
                    return Err("scope intersection state budget exceeded".into());
                }
                queue.push_back((l, r));
            }
        }
    }
    Ok(false)
}

/// No stale automatic takeover or alternative-lane alias can authorize overlap.
pub fn admit(candidate: &Claim, existing: &[Claim], catalog: &Catalog) -> Result<(), String> {
    candidate.validate(catalog)?;
    if existing.len() > 128 {
        return Err("active claim count exceeds128".into());
    }
    let mut comparisons = 0usize;
    for current in existing {
        current.validate(catalog)?;
        if current.lane == candidate.lane {
            return Err(format!(
                "lane occupied by {}/{}",
                current.provider, current.session
            ));
        }
        if candidate.lane == "git-ops" || current.lane == "git-ops" {
            continue;
        } // global Git mutex is independent of source lanes
        for a in candidate.scopes()? {
            for b in current.scopes()? {
                comparisons += 1;
                if comparisons > 8192 {
                    return Err("scope comparison budget exceeded".into());
                }
                if overlap(a, b)? {
                    return Err(format!("scope overlaps {}: {a} / {b}", current.lane));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn catalog() -> Catalog {
        Catalog::parse(r#"{"schema_version":1,"writer_mode":"rust-exclusive","lanes":[{"id":"one","kind":"shared","primary_paths":[]},{"id":"two","kind":"shared","primary_paths":[]}]}"#).unwrap()
    }
    fn claim(lane: &str, paths: &str) -> Claim {
        Claim {
            lane: lane.into(),
            provider: "codex".into(),
            session: "mine".into(),
            goal: "scope".into(),
            paths: paths.into(),
            since: "2026-11-07T16:00:00+09:00".into(),
            status: "active".into(),
        }
    }
    #[test]
    fn glob_intersection_handles_shared_and_disjoint_patterns() {
        for (a, b, expected) in [
            ("crates/**", "crates/audit/src/lib.rs", true),
            ("crates/*/src/*.rs", "crates/audit/src/lib.rs", true),
            ("crates/*", "crates/audit/src/lib.rs", false),
            (
                ".spec/plan/*audit*.plan.md",
                ".spec/plan/*cli*.plan.md",
                true,
            ),
            ("crates/audit/**", "crates/cli/**", false),
            ("x/*.sot.md", "x/*.plan.md", false),
        ] {
            assert_eq!(overlap(a, b).unwrap(), expected, "{a} / {b}");
        }
    }
    #[test]
    fn different_lane_name_cannot_hide_overlap_or_take_over_same_lane() {
        let c = catalog();
        assert!(
            admit(
                &claim("two", "crates/audit/**"),
                &[claim("one", "crates/**")],
                &c
            )
            .is_err()
        );
        assert!(
            admit(
                &claim("one", "other/**"),
                &[claim("one", "original/**")],
                &c
            )
            .is_err()
        );
        assert!(
            admit(
                &claim("two", "crates/cli/**"),
                &[claim("one", "crates/audit/**")],
                &c
            )
            .is_ok()
        );
    }
    #[test]
    fn malformed_scope_identity_time_and_duplicates_refuse() {
        for p in ["../secret", "/root", "a//b", "a/** (except b)", "C:\\x", ""] {
            assert!(scope(p).is_err());
        }
        let c = catalog();
        let mut x = claim("one", "a/**");
        x.since = "yesterday".into();
        assert!(x.validate(&c).is_err());
        let wire = serde_json::to_string(&claim("one", "a/**"))
            .unwrap()
            .replacen("\"lane\":\"one\"", "\"lane\":\"one\",\"lane\":\"two\"", 1);
        assert!(Claim::parse(&wire, &c).is_err());
    }
    #[test]
    fn owner_match_requires_provider_and_session() {
        let c = claim("one", "a");
        assert!(c.owned_by("codex", "mine"));
        assert!(!c.owned_by("claude", "mine"));
        assert!(!c.owned_by("codex", "other"));
    }
    #[test]
    fn case_aliases_overlap_and_request_cannot_supply_ownership_state() {
        assert!(overlap("Crates/Audit/**", "crates/audit/src/lib.rs").unwrap());
        assert!(Request::parse(r#"{"lane":"one","provider":"codex","session":"mine","goal":"x","paths":"a","status":"active"}"#).is_err());
    }
}

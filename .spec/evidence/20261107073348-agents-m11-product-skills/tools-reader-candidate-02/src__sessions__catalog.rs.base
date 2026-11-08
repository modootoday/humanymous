//! JSON lane authority and read-only legacy projection drift.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema_version: u8,
    pub writer_mode: WriterMode,
    pub lanes: Vec<Lane>,
}
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WriterMode {
    LegacyCoexistence,
    RustExclusive,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Lane {
    pub id: String,
    pub kind: String,
    pub primary_paths: Vec<String>,
}
#[derive(Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub lane_count: usize,
    pub errors: Vec<String>,
    pub legacy_projection_qualified: bool,
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.as_bytes()[0].is_ascii_lowercase()
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
impl Catalog {
    pub fn parse(text: &str) -> Result<Self, String> {
        let catalog: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if catalog.schema_version != 1 || catalog.lanes.is_empty() {
            return Err("unsupported/empty lane catalog".into());
        }
        let mut seen = BTreeSet::new();
        for lane in &catalog.lanes {
            if !valid_id(&lane.id) || !seen.insert(lane.id.clone()) {
                return Err("invalid/duplicate lane id".into());
            }
            if !["legacy", "shared", "coordination", "rust"].contains(&lane.kind.as_str()) {
                return Err("unknown lane kind".into());
            }
            for path in &lane.primary_paths {
                super::model::scope(path)?;
            }
        }
        Ok(catalog)
    }
    pub fn ids(&self) -> BTreeSet<String> {
        self.lanes.iter().map(|l| l.id.clone()).collect()
    }
    pub fn contains(&self, id: &str) -> bool {
        self.lanes.iter().any(|l| l.id == id)
    }
}

fn markdown_ids(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            if !line.starts_with('|') {
                return None;
            }
            let id = line.split('|').nth(1)?.trim().trim_matches('`');
            (valid_id(id) && id != "lane").then(|| id.to_owned())
        })
        .collect()
}
fn ps_ids(text: &str) -> Result<BTreeSet<String>, String> {
    let content = text
        .split_once("$ValidLanes = @(")
        .ok_or("missing PS allowlist")?
        .1
        .split_once(')')
        .ok_or("unterminated PS allowlist")?
        .0;
    let regex = regex::Regex::new(r#""([a-z0-9-]+)""#).expect("literal pattern");
    Ok(regex.captures_iter(content).map(|m| m[1].into()).collect())
}
fn bash_ids(text: &str) -> Result<BTreeSet<String>, String> {
    let content = text
        .lines()
        .find_map(|line| line.strip_prefix("VALID=\""))
        .and_then(|s| s.strip_suffix('"'))
        .ok_or("missing bash allowlist")?;
    Ok(content.split_whitespace().map(str::to_owned).collect())
}
/// Compare content without executing either legacy script.
pub fn validate_projections(catalog: &Catalog, markdown: &str, ps: &str, bash: &str) -> Report {
    let expected = catalog.ids();
    let mut errors = Vec::new();
    for (label, actual) in [
        ("LANES.md", Ok(markdown_ids(markdown))),
        ("PowerShell", ps_ids(ps)),
        ("bash", bash_ids(bash)),
    ] {
        match actual {
            Ok(ids) if ids != expected => errors.push(format!(
                "{label}: missing={:?} extra={:?}",
                expected.difference(&ids).collect::<Vec<_>>(),
                ids.difference(&expected).collect::<Vec<_>>()
            )),
            Err(e) => errors.push(format!("{label}: {e}")),
            _ => {}
        }
    }
    Report {
        schema: "humanymous.session-catalog.v1",
        lane_count: expected.len(),
        errors,
        legacy_projection_qualified: false,
    }
}
pub fn inspect(root: &Path) -> Result<Report, Box<dyn std::error::Error>> {
    let catalog = Catalog::parse(&fs::read_to_string(
        root.join(".agents/sessions/lanes.json"),
    )?)?;
    Ok(validate_projections(
        &catalog,
        &fs::read_to_string(root.join(".agents/sessions/LANES.md"))?,
        &fs::read_to_string(root.join("scripts/agents/session-board.ps1"))?,
        &fs::read_to_string(root.join("scripts/agents/session-board.sh"))?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repository_catalog_matches_all_compatibility_projections() {
        let c = Catalog::parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../.agents/sessions/lanes.json"
        )))
        .unwrap();
        let r = validate_projections(
            &c,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../.agents/sessions/LANES.md"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../scripts/agents/session-board.ps1"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../scripts/agents/session-board.sh"
            )),
        );
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        assert_eq!(r.lane_count, 25);
    }
    fn catalog() -> Catalog {
        Catalog::parse(r#"{"schema_version":1,"writer_mode":"legacy-coexistence","lanes":[{"id":"agents-meta","kind":"shared","primary_paths":[".agents/**"]}]}"#).unwrap()
    }
    #[test]
    fn exact_projections_are_static_not_runtime_qualification() {
        let r = validate_projections(
            &catalog(),
            "| `agents-meta` | scope |",
            "$ValidLanes = @(\"agents-meta\")",
            "VALID=\"agents-meta\"",
        );
        assert!(r.errors.is_empty());
        assert!(!r.legacy_projection_qualified);
    }
    #[test]
    fn drift_and_malformed_projection_fail() {
        assert_eq!(
            validate_projections(&catalog(), "| wrong | x |", "broken", "VALID=\"wrong\"")
                .errors
                .len(),
            3
        );
    }
    #[test]
    fn rejects_duplicate_fields_ids_and_versions() {
        for text in [
            r#"{"schema_version":2,"lanes":[]}"#,
            r#"{"schema_version":1,"schema_version":1,"lanes":[]}"#,
            r#"{"schema_version":1,"lanes":[{"id":"../x","kind":"shared","primary_paths":[]}]}"#,
        ] {
            assert!(Catalog::parse(text).is_err());
        }
    }
}

//! Deterministic legacy routing proxy; never a claim about model activation.
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};
#[derive(Deserialize)]
struct Rules {
    rules: BTreeMap<String, Vec<String>>,
}
#[derive(Deserialize)]
struct Queries {
    queries: Vec<Query>,
}
#[derive(Deserialize)]
struct Query {
    query: String,
    skill: String,
    should_trigger: bool,
    split: String,
}
#[derive(Serialize)]
pub struct Row {
    query: String,
    skill: String,
    split: String,
    expected: bool,
    actual: bool,
}
#[derive(Serialize)]
pub struct Report {
    pub schema_version: u8,
    pub total: usize,
    pub failed: usize,
    pub rows: Vec<Row>,
    pub elapsed_micros: u128,
}
pub fn evaluate(root: &Path) -> Result<Report, Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let rules: Rules =
        serde_json::from_slice(&fs::read(root.join(".agents/evals/trigger-rules.json"))?)?;
    let queries: Queries =
        serde_json::from_slice(&fs::read(root.join(".agents/evals/trigger-queries.json"))?)?;
    let patterns = rules
        .rules
        .into_iter()
        .map(|(skill, patterns)| {
            Ok((
                skill,
                patterns
                    .into_iter()
                    .map(|p| RegexBuilder::new(&p).case_insensitive(true).build())
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, regex::Error>>()?;
    let mut rows = Vec::new();
    let mut failed = 0;
    for q in queries.queries {
        let actual = patterns
            .get(&q.skill)
            .ok_or_else(|| format!("missing routing rule {}", q.skill))?
            .iter()
            .any(|r| r.is_match(&q.query));
        failed += usize::from(actual != q.should_trigger);
        rows.push(Row {
            query: q.query,
            skill: q.skill,
            split: q.split,
            expected: q.should_trigger,
            actual,
        });
    }
    if rows.is_empty() {
        return Err("empty routing dataset".into());
    }
    Ok(Report {
        schema_version: 1,
        total: rows.len(),
        failed,
        rows,
        elapsed_micros: started.elapsed().as_micros(),
    })
}

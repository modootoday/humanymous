//! Exact-byte synthetic experiments reuse the shared parser, never a copied policy.
mod author;
mod i18n;
use humanymous_integrity::adapter::{AdapterProfile, parse_with_profile};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, io::Write, path::Path};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema: String,
    scope: String,
    now_ms: u64,
    cases: Vec<Case>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    profile: String,
    input_hex: String,
    sha256: String,
    accept: bool,
}

/// Parser execution evidence, explicitly narrower than product conformance.
#[derive(Debug, Serialize)]
pub struct VectorReport {
    /// Complete fixture digest, not an automatically refreshed acceptance record.
    pub fixture_sha256: String,
    /// Positive parser acceptances, not ALLOW verdicts or origin effects.
    pub accepted: usize,
    /// Negative parser rejections, not DENY verdicts.
    pub rejected: usize,
    /// Every expectation mismatch, without skips.
    pub errors: Vec<String>,
    /// Always false: one shared parser is not independent interoperability.
    pub draft_v0_conformance: bool,
}

/// Read and execute every authored exact-byte case using the real shared parser.
pub fn check(path: &Path) -> Result<VectorReport, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() > 2_000_000 {
        return Err("fixture budget exceeded".into());
    }
    let corpus: Corpus = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if corpus.schema != "humanymous.experimental-parser-vectors/1"
        || corpus.cases.is_empty()
        || corpus.cases.len() > 256
    {
        return Err("unsupported or empty fixture corpus".into());
    }
    evaluate(corpus, hex::encode(Sha256::digest(bytes)))
}

fn evaluate(corpus: Corpus, fixture_sha256: String) -> Result<VectorReport, String> {
    let mut report = VectorReport {
        fixture_sha256,
        accepted: 0,
        rejected: 0,
        errors: Vec::new(),
        draft_v0_conformance: false,
    };
    let mut ids = BTreeSet::new();
    for case in corpus.cases {
        if !ids.insert(case.id.clone()) {
            report.errors.push(format!("duplicate case {}", case.id));
        }
        let bytes = hex::decode(&case.input_hex).map_err(|e| format!("{}: {e}", case.id))?;
        if hex::encode(&bytes) != case.input_hex
            || hex::encode(Sha256::digest(&bytes)) != case.sha256
        {
            report
                .errors
                .push(format!("{}: encoding/digest mismatch", case.id));
            continue;
        }
        let profile = match case.profile.as_str() {
            "node" => AdapterProfile::Node,
            "nginx" => AdapterProfile::Nginx,
            "python-asgi" => AdapterProfile::PythonAsgi,
            "python-wsgi" => AdapterProfile::PythonWsgi,
            "ruby-rack" => AdapterProfile::RubyRack,
            "apache" => AdapterProfile::Apache,
            "aspnetcore" => AdapterProfile::DotnetAspNetCore,
            _ => return Err(format!("{}: unknown trusted parser profile", case.id)),
        };
        let accepted = parse_with_profile(&bytes, corpus.now_ms, profile).is_ok();
        if accepted {
            report.accepted += 1;
        } else {
            report.rejected += 1;
        }
        if accepted != case.accept {
            report.errors.push(format!(
                "{}: expected accept={}, actual={accepted}",
                case.id, case.accept
            ));
        }
    }
    Ok(report)
}

/// Explicit synthetic-fixture authoring command. Refuses overwrite; never run by CI.
pub fn export(path: &Path) -> Result<(), String> {
    export_corpus(path, &author::corpus())
}

/// Author the separate synthetic internationalization corpus; refuses overwrite.
pub fn export_i18n(path: &Path) -> Result<(), String> {
    export_corpus(path, &i18n::corpus())
}

fn export_corpus(path: &Path, corpus: &Corpus) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(corpus).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalized_host_profiles_use_fixed_positive_and_negative_bytes() {
        let corpus: Corpus = serde_json::from_str(include_str!(
            "../../../standards/draft-v0/validation/fixtures/adapter-normalized-profiles-v1.json"
        ))
        .unwrap();
        assert_eq!(corpus.cases.len(), 25);
        let report = evaluate(corpus, "test".into()).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!((report.accepted, report.rejected), (5, 20));
        assert!(!report.draft_v0_conformance);
    }
    #[test]
    fn binary_and_unicode_corpus_preserves_distinct_commitment_bytes() {
        let corpus = i18n::corpus();
        assert_eq!(corpus.cases.len(), 10);
        let report = evaluate(corpus, "test".into()).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!((report.accepted, report.rejected), (6, 4));
        assert!(!report.draft_v0_conformance);
    }
    #[test]
    fn expectations_are_fixed_and_duplicate_key_bytes_are_not_round_tripped() {
        let corpus = author::corpus();
        assert_eq!(corpus.cases.len(), 24);
        let escaped = corpus
            .cases
            .iter()
            .find(|c| c.id == "escaped-duplicate-key")
            .unwrap();
        assert!(
            String::from_utf8(hex::decode(&escaped.input_hex).unwrap())
                .unwrap()
                .contains("protoco\\u006c")
        );
        let report = evaluate(corpus, "test".into()).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!((report.accepted, report.rejected), (4, 20));
        assert!(!report.draft_v0_conformance);
    }
    #[test]
    fn changed_digest_or_expectation_and_duplicate_id_fail() {
        let mut corpus = author::corpus();
        corpus.cases[0].sha256 = "0".repeat(64);
        corpus.cases[1].accept = false;
        corpus.cases[2].id = corpus.cases[1].id.clone();
        let report = evaluate(corpus, "test".into()).unwrap();
        assert!(report.errors.iter().any(|e| e.contains("digest")));
        assert!(report.errors.iter().any(|e| e.contains("expected")));
        assert!(report.errors.iter().any(|e| e.contains("duplicate case")));
    }
}

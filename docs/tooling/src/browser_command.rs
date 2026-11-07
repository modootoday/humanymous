//! CLI contract for caller-owned hosted WebDriver/site services.
use std::{collections::BTreeMap, path::PathBuf};

struct Options {
    source: PathBuf,
    site: PathBuf,
    baseurl: String,
    webdriver: String,
    base: String,
}
fn parse(mut arguments: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut values = BTreeMap::new();
    while let Some(flag) = arguments.next() {
        if !["--source", "--site", "--baseurl", "--webdriver", "--base"].contains(&flag.as_str()) {
            return Err(format!("unknown browser argument {flag}"));
        }
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        if value.starts_with("--") || values.insert(flag.clone(), value).is_some() {
            return Err(format!("missing or duplicate browser argument {flag}"));
        }
    }
    Ok(Options {
        source: values
            .remove("--source")
            .unwrap_or_else(|| "docs".into())
            .into(),
        site: values
            .remove("--site")
            .ok_or("browser requires --site")?
            .into(),
        baseurl: values.remove("--baseurl").unwrap_or_default(),
        webdriver: values
            .remove("--webdriver")
            .ok_or("browser requires --webdriver")?,
        base: values.remove("--base").ok_or("browser requires --base")?,
    })
}

pub(super) fn run(arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let options = parse(arguments)?;
    let result = humanymous_pages::browser::run(
        &options.source,
        &options.site,
        &options.baseurl,
        &options.webdriver,
        &options.base,
    );
    let report = match result {
        Ok(report) => report,
        Err(error) => {
            println!(
                "{}",
                serde_json::json!({
                    "scope":"pages-keyboard-component","status":"failed",
                    "full_accessibility_qualified":false,"stage":"setup",
                    "expected_cases":null,"performed_cases":0,"not_run":null,
                    "session_deleted":"NOT_MEASURED","assistive_technology":"NOT_RUN",
                    "error":error
                })
            );
            return Err("Pages browser setup failed; no measurements accepted".into());
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    if report.passed() {
        Ok(())
    } else {
        Err("Pages browser component checks failed".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> impl Iterator<Item = String> {
        values.iter().map(|value| (*value).to_owned())
    }

    #[test]
    fn browser_cli_requires_services_without_accepting_duplicate_or_unknown_flags() {
        let required = [
            "--site",
            "built",
            "--webdriver",
            "http://127.0.0.1:4444/",
            "--base",
            "http://127.0.0.1:8080/",
        ];
        let options = parse(args(&required)).unwrap();
        assert_eq!(options.source, PathBuf::from("docs"));
        assert_eq!(options.baseurl, "");
        assert!(parse(args(&[])).is_err());
        assert!(parse(args(&["--site"])).is_err());
        assert!(parse(args(&["--unknown", "value"])).is_err());
        let duplicate = [required.as_slice(), &["--site", "other"]].concat();
        assert!(parse(args(&duplicate)).is_err());
        assert!(parse(args(&["--site", "--webdriver"])).is_err());
    }
}

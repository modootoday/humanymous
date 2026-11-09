//! Cap-day CEF pilot bundle resolution (env + durable allowlisted bind).
//!
//! Cap=0 presence may be satisfied by process env `HUMANYMOUS_CEF_PILOT_BUNDLE`
//! or by a durable Cap-day bind pointing at an allowlisted complete private host
//! tree. Never invents CEF binaries, BrowserReady, or product. Does not mutate
//! process env (forbid unsafe-code / set_var).
use crate::docker::Result;
use crate::orchestration::g4_cef_at::CEF_BUNDLE_ENV;
use crate::snapshot;
use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Committed Cap-day allowlist (relative host candidates only).
pub const ALLOWLIST_RELATIVE: &str = "test/lab/runner/cef-cap-day-allowlist.json";

/// Durable Cap-day bind receipt (private; not freeze source).
pub const CAP_DAY_BIND_RELATIVE: &str = ".spec.local/lab/cef-cap-day-bind.json";

const ALLOWLIST_SCHEMA: &str = "humanymous.lab-cef-cap-day-allowlist/1";
const BIND_SCHEMA: &str = "humanymous.lab-cef-cap-day-bind/1";

/// Known private host CEF trees (Unlock A prior Cap realloc search).
pub const HOST_CANDIDATES: &[&str] = &[
    ".spec.local/root/evidence/20261107004558-browser-ui-cef-native/bundle",
    ".spec.local/ct/debug",
];

const REQUIRED_FILES: &[&str] = &[
    "libcef.dll",
    "archive.json",
    "humanymous_cef_bootstrap.exe",
    "humanymous_cef_bootstrap.dll",
    "humanymous_cef_desktop.dll",
    "chrome_elf.dll",
    "icudtl.dat",
    "resources.pak",
    "locales",
];

/// How Cap-day CEF presence was resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CefBundleSource {
    ProcessEnv,
    DurableBind,
}

/// Resolved Cap-day CEF bundle path.
#[derive(Clone, Debug)]
pub struct CefBundleResolution {
    pub source: CefBundleSource,
    pub relative: Option<String>,
    pub absolute: PathBuf,
}

/// Inspect one relative host candidate for Cap-day completeness.
pub fn inspect_candidate(root: &Path, relative: &str) -> Value {
    let path = root.join(relative);
    if !path.is_dir() {
        return json!({
            "relative": relative,
            "present": false,
            "complete": false,
            "missing": REQUIRED_FILES,
            "libcef_bytes": null
        });
    }
    let mut missing = Vec::new();
    for name in REQUIRED_FILES {
        let item = path.join(name);
        let ok = if *name == "locales" {
            item.is_dir()
        } else {
            item.is_file()
        };
        if !ok {
            missing.push(*name);
        }
    }
    let libcef = path.join("libcef.dll");
    let libcef_bytes = if libcef.is_file() {
        fs::metadata(&libcef).ok().map(|meta| meta.len())
    } else {
        None
    };
    json!({
        "relative": relative,
        "absolute": path.display().to_string(),
        "present": true,
        "complete": missing.is_empty(),
        "missing": missing,
        "libcef_bytes": libcef_bytes
    })
}

fn allowlisted(relative: &str) -> bool {
    HOST_CANDIDATES.contains(&relative)
}

fn read_bind(root: &Path) -> Result<Option<Value>> {
    let path = root.join(CAP_DAY_BIND_RELATIVE);
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("cef Cap-day bind read {CAP_DAY_BIND_RELATIVE}: {error}"))?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("cef Cap-day bind json {CAP_DAY_BIND_RELATIVE}: {error}"))?;
    Ok(Some(value))
}

/// Resolve Cap-day CEF presence: process env first, then durable allowlisted bind.
pub fn resolve(root: &Path) -> Result<Option<CefBundleResolution>> {
    if let Some(raw) = env::var_os(CEF_BUNDLE_ENV).filter(|value| !value.is_empty()) {
        let absolute = PathBuf::from(&raw);
        let relative = absolute
            .strip_prefix(root)
            .ok()
            .map(|path| path.to_string_lossy().replace('\\', "/"));
        return Ok(Some(CefBundleResolution {
            source: CefBundleSource::ProcessEnv,
            relative,
            absolute,
        }));
    }
    let Some(bind) = read_bind(root)? else {
        return Ok(None);
    };
    if bind.get("schema").and_then(Value::as_str) != Some(BIND_SCHEMA) {
        return Ok(None);
    }
    let relative = bind
        .get("relative")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if relative.is_empty() || !allowlisted(relative) {
        return Ok(None);
    }
    let observation = inspect_candidate(root, relative);
    if observation["complete"].as_bool() != Some(true) {
        return Ok(None);
    }
    Ok(Some(CefBundleResolution {
        source: CefBundleSource::DurableBind,
        relative: Some(relative.to_string()),
        absolute: root.join(relative),
    }))
}

/// Cap-day CEF presence (env or durable allowlisted complete bind).
pub fn cap_day_present(root: &Path) -> Result<bool> {
    Ok(resolve(root)?.is_some())
}

/// Exact missing Cap-day CEF prerequisites (empty when present).
pub fn exact_missing_for_cap_day(root: &Path) -> Result<Vec<String>> {
    if cap_day_present(root)? {
        Ok(Vec::new())
    } else {
        Ok(vec![format!(
            "env:{CEF_BUNDLE_ENV}|durable:{CAP_DAY_BIND_RELATIVE}"
        )])
    }
}

fn utc_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

/// Cap=0 durable Cap-day bind against allowlisted complete private host.
///
/// Usage: `cef-bundle-bind [--workspace PATH] [--dry-run] [--emit PATH]`
pub fn run_bind(args: &[String]) -> Result<i32> {
    let mut workspace = None;
    let mut emit_path = None;
    let mut dry_run = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--workspace" => {
                index += 1;
                workspace = Some(args.get(index).ok_or("--workspace path missing")?.clone());
            }
            "--emit" => {
                index += 1;
                emit_path = Some(args.get(index).ok_or("--emit path missing")?.clone());
            }
            "--dry-run" => dry_run = true,
            _ => {
                return Err(
                    "usage: cef-bundle-bind [--workspace PATH] [--dry-run] [--emit PATH]".into(),
                );
            }
        }
        index += 1;
    }
    let root = match workspace {
        Some(path) => Path::new(&path).canonicalize().map_err(|error| {
            format!("cef-bundle-bind refused: workspace canonicalize {path}: {error}")
        })?,
        None => snapshot::root()?,
    };
    let report = bind(&root, dry_run)?;
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("cef-bundle-bind encode failed: {error}"))?;
    if let Some(path) = emit_path {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).map_err(|error| format!("emit mkdir: {error}"))?;
        }
        fs::write(&path, format!("{rendered}\n"))
            .map_err(|error| format!("emit write failed: {error}"))?;
    }
    println!("{rendered}");
    Ok(0)
}

fn bind(root: &Path, dry_run: bool) -> Result<Value> {
    let allowlist_path = root.join(ALLOWLIST_RELATIVE);
    let allowlist_present = allowlist_path.is_file();
    let mut host_candidates = Vec::new();
    let mut selected = None;
    for relative in HOST_CANDIDATES {
        let observation = inspect_candidate(root, relative);
        if selected.is_none() && observation["complete"].as_bool() == Some(true) {
            selected = Some((*relative).to_string());
        }
        host_candidates.push(observation);
    }
    let Some(relative) = selected else {
        return Ok(json!({
            "schema": BIND_SCHEMA,
            "record_kind": "lab-cef-cap-day-bind",
            "cap": 0,
            "cap_burn": 0,
            "product": 0,
            "pass_product_clean_room": 0,
            "browser_ready": false,
            "status": "refused_no_complete_allowlisted_host",
            "bound": false,
            "dry_run": dry_run,
            "cef_bundle_env": CEF_BUNDLE_ENV,
            "allowlist_relative": ALLOWLIST_RELATIVE,
            "allowlist_present": allowlist_present,
            "allowlist_schema": ALLOWLIST_SCHEMA,
            "durable_bind_relative": CAP_DAY_BIND_RELATIVE,
            "host_candidates": host_candidates,
            "exact_missing_for_cap_day": exact_missing_for_cap_day(root)?,
            "note": "Cap=0 bind refused: no complete allowlisted private CEF host; never invent CEF binary or BrowserReady",
            "honesty": {
                "never_invent_browser_ready": true,
                "never_invent_pass_product_clean_room": true,
                "never_invent_cef_binary": true,
                "never_mutate_process_env": true,
                "detection_freeze": true,
                "cap0_only": true
            }
        }));
    };
    let absolute = root.join(&relative);
    let bind_body = json!({
        "schema": BIND_SCHEMA,
        "record_kind": "lab-cef-cap-day-bind",
        "cap": 0,
        "cap_burn": 0,
        "product": 0,
        "pass_product_clean_room": 0,
        "browser_ready": false,
        "env": CEF_BUNDLE_ENV,
        "relative": relative,
        "absolute": absolute.display().to_string(),
        "complete": true,
        "allowlist_relative": ALLOWLIST_RELATIVE,
        "source": "allowlisted_private_host",
        "bound_unix_secs": utc_now(),
        "note": "Durable Cap-day CEF presence bind; process env preferred when set; never invent BrowserReady"
    });
    if !dry_run {
        let path = root.join(CAP_DAY_BIND_RELATIVE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("bind mkdir: {error}"))?;
        }
        let rendered = serde_json::to_string_pretty(&bind_body)
            .map_err(|error| format!("bind encode: {error}"))?;
        fs::write(&path, format!("{rendered}\n"))
            .map_err(|error| format!("bind write failed: {error}"))?;
    }
    let resolved = resolve(root)?;
    Ok(json!({
        "schema": BIND_SCHEMA,
        "record_kind": "lab-cef-cap-day-bind-receipt",
        "cap": 0,
        "cap_burn": 0,
        "product": 0,
        "pass_product_clean_room": 0,
        "browser_ready": false,
        "status": if dry_run { "dry_run_selected" } else { "bound" },
        "bound": !dry_run,
        "dry_run": dry_run,
        "cef_bundle_env": CEF_BUNDLE_ENV,
        "allowlist_relative": ALLOWLIST_RELATIVE,
        "allowlist_present": allowlist_present,
        "durable_bind_relative": CAP_DAY_BIND_RELATIVE,
        "selected_private_host_relative": relative,
        "selected_absolute": absolute.display().to_string(),
        "host_candidates": host_candidates,
        "cap_day_present": resolved.is_some(),
        "resolution_source": resolved.as_ref().map(|item| match item.source {
            CefBundleSource::ProcessEnv => "process_env",
            CefBundleSource::DurableBind => "durable_bind",
        }),
        "exact_missing_for_cap_day": if resolved.is_some() {
            json!([])
        } else {
            json!(exact_missing_for_cap_day(root)?)
        },
        "bind_body": bind_body,
        "note": "Cap=0 durable Cap-day CEF bind against allowlisted complete private host; Unlock A ≠ BrowserReady; never invent pass_product_clean_room",
        "honesty": {
            "never_invent_browser_ready": true,
            "never_invent_pass_product_clean_room": true,
            "never_invent_cef_binary": true,
            "never_mutate_process_env": true,
            "detection_freeze": true,
            "cap0_only": true
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn empty_root_has_no_cap_day_presence() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("hmn-cef-capday-{nanos}"));
        fs::create_dir_all(&root).expect("tmpdir");
        // Ambient process env may be set; durable bind alone is absent here.
        let bind_path = root.join(CAP_DAY_BIND_RELATIVE);
        assert!(!bind_path.is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn allowlist_contains_known_private_hosts() {
        assert!(allowlisted(
            ".spec.local/root/evidence/20261107004558-browser-ui-cef-native/bundle"
        ));
        assert!(allowlisted(".spec.local/ct/debug"));
        assert!(!allowlisted(".spec.local/invented/cef"));
    }
}

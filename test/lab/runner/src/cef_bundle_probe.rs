//! Cap=0 CEF pilot bundle presence probe.
//!
//! Exact missing-path receipt for Cap day: env `HUMANYMOUS_CEF_PILOT_BUNDLE`
//! and/or durable Cap-day bind vs private host trees (Unlock A prior search).
//! Never invents CEF binary / BrowserReady / product. Cap burn stays 0. Does
//! not set the env (see `cef-bundle-bind` for durable Cap-day bind).
use crate::cef_bundle::{
    self, ALLOWLIST_RELATIVE, CAP_DAY_BIND_RELATIVE, CefBundleSource, HOST_CANDIDATES,
};
use crate::docker::Result;
use crate::orchestration::g4_cef_at::{CEF_BUNDLE_ENV, PATH_RELEASE_SURFACES};
use crate::snapshot;
use serde_json::{Value, json};
use std::{env, fs, path::Path};

const SCHEMA: &str = "humanymous.lab-cef-bundle-probe/1";

/// Cap=0 CEF bundle probe.
///
/// Usage: `cef-bundle-probe [--workspace PATH] [--emit PATH]`
pub fn run(args: &[String]) -> Result<i32> {
    let mut workspace = None;
    let mut emit_path = None;
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
            _ => {
                return Err("usage: cef-bundle-probe [--workspace PATH] [--emit PATH]".into());
            }
        }
        index += 1;
    }
    let root = match workspace {
        Some(path) => Path::new(&path).canonicalize().map_err(|error| {
            format!("cef-bundle-probe refused: workspace canonicalize {path}: {error}")
        })?,
        None => snapshot::root()?,
    };
    let report = probe(&root)?;
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("cef-bundle-probe encode failed: {error}"))?;
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

fn probe(root: &Path) -> Result<Value> {
    let env_raw = env::var_os(CEF_BUNDLE_ENV);
    let env_set = env_raw.as_ref().is_some_and(|value| !value.is_empty());
    let env_value = env_raw
        .as_ref()
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string_lossy().into_owned());

    let mut host_candidates = Vec::new();
    let mut selected_host = None;
    for relative in HOST_CANDIDATES {
        let observation = cef_bundle::inspect_candidate(root, relative);
        if selected_host.is_none() && observation["complete"].as_bool() == Some(true) {
            selected_host = Some(relative.to_string());
        }
        host_candidates.push(observation);
    }

    let resolved = cef_bundle::resolve(root)?;
    let cap_day_present = resolved.is_some();
    let exact_missing = cef_bundle::exact_missing_for_cap_day(root)?;
    let host_found = selected_host.is_some();
    let resolution_source = resolved.as_ref().map(|item| match item.source {
        CefBundleSource::ProcessEnv => "process_env",
        CefBundleSource::DurableBind => "durable_bind",
    });
    let resolved_relative = resolved.as_ref().and_then(|item| item.relative.clone());
    let resolved_absolute = resolved
        .as_ref()
        .map(|item| item.absolute.display().to_string());
    let status = if cap_day_present {
        if matches!(
            resolved.as_ref().map(|item| &item.source),
            Some(CefBundleSource::DurableBind)
        ) {
            "cap_day_bound"
        } else {
            "env_bound"
        }
    } else if host_found {
        "env_absent_private_host_present"
    } else {
        "env_absent_host_absent"
    };
    Ok(json!({
        "schema": SCHEMA,
        "record_kind": "lab-cef-bundle-probe",
        "cap": 0,
        "cap_burn": 0,
        "product": 0,
        "pass_product_clean_room": 0,
        "browser_ready": false,
        "cef_bundle_env": CEF_BUNDLE_ENV,
        "cef_bundle_env_present": env_set,
        "cef_bundle_env_value": env_value,
        "cap_day_present": cap_day_present,
        "resolution_source": resolution_source,
        "resolved_relative": resolved_relative,
        "resolved_absolute": resolved_absolute,
        "allowlist_relative": ALLOWLIST_RELATIVE,
        "durable_bind_relative": CAP_DAY_BIND_RELATIVE,
        "durable_bind_present": root.join(CAP_DAY_BIND_RELATIVE).is_file(),
        "exact_missing": exact_missing.clone(),
        "exact_missing_for_cap_day": exact_missing,
        "host_candidates": host_candidates,
        "private_host_bundle_found": host_found,
        "selected_private_host_relative": selected_host,
        "selected_reason": if host_found {
            "complete private evidence CEF runtime (archive.json + libcef.dll + locales); not invented; Cap day uses env or durable cef-bundle-bind"
        } else {
            "no complete private host CEF tree observed"
        },
        "path_release_surfaces": PATH_RELEASE_SURFACES,
        "status": status,
        "cap0_helpers": {
            "cef_bundle_bind": "cargo run --locked -p humanymous-lab-runner -- cef-bundle-bind",
            "cef_bundle_probe": "cargo run --locked -p humanymous-lab-runner -- cef-bundle-probe"
        },
        "note": "Cap=0 probe only; Unlock A g4 component ≠ BrowserReady; Cap day presence via process env or durable allowlisted bind; never invent CEF binary or BrowserReady",
        "honesty": {
            "never_set_env_from_probe": true,
            "never_invent_cef_binary": true,
            "never_invent_browser_ready": true,
            "never_invent_pass_product_clean_room": true,
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
    fn empty_root_reports_env_and_host_absent() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("hmn-cef-probe-{nanos}"));
        fs::create_dir_all(&root).expect("tmpdir");
        let report = probe(&root).expect("probe");
        assert_eq!(report["schema"], SCHEMA);
        assert_eq!(report["product"], 0);
        assert_eq!(report["browser_ready"], false);
        assert_eq!(report["private_host_bundle_found"], false);
        assert!(
            report["exact_missing_for_cap_day"]
                .as_array()
                .expect("arr")
                .iter()
                .any(|item| item
                    .as_str()
                    .is_some_and(|text| text.contains(CEF_BUNDLE_ENV)))
                || report["cap_day_present"].as_bool() == Some(true)
        );
        let _ = fs::remove_dir_all(&root);
    }
}

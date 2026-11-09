//! Cap=0 CEF pilot bundle presence probe.
//!
//! Exact missing-path receipt for Cap day: env `HUMANYMOUS_CEF_PILOT_BUNDLE`
//! vs private host trees (Unlock A prior search). Never invents CEF binary /
//! BrowserReady / product. Cap burn stays 0. Does not set the env.
use crate::docker::Result;
use crate::orchestration::g4_cef_at::{CEF_BUNDLE_ENV, PATH_RELEASE_SURFACES};
use crate::snapshot;
use serde_json::{Value, json};
use std::{env, fs, path::Path};

const SCHEMA: &str = "humanymous.lab-cef-bundle-probe/1";

/// Known private host CEF trees (prior Unlock A Cap realloc search).
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

fn inspect_candidate(root: &Path, relative: &str) -> Value {
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
        let observation = inspect_candidate(root, relative);
        if selected_host.is_none() && observation["complete"].as_bool() == Some(true) {
            selected_host = Some(relative.to_string());
        }
        host_candidates.push(observation);
    }

    let exact_missing = if env_set {
        Vec::<String>::new()
    } else {
        vec![format!("env:{CEF_BUNDLE_ENV}")]
    };

    let host_found = selected_host.is_some();
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
        "exact_missing": exact_missing,
        "exact_missing_for_cap_day": if env_set {
            json!([])
        } else {
            json!([format!("env:{CEF_BUNDLE_ENV}")])
        },
        "host_candidates": host_candidates,
        "private_host_bundle_found": host_found,
        "selected_private_host_relative": selected_host,
        "selected_reason": if host_found {
            "complete private evidence CEF runtime (archive.json + libcef.dll + locales); not invented; Cap day still requires env set"
        } else {
            "no complete private host CEF tree observed"
        },
        "path_release_surfaces": PATH_RELEASE_SURFACES,
        "status": if env_set {
            "env_bound"
        } else if host_found {
            "env_absent_private_host_present"
        } else {
            "env_absent_host_absent"
        },
        "note": "Cap=0 probe only; Unlock A g4 component ≠ BrowserReady; Cap day refuse-closed until HUMANYMOUS_CEF_PILOT_BUNDLE is set to a complete bundle; never invent CEF binary or BrowserReady",
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
        // Ensure env is unset for this assertion path by only checking host fields
        // when env happens to be set in the ambient process.
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
                .any(|item| item.as_str() == Some(&format!("env:{CEF_BUNDLE_ENV}")))
                || report["cef_bundle_env_present"].as_bool() == Some(true)
        );
        let _ = fs::remove_dir_all(&root);
    }
}

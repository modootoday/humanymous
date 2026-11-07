//! The dedicated controller is blocked pending a reviewed shared lifecycle API.
//! Procfs observations are diagnostics, never authorization to signal a group.
use std::{fs, path::Path, process::Command};

pub(super) fn actual(
    _command: &mut Command,
    _bundle: &Path,
) -> Result<(std::process::ExitStatus, Vec<u8>), String> {
    Err("NOT_ACCEPTED: actual controller execution requires reviewed shared lifecycle".into())
}

pub(super) fn cleanup(bundle: &Path) -> Result<(), String> {
    let marker = format!("HUMANYMOUS_PYTHON_MODULE_DIR={}/module", bundle.display());
    for entry in fs::read_dir("/proc").map_err(|_| "process inventory unavailable")? {
        let entry = entry.map_err(|_| "process inventory entry unavailable")?;
        if entry.file_name().to_string_lossy().parse::<u32>().is_err() {
            continue;
        }
        if let Ok(environment) = fs::read(entry.path().join("environ"))
            && environment
                .split(|byte| *byte == 0)
                .any(|value| value == marker.as_bytes())
        {
            return Err(
                "cleanup FAIL: task process observed; safe lifetime proof unavailable".into(),
            );
        }
    }
    // Observation only: absence of a marker is not an atomic cleanup proof.
    Err("cleanup NOT_ACCEPTED: shared atomic owned-lifetime proof unavailable".into())
}

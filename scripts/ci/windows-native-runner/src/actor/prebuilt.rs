//! Pure prebuilt contract only. No OS launch, file verification or cleanup proof.
use humanymous_test_support::json::decode_strict_json;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, io};

const INPUT_LIMIT: usize = 256 * 1024;
const APP_NAMES: [&str; 6] = [
    "Humanymous.HostPilot.exe",
    "Humanymous.HostPilot.dll",
    "Humanymous.HostPilot.deps.json",
    "Humanymous.HostPilot.runtimeconfig.json",
    "Humanymous.Native.dll",
    "humanymous_dotnet_adapter_export.dll",
];
const SELECTOR: &str = "actual_kestrel_engine_audit_body_gc_and_shutdown";

fn invalid() -> io::Error {
    io::Error::other("invalid prebuilt contract")
}
fn keys(value: &Value, expected: &[&str]) -> io::Result<()> {
    let object = value.as_object().ok_or_else(invalid)?;
    if object.len() != expected.len() || expected.iter().any(|name| !object.contains_key(*name)) {
        return Err(invalid());
    }
    Ok(())
}
fn text(value: &Value) -> io::Result<&str> {
    value.as_str().ok_or_else(invalid)
}
fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn component(value: &str) -> bool {
    let stem = value.split('.').next().unwrap_or("").to_ascii_uppercase();
    !value.is_empty()
        && value.len() <= 128
        && value != "."
        && value != ".."
        && !value.ends_with('.')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        && !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}
fn relative(value: &str) -> bool {
    value.len() <= 512 && value.split('/').all(component)
}
fn absolute(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() > 3
        && bytes.len() <= 1024
        && bytes[0].is_ascii_uppercase()
        && bytes[1..3] == *b":/"
        && relative(&value[3..])
}

/// Parsed metadata, never native file verification or original build provenance.
struct FileEntry {
    path: String,
    bytes: u64,
    sha256: String,
}
struct Manifest {
    files: Vec<FileEntry>,
    total_bytes: u64,
}
#[derive(Clone, Copy)]
enum ClosureKind {
    App,
    Runtime,
}

/// The expected digest must originate in independently bound build/observation
/// evidence. Comparing a manifest to its own advertised digest provides no trust.
fn parse_manifest(bytes: &[u8], expected_digest: &str, kind: ClosureKind) -> io::Result<Manifest> {
    if bytes.len() > INPUT_LIMIT
        || !hex(expected_digest, 64)
        || format!("{:x}", Sha256::digest(bytes)) != expected_digest
    {
        return Err(invalid());
    }
    let value = decode_strict_json(bytes, INPUT_LIMIT).map_err(|_| invalid())?;
    keys(&value, &["schema", "files"])?;
    if value["schema"] != "humanymous.dotnet-prebuilt-files/1" {
        return Err(invalid());
    }
    let rows = value["files"].as_array().ok_or_else(invalid)?;
    let (count, total_limit, file_limit) = match kind {
        ClosureKind::App => (6, 512 * 1024 * 1024_u64, 256 * 1024 * 1024_u64),
        ClosureKind::Runtime => (336, 192 * 1024 * 1024_u64, 64 * 1024 * 1024_u64),
    };
    if rows.len() != count || rows.len() > 512 {
        return Err(invalid());
    }
    let mut names = BTreeSet::new();
    let mut files = Vec::new();
    let mut total_bytes = 0_u64;
    for row in rows {
        keys(row, &["path", "bytes", "sha256"])?;
        let path = text(&row["path"])?;
        let sha256 = text(&row["sha256"])?;
        let length = row["bytes"].as_u64().ok_or_else(invalid)?;
        if !relative(path)
            || !hex(sha256, 64)
            || length == 0
            || length > file_limit
            || !names.insert(path.to_ascii_lowercase())
        {
            return Err(invalid());
        }
        match kind {
            ClosureKind::App if !APP_NAMES.contains(&path) => return Err(invalid()),
            ClosureKind::Runtime
                if !(path == "host/fxr/10.0.14/hostfxr.dll"
                    || path.starts_with("shared/Microsoft.NETCore.App/10.0.14/")
                    || path.starts_with("shared/Microsoft.AspNetCore.App/10.0.14/")) =>
            {
                return Err(invalid());
            }
            _ => {}
        }
        total_bytes = total_bytes.checked_add(length).ok_or_else(invalid)?;
        if total_bytes > total_limit {
            return Err(invalid());
        }
        files.push(FileEntry {
            path: path.into(),
            bytes: length,
            sha256: sha256.into(),
        });
    }
    if matches!(kind, ClosureKind::Runtime)
        && (!names.contains("host/fxr/10.0.14/hostfxr.dll")
            || !names
                .iter()
                .any(|name| name.starts_with("shared/microsoft.netcore.app/10.0.14/"))
            || !names
                .iter()
                .any(|name| name.starts_with("shared/microsoft.aspnetcore.app/10.0.14/")))
    {
        return Err(invalid());
    }
    Ok(Manifest { files, total_bytes })
}

const OBSERVATION_SHA256: &str = "d9bd271d558f07ad6b1438a27d6c8678b24915e26d03b5ed8cc5e3368529944d";
const APP_ROLES: [&str; 6] = [
    "apphost",
    "assembly",
    "deps",
    "runtimeconfig",
    "binding",
    "facade",
];

struct InputPins<'a> {
    app_manifest: &'a str,
    runtime_manifest: &'a str,
    harness: &'a str,
    provenance: &'a str,
}
struct ParsedInput {
    nonce: String,
    harness: FileEntry,
    app_root: String,
    app: Manifest,
    runtime_root: String,
    runtime: Manifest,
    runtime_manifest: FileEntry,
    provenance: Provenance,
}
fn artifact(value: &Value, limit: u64) -> io::Result<FileEntry> {
    keys(value, &["path", "bytes", "sha256"])?;
    let path = text(&value["path"])?;
    let bytes = value["bytes"].as_u64().ok_or_else(invalid)?;
    let sha256 = text(&value["sha256"])?;
    if !absolute(path) || bytes == 0 || bytes > limit || !hex(sha256, 64) {
        return Err(invalid());
    }
    Ok(FileEntry {
        path: path.into(),
        bytes,
        sha256: sha256.into(),
    })
}
/// New normalized files-only bytes, distinct from the historical observation.
/// Normalization sorts exact relative paths, uses compact serde JSON plus LF.
fn normalized_manifest(rows: &Value, app: bool) -> io::Result<Vec<u8>> {
    let rows = rows.as_array().ok_or_else(invalid)?;
    if rows.len() > 512 {
        return Err(invalid());
    }
    let mut normalized = Vec::new();
    for row in rows {
        if app {
            keys(row, &["role", "relativepath", "bytes", "sha256"])?;
        } else {
            keys(row, &["relativepath", "bytes", "sha256"])?;
        }
        let path = text(&row["relativepath"])?;
        if app {
            let index = APP_NAMES
                .iter()
                .position(|name| *name == path)
                .ok_or_else(invalid)?;
            if row["role"] != APP_ROLES[index] {
                return Err(invalid());
            }
        }
        normalized
            .push(serde_json::json!({"path":path,"bytes":row["bytes"],"sha256":row["sha256"]}));
    }
    normalized.sort_by(|first, second| first["path"].as_str().cmp(&second["path"].as_str()));
    let mut bytes = serde_json::to_vec(
        &serde_json::json!({"schema":"humanymous.dotnet-prebuilt-files/1","files":normalized}),
    )
    .map_err(|_| invalid())?;
    bytes.push(b'\n');
    Ok(bytes)
}
fn parse_input(bytes: &[u8], pins: &InputPins<'_>) -> io::Result<ParsedInput> {
    let value = decode_strict_json(bytes, INPUT_LIMIT).map_err(|_| invalid())?;
    keys(
        &value,
        &[
            "schema",
            "profile",
            "nonce",
            "harness",
            "app",
            "runtime",
            "provenance",
        ],
    )?;
    if value["schema"] != "humanymous.dotnet-prebuilt-actor-input/1"
        || value["profile"] != "standard-user-dotnet-prebuilt"
    {
        return Err(invalid());
    }
    let nonce = text(&value["nonce"])?;
    if !hex(nonce, 32) || nonce.bytes().all(|byte| byte == b'0') {
        return Err(invalid());
    }
    keys(&value["app"], &["root", "files"])?;
    keys(
        &value["runtime"],
        &["root", "manifest", "files", "parent_observation_sha256"],
    )?;
    let app_root = text(&value["app"]["root"])?;
    let runtime_root = text(&value["runtime"]["root"])?;
    if !absolute(app_root)
        || !absolute(runtime_root)
        || app_root.eq_ignore_ascii_case(runtime_root)
        || app_root
            .to_ascii_lowercase()
            .starts_with(&(runtime_root.to_ascii_lowercase() + "/"))
        || runtime_root
            .to_ascii_lowercase()
            .starts_with(&(app_root.to_ascii_lowercase() + "/"))
    {
        return Err(invalid());
    }
    if value["runtime"]["parent_observation_sha256"] != OBSERVATION_SHA256 {
        return Err(invalid());
    }
    let harness = artifact(&value["harness"], 256 * 1024 * 1024)?;
    if harness.sha256 != pins.harness {
        return Err(invalid());
    }
    let app = parse_manifest(
        &normalized_manifest(&value["app"]["files"], true)?,
        pins.app_manifest,
        ClosureKind::App,
    )?;
    let runtime_bytes = normalized_manifest(&value["runtime"]["files"], false)?;
    let runtime = parse_manifest(&runtime_bytes, pins.runtime_manifest, ClosureKind::Runtime)?;
    let runtime_manifest = artifact(&value["runtime"]["manifest"], INPUT_LIMIT as u64)?;
    if runtime_manifest.bytes != runtime_bytes.len() as u64
        || runtime_manifest.sha256 != pins.runtime_manifest
    {
        return Err(invalid());
    }
    let provenance = Provenance::parse(&value["provenance"])?;
    if !hex(pins.provenance, 64)
        || format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&value["provenance"]).map_err(|_| invalid())?)
        ) != pins.provenance
    {
        return Err(invalid());
    }
    Ok(ParsedInput {
        nonce: nonce.into(),
        harness,
        app_root: app_root.into(),
        app,
        runtime_root: runtime_root.into(),
        runtime,
        runtime_manifest,
        provenance,
    })
}

/// Named source lineages; metadata does not prove original build custody.
struct Provenance {
    managed: Value,
    native: Value,
    harness: Value,
}
impl Provenance {
    fn parse(value: &Value) -> io::Result<Self> {
        keys(value, &["managed", "native", "harness"])?;
        for role in ["managed", "native", "harness"] {
            let row = &value[role];
            let fields = if role == "managed" {
                ["source_sha", "required_source_sha", "receipt_sha256"]
            } else {
                ["source_sha", "cargo_lock_sha256", "receipt_sha256"]
            };
            keys(row, &fields)?;
            for name in fields {
                if !hex(
                    text(&row[name])?,
                    if name.ends_with("source_sha") { 40 } else { 64 },
                ) {
                    return Err(invalid());
                }
            }
        }
        Ok(Self {
            managed: value["managed"].clone(),
            native: value["native"].clone(),
            harness: value["harness"].clone(),
        })
    }
}

/// One parent-created budget mapping; consumers borrow it, never renew it.
struct Budget {
    run_until: Option<std::time::Instant>,
    cleanup_until: std::time::Instant,
}
/// Distinct trusted-parent context has no apphost, harness or workload launch API.
struct CleanupOnlyContext {
    budget: Budget,
}
impl CleanupOnlyContext {
    fn map(value: &Value, wall_ms: u64, received: std::time::Instant) -> io::Result<Self> {
        Ok(Self {
            budget: Budget::map_phase(value, wall_ms, received, true)?,
        })
    }
}
impl Budget {
    fn map(value: &Value, wall_ms: u64, received: std::time::Instant) -> io::Result<Self> {
        Self::map_phase(value, wall_ms, received, false)
    }
    fn map_phase(
        value: &Value,
        wall_ms: u64,
        received: std::time::Instant,
        cleanup_only: bool,
    ) -> io::Result<Self> {
        keys(
            value,
            &[
                "issued_utc_ms",
                "run_expires_utc_ms",
                "cleanup_expires_utc_ms",
                "parent_elapsed_ms_at_dispatch",
            ],
        )?;
        let number = |key| value[key].as_u64().ok_or_else(invalid);
        let issued = number("issued_utc_ms")?;
        let run = number("run_expires_utc_ms")?;
        let cleanup = number("cleanup_expires_utc_ms")?;
        let elapsed = number("parent_elapsed_ms_at_dispatch")?;
        if run.checked_sub(issued) != Some(180000)
            || cleanup.checked_sub(run) != Some(60000)
            || elapsed >= 240000
            || (!cleanup_only && elapsed >= 180000)
            || wall_ms
                .checked_sub(issued)
                .is_none_or(|wall_elapsed| wall_elapsed < elapsed)
        {
            return Err(invalid());
        }
        let cleanup_remaining = cleanup
            .checked_sub(wall_ms)
            .ok_or_else(invalid)?
            .min(240000 - elapsed);
        if cleanup_remaining == 0 {
            return Err(invalid());
        }
        let run_until = if cleanup_only {
            None
        } else {
            let remaining = run
                .checked_sub(wall_ms)
                .ok_or_else(invalid)?
                .min(180000 - elapsed);
            if remaining == 0 {
                return Err(invalid());
            }
            Some(
                received
                    .checked_add(std::time::Duration::from_millis(remaining))
                    .ok_or_else(invalid)?,
            )
        };
        Ok(Self {
            run_until,
            cleanup_until: received
                .checked_add(std::time::Duration::from_millis(cleanup_remaining))
                .ok_or_else(invalid)?,
        })
    }
}

/// Syntax-only parent-generated context. It is not the caller input DTO and
/// cannot establish protected roots/token/handles. No native adapter uses it.
struct LaunchContext {
    apphost: String,
    app_root: String,
    system_root: String,
    runtime_root: String,
    private_work: String,
    audit_root: String,
    handoff_root: String,
    actor_sid: String,
    provenance: Provenance,
    budget: Budget,
}
impl LaunchContext {
    fn parse(bytes: &[u8], wall_ms: u64, received: std::time::Instant) -> io::Result<Self> {
        let value = decode_strict_json(bytes, INPUT_LIMIT).map_err(|_| invalid())?;
        keys(
            &value,
            &[
                "schema",
                "apphost",
                "app_root",
                "system_root",
                "runtime_root",
                "private_work",
                "audit_root",
                "handoff_root",
                "actor_sid",
                "provenance",
                "budget",
            ],
        )?;
        if value["schema"] != "humanymous.dotnet-prebuilt-launch/1" {
            return Err(invalid());
        }
        let field = |key| -> io::Result<String> {
            let result = text(&value[key])?;
            if !absolute(result) {
                return Err(invalid());
            }
            Ok(result.into())
        };
        let actor_sid = text(&value["actor_sid"])?;
        let parts: Vec<_> = actor_sid.split('-').collect();
        if parts.len() < 4
            || parts.len() > 18
            || parts[..2] != ["S", "1"]
            || parts[2..].iter().any(|part| {
                part.is_empty()
                    || part.len() > 10
                    || !part.bytes().all(|byte| byte.is_ascii_digit())
                    || part.parse::<u32>().is_err()
                    || (part.len() > 1 && part.starts_with('0'))
            })
        {
            return Err(invalid());
        }
        let result = Self {
            apphost: field("apphost")?,
            app_root: field("app_root")?,
            system_root: field("system_root")?,
            runtime_root: field("runtime_root")?,
            private_work: field("private_work")?,
            audit_root: field("audit_root")?,
            handoff_root: field("handoff_root")?,
            actor_sid: actor_sid.into(),
            provenance: Provenance::parse(&value["provenance"])?,
            budget: Budget::map(&value["budget"], wall_ms, received)?,
        };
        if result.apphost != format!("{}/Humanymous.HostPilot.exe", result.app_root) {
            return Err(invalid());
        }
        let roots = [
            &result.app_root,
            &result.system_root,
            &result.runtime_root,
            &result.private_work,
            &result.handoff_root,
        ];
        for (index, first) in roots.iter().enumerate() {
            for second in roots.iter().skip(index + 1) {
                let first = first.to_ascii_lowercase();
                let second = second.to_ascii_lowercase();
                if first == second
                    || first.starts_with(&(second.clone() + "/"))
                    || second.starts_with(&(first + "/"))
                {
                    return Err(invalid());
                }
            }
        }
        if !result
            .audit_root
            .starts_with(&(result.private_work.clone() + "/"))
        {
            return Err(invalid());
        }
        Ok(result)
    }
    fn environment(&self) -> Vec<(&'static str, String)> {
        vec![
            ("SystemRoot", self.system_root.clone()),
            ("WINDIR", self.system_root.clone()),
            ("PATH", format!("{}/System32", self.system_root)),
            ("DOTNET_ROOT", self.runtime_root.clone()),
            ("DOTNET_ROOT_X64", self.runtime_root.clone()),
            ("DOTNET_MULTILEVEL_LOOKUP", "0".into()),
            ("DOTNET_EnableDiagnostics", "0".into()),
            ("DOTNET_ROLL_FORWARD", "LatestPatch".into()),
            ("TEMP", format!("{}/temp", self.private_work)),
            ("TMP", format!("{}/temp", self.private_work)),
            ("USERPROFILE", format!("{}/profile", self.private_work)),
        ]
    }
    fn harness_arguments(&self) -> [&'static str; 5] {
        [
            "--ignored",
            "--nocapture",
            "--exact",
            SELECTOR,
            "--test-threads=1",
        ]
    }
}

/// Symbolic state ordering only; observations cannot authenticate native facts.
#[derive(Default)]
struct CleanupOrder {
    started: bool,
    main_empty: bool,
    cleanup_started: bool,
    cleanup_empty: bool,
    private_removed: bool,
    profile_before_absent: bool,
    profile_after_absent: bool,
    account_deleted: bool,
    scratch_deleted: bool,
    readers_joined: bool,
    outcome_bound: bool,
}
impl CleanupOrder {
    fn start(&mut self, profile_absent: bool) -> io::Result<()> {
        if self.started || !profile_absent {
            return Err(invalid());
        }
        self.started = true;
        self.profile_before_absent = true;
        Ok(())
    }
    fn main_empty(&mut self) -> io::Result<()> {
        if !self.started || self.main_empty {
            return Err(invalid());
        }
        self.main_empty = true;
        Ok(())
    }
    fn start_recovery(&mut self) -> io::Result<()> {
        if !self.main_empty || self.cleanup_started || self.account_deleted {
            return Err(invalid());
        }
        self.cleanup_started = true;
        self.private_removed = false;
        self.profile_after_absent = false;
        Ok(())
    }
    fn private_removed(&mut self) -> io::Result<()> {
        if !self.started || self.account_deleted {
            return Err(invalid());
        }
        self.private_removed = true;
        Ok(())
    }
    fn cleanup_empty(&mut self) -> io::Result<()> {
        if !self.cleanup_started || !self.private_removed {
            return Err(invalid());
        }
        self.cleanup_empty = true;
        Ok(())
    }
    fn profile_absent_after(&mut self) -> io::Result<()> {
        if !self.main_empty || (self.cleanup_started && !self.cleanup_empty) {
            return Err(invalid());
        }
        self.profile_after_absent = true;
        Ok(())
    }
    fn delete_account(&mut self) -> io::Result<()> {
        if !self.main_empty
            || !self.private_removed
            || !self.profile_after_absent
            || (self.cleanup_started && !self.cleanup_empty)
            || self.account_deleted
        {
            return Err(invalid());
        }
        self.account_deleted = true;
        Ok(())
    }
    fn delete_scratch(&mut self) -> io::Result<()> {
        if !self.account_deleted || self.scratch_deleted {
            return Err(invalid());
        }
        self.scratch_deleted = true;
        Ok(())
    }
    fn clean_pass(&self, execution_passed: bool) -> bool {
        execution_passed
            && self.started
            && self.main_empty
            && self.private_removed
            && self.profile_before_absent
            && self.profile_after_absent
            && self.account_deleted
            && self.scratch_deleted
            && self.readers_joined
            && self.outcome_bound
            && (!self.cleanup_started || self.cleanup_empty)
    }
}

#[cfg(test)]
#[path = "prebuilt/tests.rs"]
mod tests;

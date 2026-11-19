//! Pure `standard-user-dotnet-prebuilt` contract (SoT dotnet-protected-host-
//! launch N4, owner decision D5(a), issue 75 comment 6998447599). It parses the
//! closed `/2` input, fixes the two selectors and their test inventories, binds
//! the staged runtime closure to constants derived from the SHA-512-pinned
//! win-x64 SDK, and orders cleanup. It opens no file and starts no process;
//! the native parent and actor child live in `actor/prebuilt_run.rs`.
use humanymous_platform::testing::{
    json::decode_strict_json,
    parent_context::{UnverifiedProvenance, host::UnverifiedHostContext},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, io};

pub(crate) const INPUT_LIMIT: usize = 256 * 1024;
pub(crate) const INPUT_SCHEMA: &str = "humanymous.dotnet-prebuilt-actor-input/2";
pub(crate) const PROFILE: &str = "standard-user-dotnet-prebuilt";
const FILES_SCHEMA: &str = "humanymous.dotnet-prebuilt-files/1";
/// App closure: the published FDD app plus both native exports it loads.
pub(crate) const APP_FILES: [(&str, &str); 7] = [
    ("apphost", "Humanymous.HostPilot.exe"),
    ("assembly", "Humanymous.HostPilot.dll"),
    ("deps", "Humanymous.HostPilot.deps.json"),
    ("runtimeconfig", "Humanymous.HostPilot.runtimeconfig.json"),
    ("binding", "Humanymous.Native.dll"),
    ("facade", "humanymous_dotnet_adapter_export.dll"),
    ("generic", "humanymous_native_abi_export.dll"),
];
/// Runtime closure of the pinned SDK 10.0.403 (runtime 10.0.14): hostfxr and
/// the two shared frameworks, 336 files and 110,309,027 bytes. This is the
/// SHA-256 of their normalized manifest, computed 2026-11-19 from the
/// SHA-512-verified `dotnet-sdk-10.0.403-win-x64.zip`. The same count and
/// total as the earlier read-only observation (plan 20261107115350).
pub(crate) const RUNTIME_MANIFEST_SHA256: &str =
    "eaf43db54d20608d79deeadd185a63c2b24099c79db8a6fc9032b4cbd06f6680";
pub(crate) const RUNTIME_FILES: usize = 336;
pub(crate) const RUNTIME_BYTES: u64 = 110_309_027;
/// The pinned archive's `dotnet.exe` muxer, which the generic case runs.
pub(crate) const MUXER_SHA256: &str =
    "1e881ec1786c31ba712d052258b4ad4202ac155df5965794030037878abb822f";
pub(crate) const MUXER_BYTES: u64 = 167_208;
pub(crate) const RUNTIME_PREFIXES: [&str; 3] = [
    "host/fxr/10.0.14/",
    "shared/Microsoft.NETCore.App/10.0.14/",
    "shared/Microsoft.AspNetCore.App/10.0.14/",
];

/// Kestrel case: the only one that consumes the creator-channel envelope.
pub(crate) const KESTREL: &str = "actual_kestrel_engine_audit_body_gc_and_shutdown";
pub(crate) const GENERIC: &str = "generic::actual_generic_abi_only_process";
pub(crate) const PACKAGE: &str = "sdk_package_and_real_host_matrix";

/// The two fixed selectors (N4 amends the plan's single `SELECTOR`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Selector {
    /// Row 24: the Kestrel case, then the generic ABI-only case.
    Hosted,
    /// Rows 72/73: the package-matrix controller.
    Package,
}
impl Selector {
    pub(crate) fn parse(value: &str) -> io::Result<Self> {
        match value {
            "hosted" => Ok(Self::Hosted),
            "package" => Ok(Self::Package),
            _ => Err(invalid()),
        }
    }
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Hosted => "hosted",
            Self::Package => "package",
        }
    }
    pub(crate) fn tests(self) -> &'static [&'static str] {
        match self {
            Self::Hosted => &[KESTREL, GENERIC],
            Self::Package => &[PACKAGE],
        }
    }
}
/// Fixed harness arguments for one inventoried test, the same five the Linux
/// provider passes; nothing else is accepted.
pub(crate) fn harness_arguments(selector: Selector, test: &str) -> io::Result<[&'static str; 5]> {
    let name = selector
        .tests()
        .iter()
        .find(|name| **name == test)
        .ok_or_else(invalid)?;
    Ok([
        "--exact",
        name,
        "--ignored",
        "--test-threads=1",
        "--nocapture",
    ])
}

pub(crate) fn invalid() -> io::Error {
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
pub(crate) fn hex(value: &str, length: usize) -> bool {
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
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-+".contains(&byte))
        && !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}
pub(crate) fn relative(value: &str) -> bool {
    value.len() <= 512 && value.split('/').all(component)
}
/// `X:/...` forward-slash absolute local path text.
pub(crate) fn absolute(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() > 3
        && bytes.len() <= 1024
        && bytes[0].is_ascii_uppercase()
        && bytes[1..3] == *b":/"
        && relative(&value[3..])
}
/// `child` is strictly below `root` (case-insensitive path text).
pub(crate) fn below(child: &str, root: &str) -> bool {
    child
        .to_ascii_lowercase()
        .starts_with(&(root.to_ascii_lowercase() + "/"))
}
fn disjoint(first: &str, second: &str) -> bool {
    !first.eq_ignore_ascii_case(second) && !below(first, second) && !below(second, first)
}

/// One artifact or manifest row: relative or absolute path, length, SHA-256.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileEntry {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
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

/// Normalized files-only manifest bytes: rows sorted by exact relative path,
/// compact serde JSON with sorted keys, then LF.
pub(crate) fn normalized_manifest(rows: &[FileEntry]) -> io::Result<Vec<u8>> {
    if rows.len() > 512 {
        return Err(invalid());
    }
    let mut sorted: Vec<_> = rows.iter().collect();
    sorted.sort_by(|first, second| first.path.cmp(&second.path));
    let files: Vec<_> = sorted
        .into_iter()
        .map(|row| serde_json::json!({"path":row.path,"bytes":row.bytes,"sha256":row.sha256}))
        .collect();
    let mut bytes = serde_json::to_vec(&serde_json::json!({"schema":FILES_SCHEMA,"files":files}))
        .map_err(|_| invalid())?;
    bytes.push(b'\n');
    Ok(bytes)
}
pub(crate) fn manifest_sha256(rows: &[FileEntry]) -> io::Result<String> {
    Ok(format!("{:x}", Sha256::digest(normalized_manifest(rows)?)))
}

/// Closed runtime closure rows the parent observed while staging. They must be
/// exactly the pinned SDK's closure, by count, total and normalized digest.
pub(crate) fn runtime_closure_ok(rows: &[FileEntry]) -> io::Result<()> {
    let mut names = BTreeSet::new();
    let mut total = 0_u64;
    for row in rows {
        if !relative(&row.path)
            || !hex(&row.sha256, 64)
            || row.bytes == 0
            || row.bytes > 64 * 1024 * 1024
            || !RUNTIME_PREFIXES
                .iter()
                .any(|prefix| row.path.starts_with(prefix))
            || !names.insert(row.path.to_ascii_lowercase())
        {
            return Err(invalid());
        }
        total = total.checked_add(row.bytes).ok_or_else(invalid)?;
    }
    if rows.len() != RUNTIME_FILES
        || total != RUNTIME_BYTES
        || !names.contains("host/fxr/10.0.14/hostfxr.dll")
        || manifest_sha256(rows)? != RUNTIME_MANIFEST_SHA256
    {
        return Err(invalid());
    }
    Ok(())
}

/// Parsed `/2` input. Every path is admin-side source text the parent copies
/// from; no row grants the actor anything.
pub(crate) struct ParsedInput {
    pub selector: Selector,
    pub source_sha: String,
    pub manifest_sha256: String,
    pub harness: FileEntry,
    pub managed_source: FileEntry,
    pub app_root: String,
    pub app: Vec<FileEntry>,
    pub runtime_root: String,
    pub provenance: UnverifiedProvenance,
}
pub(crate) fn parse_input(bytes: &[u8]) -> io::Result<ParsedInput> {
    let value = decode_strict_json(bytes, INPUT_LIMIT).map_err(|_| invalid())?;
    keys(
        &value,
        &[
            "schema",
            "profile",
            "selector",
            "source",
            "harness",
            "managed_source",
            "app",
            "runtime",
            "provenance",
        ],
    )?;
    if value["schema"] != INPUT_SCHEMA || value["profile"] != PROFILE {
        return Err(invalid());
    }
    let selector = Selector::parse(text(&value["selector"])?)?;
    keys(
        &value["source"],
        &["sha", "manifest_sha256", "cargo_lock_sha256"],
    )?;
    let source_sha = text(&value["source"]["sha"])?;
    let manifest_sha256 = text(&value["source"]["manifest_sha256"])?;
    let cargo_lock_sha256 = text(&value["source"]["cargo_lock_sha256"])?;
    if !hex(source_sha, 40) || !hex(manifest_sha256, 64) || !hex(cargo_lock_sha256, 64) {
        return Err(invalid());
    }
    let harness = artifact(&value["harness"], 256 * 1024 * 1024)?;
    let managed_source = artifact(&value["managed_source"], 1024 * 1024)?;
    keys(&value["app"], &["root", "files"])?;
    keys(&value["runtime"], &["root"])?;
    let app_root = text(&value["app"]["root"])?;
    let runtime_root = text(&value["runtime"]["root"])?;
    if !absolute(app_root) || !absolute(runtime_root) || !disjoint(app_root, runtime_root) {
        return Err(invalid());
    }
    let rows = value["app"]["files"].as_array().ok_or_else(invalid)?;
    if rows.len() != APP_FILES.len() {
        return Err(invalid());
    }
    let mut app = Vec::new();
    for (row, (role, name)) in rows.iter().zip(APP_FILES) {
        keys(row, &["role", "path", "bytes", "sha256"])?;
        let bytes = row["bytes"].as_u64().ok_or_else(invalid)?;
        let sha256 = text(&row["sha256"])?;
        if row["role"] != role
            || row["path"] != name
            || bytes == 0
            || bytes > 256 * 1024 * 1024
            || !hex(sha256, 64)
        {
            return Err(invalid());
        }
        app.push(FileEntry {
            path: name.into(),
            bytes,
            sha256: sha256.into(),
        });
    }
    let provenance: UnverifiedProvenance =
        serde_json::from_value(value["provenance"].clone()).map_err(|_| invalid())?;
    provenance.validate_syntax().map_err(|_| invalid())?;
    // The provenance names exactly this source and these artifacts.
    if provenance.managed.source_sha != source_sha
        || provenance.managed.required_source_sha != source_sha
        || provenance.managed.receipt_sha256 != app[1].sha256
        || provenance.native.source_sha != source_sha
        || provenance.native.cargo_lock_sha256 != cargo_lock_sha256
        || provenance.native.receipt_sha256 != app[5].sha256
        || provenance.harness.source_sha != source_sha
        || provenance.harness.cargo_lock_sha256 != cargo_lock_sha256
        || provenance.harness.receipt_sha256 != harness.sha256
    {
        return Err(invalid());
    }
    Ok(ParsedInput {
        selector,
        source_sha: source_sha.into(),
        manifest_sha256: manifest_sha256.into(),
        harness,
        managed_source,
        app_root: app_root.into(),
        app,
        runtime_root: runtime_root.into(),
        provenance,
    })
}

/// Actor-side path policy over a decoded, still-unverified `/2` context.
pub(crate) fn host_paths_ok(context: &UnverifiedHostContext) -> io::Result<()> {
    let system = context.system_root.as_deref().ok_or_else(invalid)?;
    if [
        context.apphost.as_str(),
        &context.app_root,
        &context.runtime_root,
        system,
        &context.private_work,
        &context.audit_root,
        &context.handoff_root,
    ]
    .into_iter()
    .any(|value| !absolute(value))
        || context.apphost != format!("{}/Humanymous.HostPilot.exe", context.app_root)
        || context.audit_root != format!("{}/audit", context.private_work)
    {
        return Err(invalid());
    }
    let roots = [
        &context.app_root,
        &context.runtime_root,
        &context.private_work,
        &context.handoff_root,
    ];
    for (index, first) in roots.iter().enumerate() {
        for second in roots.iter().skip(index + 1) {
            if !disjoint(first, second) {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

/// The actor child's one result line, exactly as the parent accepts it.
pub(crate) const CHILD_SCHEMA: &str = "humanymous.dotnet-prebuilt-child/1";
pub(crate) fn child_line_ok(line: &Value, selector: Selector) -> bool {
    let Some(tests) = line["tests"].as_array() else {
        return false;
    };
    line["schema"] == CHILD_SCHEMA
        && line["selector"] == selector.label()
        && line["producer_bound"] == true
        && line["harness_verified"] == true
        && line["private_created"] == true
        && line["private_removed"] == true
        && line["handoff_written"] == true
        && tests.len() == selector.tests().len()
        && tests.iter().zip(selector.tests()).all(|(row, name)| {
            row["name"] == *name
                && row["exit_code"] == 0
                && row["timed_out"] == false
                && row["readers_joined"] == true
                && row["creator_channel"] == (*name == KESTREL || *name == PACKAGE)
        })
}

/// Exact single-test footer and case line for one `--exact` test alone.
pub(crate) fn single_test_passed(output: &str, name: &str) -> bool {
    let footers: Vec<_> = output
        .lines()
        .filter(|line| line.starts_with("test result:"))
        .collect();
    footers.len() == 1
        && footers[0].starts_with("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; ")
        && output
            .lines()
            .filter(|line| line.starts_with(&format!("test {name} ... ")))
            .count()
            == 1
}

/// Symbolic state ordering only; observations cannot authenticate native facts.
#[derive(Default)]
pub(crate) struct CleanupOrder {
    started: bool,
    main_empty: bool,
    cleanup_started: bool,
    cleanup_empty: bool,
    private_removed: bool,
    profile_before_absent: bool,
    profile_after_absent: bool,
    account_deleted: bool,
    scratch_deleted: bool,
    pub readers_joined: bool,
    pub outcome_bound: bool,
}
impl CleanupOrder {
    /// Whether the actor launch sequence began; before it, no ordering fact applies.
    pub(crate) fn started(&self) -> bool {
        self.started
    }
    pub(crate) fn start(&mut self, profile_absent: bool) -> io::Result<()> {
        if self.started || !profile_absent {
            return Err(invalid());
        }
        self.started = true;
        self.profile_before_absent = true;
        Ok(())
    }
    pub(crate) fn main_empty(&mut self) -> io::Result<()> {
        if !self.started || self.main_empty {
            return Err(invalid());
        }
        self.main_empty = true;
        Ok(())
    }
    #[cfg(test)]
    fn start_recovery(&mut self) -> io::Result<()> {
        if !self.main_empty || self.cleanup_started || self.account_deleted {
            return Err(invalid());
        }
        self.cleanup_started = true;
        self.private_removed = false;
        self.profile_after_absent = false;
        Ok(())
    }
    pub(crate) fn private_removed(&mut self) -> io::Result<()> {
        if !self.started || self.account_deleted {
            return Err(invalid());
        }
        self.private_removed = true;
        Ok(())
    }
    #[cfg(test)]
    fn cleanup_empty(&mut self) -> io::Result<()> {
        if !self.cleanup_started || !self.private_removed {
            return Err(invalid());
        }
        self.cleanup_empty = true;
        Ok(())
    }
    pub(crate) fn profile_absent_after(&mut self) -> io::Result<()> {
        if !self.main_empty || (self.cleanup_started && !self.cleanup_empty) {
            return Err(invalid());
        }
        self.profile_after_absent = true;
        Ok(())
    }
    pub(crate) fn delete_account(&mut self) -> io::Result<()> {
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
    pub(crate) fn delete_scratch(&mut self) -> io::Result<()> {
        if !self.account_deleted || self.scratch_deleted {
            return Err(invalid());
        }
        self.scratch_deleted = true;
        Ok(())
    }
    pub(crate) fn clean_pass(&self, execution_passed: bool) -> bool {
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

//! Native `dotnet-prebuilt` runner mode (SoT dotnet-protected-host-launch N4,
//! owner decision D5(a), issue 75 comment 6998447599). The hops are those of
//! SoT 3.1.2: the elevated hosted parent creates this runner as the NoProfile
//! standard-user actor child (N1/N5) and writes one envelope into its stdin;
//! the actor child binds the published context to that envelope (N3) and only
//! then creates the harness, writing the same envelope into the harness stdin.
//! Every pure rule is in `prebuilt.rs`.
use super::{
    CleanupTarget, StartFailure, account, acl, actor_for_nonce, profile_absent,
    start_prebuilt_actor, stdio_fixture::icacls,
};
use crate::{
    evidence,
    prebuilt::{
        self, CHILD_SCHEMA, CleanupOrder, FileEntry, KESTREL, MUXER_BYTES, MUXER_SHA256, PACKAGE,
        ParsedInput, RUNTIME_PREFIXES, Selector,
    },
};
use humanymous_platform::testing::parent_context::{
    PreflightEnvironment, UnverifiedBudget,
    admission::{
        AdmissionProfile, MAX_ENVELOPE, UnverifiedEnvelope, decode_envelope, encode_envelope,
    },
    encode_provenance,
    host::{
        ActorIdentity, UnverifiedHostBinding, UnverifiedHostContext, decode_host, encode_host,
        encode_host_binding,
    },
};
use humanymous_platform::windows_object::{
    create_directory_with_descriptor, create_file::CreatedFile,
    create_file::create_new_file_with_descriptor,
};
use humanymous_runtime::native_security::storage::{
    CarrierCreator, CreatedCarrierFile, CreatorChannelFrame, create_user_private_directory,
    inspect_actor_context, publish_actor_context_created_with,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    ffi::{OsStr, OsString},
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use windows_permissions::{LocalBox, SecurityDescriptor};

pub(crate) const CHILD_MODE: &str = "dotnet-prebuilt-actor";
const REPORT_SCHEMA: &str = "humanymous.dotnet-prebuilt-run/1";
/// One original run budget for the whole selector, never renewed (D5(a)).
const RUN_MS: u64 = 180_000;
const CLEANUP_MS: u64 = 60_000;
const CHANNEL_BUDGET: Duration = Duration::from_secs(30);
const CHILD_LINE_LIMIT: usize = 64 * 1024;
const HARNESS_OUTPUT_LIMIT: usize = 1024 * 1024;
/// Artifact root DACL: Administrators and SYSTEM full, actor read/execute only.
const ACTOR_RX: &str = "0x1200a9";

fn other(reason: &'static str) -> io::Error {
    io::Error::other(reason)
}

/// D5-N4 option (a), issue #75 comment 6999924070: the carrier directory and
/// files get the publisher's exact descriptor, owner Administrators included,
/// in the creating call. No existing object is opened and no owner is changed;
/// a refused creation is returned as is and the publisher cleans up.
struct OwnerAtCreation;
struct HeldCarrierFile(CreatedFile);
impl CreatedCarrierFile for HeldCarrierFile {
    fn file(&self) -> &fs::File {
        self.0.file()
    }
}
impl CarrierCreator for OwnerAtCreation {
    fn create_directory(&self, path: &Path, sddl: &str) -> io::Result<()> {
        let descriptor: LocalBox<SecurityDescriptor> = sddl.parse()?;
        // The publisher records and owns the empty-directory cleanup.
        let _created = create_directory_with_descriptor(path, &descriptor)
            .map_err(|_| other("carrier directory owner-at-creation refused"))?;
        Ok(())
    }
    fn create_file(&self, path: &Path, sddl: &str) -> io::Result<Box<dyn CreatedCarrierFile>> {
        let descriptor: LocalBox<SecurityDescriptor> = sddl.parse()?;
        let created = create_new_file_with_descriptor(path, &descriptor)
            .map_err(|_| other("carrier file owner-at-creation refused"))?;
        Ok(Box::new(HeldCarrierFile(created)))
    }
}
fn now_ms() -> io::Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| other("wall clock before epoch"))?
        .as_millis()
        .try_into()
        .map_err(|_| other("wall clock out of range"))
}
fn hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Forward-slash path text, as the `/2` wire and harness checks expect.
fn wire(path: &Path) -> io::Result<String> {
    let text = path.to_str().ok_or_else(|| other("non-UTF-8 path"))?;
    let text = text.strip_prefix(r"\\?\").unwrap_or(text);
    Ok(text.replace('\\', "/"))
}
fn native(path: &str) -> PathBuf {
    PathBuf::from(path.replace('/', "\\"))
}
fn bounded_reader<R: Read + Send + 'static>(
    stream: R,
    limit: usize,
) -> mpsc::Receiver<io::Result<Vec<u8>>> {
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stream
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = sender.send(result);
    });
    receiver
}
fn collect(
    receiver: &mpsc::Receiver<io::Result<Vec<u8>>>,
    until: Instant,
    limit: usize,
) -> Option<Vec<u8>> {
    receiver
        .recv_timeout(until.saturating_duration_since(Instant::now()))
        .ok()
        .and_then(Result::ok)
        .filter(|bytes| bytes.len() <= limit)
}
/// Copy one regular file, hashing what was read and then what landed.
fn stage_file(source: &Path, destination: &Path) -> io::Result<FileEntry> {
    let metadata = fs::symlink_metadata(source)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(other("staged source is not a regular file"));
    }
    let bytes = fs::read(source)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    let staged = fs::read(destination)?;
    if staged != bytes {
        return Err(other("staged bytes differ from the source"));
    }
    Ok(FileEntry {
        path: String::new(),
        bytes: bytes.len() as u64,
        sha256: hex(&bytes),
    })
}
fn walk(base: &Path, directory: &Path, rows: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(other("runtime closure contains a link"));
        }
        if kind.is_dir() {
            walk(base, &entry.path(), rows)?;
        } else if kind.is_file() {
            rows.push(
                entry
                    .path()
                    .strip_prefix(base)
                    .map_err(|_| other("runtime path escapes"))?
                    .to_path_buf(),
            );
        } else {
            return Err(other("runtime closure contains a special file"));
        }
    }
    Ok(())
}
fn set_dacl(path: &Path, sddl: &str) -> io::Result<()> {
    use windows_permissions::{
        LocalBox, SecurityDescriptor,
        constants::{SeObjectType, SecurityInformation},
        wrappers,
    };
    let descriptor: LocalBox<SecurityDescriptor> = sddl.parse()?;
    wrappers::SetNamedSecurityInfo(
        path.as_os_str(),
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        None,
        None,
        descriptor.dacl(),
        None,
    )
}

/// Everything the parent staged into the protected artifact root.
struct Staged {
    app_root: PathBuf,
    runtime_root: PathBuf,
    harness: PathBuf,
    generic_input: PathBuf,
}
/// Stage the closed closures into a new admin-owned root that the actor can
/// only read and execute. The ACL is set before any file exists, so every
/// staged file inherits it. Runtime bytes must equal the pinned SDK closure.
fn stage(input: &ParsedInput, art: &Path, sid: &str, facts: &mut Value) -> io::Result<Staged> {
    fs::create_dir(art)?;
    set_dacl(
        art,
        &format!("D:P(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)(A;OICI;{ACTOR_RX};;;{sid})"),
    )?;
    let app_root = art.join("app");
    let source = native(&input.app_root);
    for row in &input.app {
        let staged = stage_file(&source.join(&row.path), &app_root.join(&row.path))?;
        if staged.bytes != row.bytes || staged.sha256 != row.sha256 {
            return Err(other("staged app file differs from its input row"));
        }
    }
    let runtime_source = native(&input.runtime_root);
    let runtime_root = art.join("runtime");
    let mut paths = Vec::new();
    for prefix in RUNTIME_PREFIXES {
        let directory = runtime_source.join(prefix.trim_end_matches('/').replace('/', "\\"));
        walk(&runtime_source, &directory, &mut paths)?;
    }
    let mut rows = Vec::new();
    for relative in paths {
        let mut staged = stage_file(
            &runtime_source.join(&relative),
            &runtime_root.join(&relative),
        )?;
        staged.path = wire(&relative)?;
        rows.push(staged);
    }
    prebuilt::runtime_closure_ok(&rows)?;
    let muxer = stage_file(
        &runtime_source.join("dotnet.exe"),
        &runtime_root.join("dotnet.exe"),
    )?;
    if muxer.bytes != MUXER_BYTES || muxer.sha256 != MUXER_SHA256 {
        return Err(other("staged muxer differs from the pinned SDK"));
    }
    let harness = art.join("bin").join("harness.exe");
    let staged = stage_file(&native(&input.harness.path), &harness)?;
    if staged.bytes != input.harness.bytes || staged.sha256 != input.harness.sha256 {
        return Err(other("staged harness differs from its input"));
    }
    let program = art.join("src").join("Program.cs");
    let staged = stage_file(&native(&input.managed_source.path), &program)?;
    if staged.sha256 != input.managed_source.sha256 {
        return Err(other("staged managed source differs from its input"));
    }
    let artifact = |path: &Path, sha256: &str| -> io::Result<Value> {
        Ok(json!({"path": path.to_str().ok_or_else(|| other("path"))?, "sha256": sha256}))
    };
    let by_name = |name: &str| {
        input
            .app
            .iter()
            .find(|row| row.path == name)
            .map(|row| row.sha256.clone())
            .ok_or_else(|| other("app row absent"))
    };
    let generic = json!({
        "source_sha": input.source_sha,
        "native_source_sha": input.source_sha,
        "sdk": artifact(&runtime_root.join("dotnet.exe"), MUXER_SHA256)?,
        "assembly": artifact(&app_root.join("Humanymous.HostPilot.dll"), &by_name("Humanymous.HostPilot.dll")?)?,
        "native_library": artifact(&app_root.join("humanymous_native_abi_export.dll"), &by_name("humanymous_native_abi_export.dll")?)?,
        "managed_source": artifact(&program, &input.managed_source.sha256)?,
        "managed_support": [
            artifact(&app_root.join("Humanymous.HostPilot.deps.json"), &by_name("Humanymous.HostPilot.deps.json")?)?,
            artifact(&app_root.join("Humanymous.HostPilot.runtimeconfig.json"), &by_name("Humanymous.HostPilot.runtimeconfig.json")?)?,
            artifact(&app_root.join("Humanymous.Native.dll"), &by_name("Humanymous.Native.dll")?)?,
        ],
    });
    let generic_input = art.join("generic-input.json");
    fs::write(
        &generic_input,
        serde_json::to_vec(&generic).map_err(io::Error::other)?,
    )?;
    facts["staged"] = json!({
        "app_files": input.app.len(),
        "runtime_files": rows.len(),
        "runtime_manifest_sha256": prebuilt::RUNTIME_MANIFEST_SHA256,
        "muxer_sha256": MUXER_SHA256,
        "harness_sha256": input.harness.sha256,
        "actor_access": "read-execute",
    });
    Ok(Staged {
        app_root,
        runtime_root,
        harness,
        generic_input,
    })
}

/// The original `/2` context and binding bytes for this run.
fn context_and_binding(
    input: &ParsedInput,
    staged: &Staged,
    sid: &str,
    nonce: &str,
    roots: (&Path, &Path),
    issued: Instant,
) -> io::Result<(Vec<u8>, Vec<u8>)> {
    let (private_work, handoff) = roots;
    let system = std::env::var("SystemRoot").map_err(io::Error::other)?;
    let issued_ms = now_ms()?;
    let context = UnverifiedHostContext {
        schema: "humanymous.dotnet-host-launch/2".into(),
        apphost: wire(&staged.app_root.join("Humanymous.HostPilot.exe"))?,
        app_root: wire(&staged.app_root)?,
        runtime_root: wire(&staged.runtime_root)?,
        system_root: Some(system.replace('\\', "/")),
        private_work: wire(private_work)?,
        audit_root: wire(&private_work.join("audit"))?,
        handoff_root: wire(handoff)?,
        actor: ActorIdentity::Windows { sid: sid.into() },
        // Not Clone by design; an exact serde round-trip of the parsed value.
        provenance: serde_json::to_value(&input.provenance)
            .and_then(serde_json::from_value)
            .map_err(|_| other("provenance copy"))?,
        budget: UnverifiedBudget {
            issued_utc_ms: issued_ms,
            run_expires_utc_ms: issued_ms + RUN_MS,
            cleanup_expires_utc_ms: issued_ms + RUN_MS + CLEANUP_MS,
            parent_elapsed_ms_at_dispatch: issued.elapsed().as_millis() as u64,
        },
    };
    prebuilt::host_paths_ok(&context)?;
    let context_bytes = encode_host(&context).map_err(|_| other("context encoding"))?;
    let binding = UnverifiedHostBinding {
        schema: "humanymous.parent-context-binding/2".into(),
        run_nonce: nonce.into(),
        actor: context.actor.clone(),
        source_sha: input.source_sha.clone(),
        context_sha256: hex(&context_bytes),
        verified_manifest_sha256: input.manifest_sha256.clone(),
        provenance_sha256: hex(
            &encode_provenance(&context.provenance).map_err(|_| other("provenance"))?
        ),
        original_budget: UnverifiedBudget { ..context.budget },
        preflight_environment: PreflightEnvironment::GithubHosted,
    };
    let binding_bytes =
        encode_host_binding(&binding, &context).map_err(|_| other("binding encoding"))?;
    Ok((context_bytes, binding_bytes))
}

/// Diagnostic only: who owns a directory this elevated parent creates. The
/// carrier publisher requires Administrators or SYSTEM and never repairs it.
fn default_owner_class(scratch: &Path) -> &'static str {
    use windows_permissions::{
        LocalBox, Sid,
        constants::{SeObjectType, SecurityInformation},
        wrappers,
    };
    let probe = scratch.join("owner-probe");
    if fs::create_dir(&probe).is_err() {
        return "probe-failed";
    }
    let class = (|| -> io::Result<&'static str> {
        let sd = wrappers::GetNamedSecurityInfo(
            probe.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            SecurityInformation::Owner,
        )?;
        let admin: LocalBox<Sid> = "S-1-5-32-544".parse()?;
        let system: LocalBox<Sid> = "S-1-5-18".parse()?;
        Ok(match sd.owner() {
            Some(owner) if owner == &*admin => "administrators",
            Some(owner) if owner == &*system => "system",
            Some(_) => "other",
            None => "absent",
        })
    })()
    .unwrap_or("unreadable");
    let _ = fs::remove_dir(&probe);
    class
}

fn remove_tree(path: &Path, base: &Path, name: &str) -> io::Result<()> {
    if path.parent() != Some(base) || path.file_name() != Some(OsStr::new(name)) {
        return Err(other("unsafe generated cleanup path"));
    }
    match fs::remove_dir_all(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

/// Elevated hosted parent: `dotnet-prebuilt --input <abs> --report <abs>`.
pub(crate) fn run(input_path: &Path, report: &Path) -> io::Result<i32> {
    if !input_path.is_absolute() || !report.is_absolute() {
        return Err(other("absolute --input and --report required"));
    }
    if !evidence::disposable_host() {
        return Err(other("disposable GitHub-hosted actor required"));
    }
    let identity = humanymous_runtime::native_security::process::current_identity()?;
    if !identity.elevated() || !identity.enabled_administrator() {
        return Err(other("hosted provisioning administrator required"));
    }
    let mut raw = Vec::new();
    fs::File::open(input_path)?
        .take(prebuilt::INPUT_LIMIT as u64 + 1)
        .read_to_end(&mut raw)?;
    let input = prebuilt::parse_input(&raw)?;
    let logs = report
        .parent()
        .ok_or_else(|| other("report directory required"))?
        .to_path_buf();
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).map_err(io::Error::other)?;
    let nonce: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let (name, password) = actor_for_nonce(&nonce)?;
    let mut owner = account(&name, password)?;
    let (sid, _, _) =
        windows_permissions::wrappers::LookupAccountName(Option::<&OsStr>::None, &name)?;
    let sid = sid.to_string();
    let system = std::env::var("SystemRoot").map_err(io::Error::other)?;
    let drive = std::env::var("SystemDrive").map_err(io::Error::other)?;
    let profile_directory = PathBuf::from(format!("{drive}\\Users\\{name}"));
    let base =
        PathBuf::from(std::env::var_os("ProgramData").ok_or_else(|| other("ProgramData missing"))?)
            .canonicalize()?;
    let base = native(&wire(&base)?);
    let scratch = base.join(&name);
    let art_name = format!("hmart-{nonce}");
    let art = base.join(&art_name);
    let exe = std::env::current_exe()?.canonicalize()?;
    let mut facts = json!({"selector": input.selector.label()});
    let mut order = CleanupOrder::default();
    let mut tests: Vec<Value> = Vec::new();
    let mut child_line = Value::Null;
    let mut published = None;
    let outcome = (|| -> io::Result<()> {
        fs::create_dir(&scratch)?;
        acl(&scratch, &sid, identity.sid())?;
        let handoff = scratch.join("handoff");
        fs::create_dir(&handoff)?;
        acl(&handoff, &sid, identity.sid())?;
        let private_work = scratch.join("work");
        let staged = stage(&input, &art, &sid, &mut facts)?;
        icacls(&exe, &["/grant", &format!("*{sid}:(RX)"), "/Q", "/L"])?;
        icacls(
            &exe,
            &[
                "/deny",
                &format!("*{sid}:(WD,AD,WEA,WA,DE,DC,WDAC,WO)"),
                "/Q",
                "/L",
            ],
        )?;
        let issued = Instant::now();
        let (context, binding) = context_and_binding(
            &input,
            &staged,
            &sid,
            &nonce,
            (&private_work, &handoff),
            issued,
        )?;
        let frame = encode_envelope(&UnverifiedEnvelope {
            profile: AdmissionProfile::StandardUserDotnetPrebuilt,
            run_nonce: nonce.clone(),
            derived: None,
            context: context.clone(),
            binding: binding.clone(),
        })
        .map_err(|_| other("envelope encoding"))?;
        facts["admin_default_owner"] = json!(default_owner_class(&scratch));
        use humanymous_runtime::native_security::storage::ContextPublishError;
        let carrier = match publish_actor_context_created_with(
            &OwnerAtCreation,
            &nonce,
            &sid,
            &context,
            &binding,
        ) {
            Ok(carrier) => {
                facts["carrier_publish"] = json!("published");
                carrier
            }
            Err(ContextPublishError::BeforeCreation(error)) => {
                facts["carrier_publish"] = json!("refused-before-creation");
                facts["carrier_error"] = json!(error.to_string());
                return Err(error);
            }
            Err(ContextPublishError::Retained { owner, error }) => {
                // The partial carrier is cleaned through its own owner.
                facts["carrier_publish"] = json!("refused-after-creation");
                facts["carrier_error"] = json!(error.to_string());
                facts["partial_carrier_removed"] = json!(owner.finish().is_ok());
                return Err(error);
            }
        };
        let context_path = carrier.context_path();
        published = Some(carrier);
        let environment = vec![
            ("SystemRoot".to_owned(), system.clone()),
            ("WINDIR".to_owned(), system.clone()),
            ("PATH".to_owned(), format!("{system}\\System32;{system}")),
        ];
        let mut arguments: Vec<OsString> = vec![
            CHILD_MODE.into(),
            "--context".into(),
            context_path.into_os_string(),
            "--selector".into(),
            input.selector.label().into(),
            "--harness".into(),
            staged.harness.clone().into_os_string(),
        ];
        if input.selector == Selector::Hosted {
            arguments.extend([
                "--generic-input".into(),
                staged.generic_input.clone().into_os_string(),
            ]);
        }
        let absent_before = profile_absent(&sid, &profile_directory)?;
        facts["profile_absent_before"] = json!(absent_before);
        order.start(absent_before)?;
        let actor = match start_prebuilt_actor(
            &owner.account,
            &owner.password,
            (&arguments, &environment, &scratch),
            &sid,
            &profile_directory,
        ) {
            Ok(actor) => actor,
            Err(StartFailure::NotStarted(error)) => {
                facts["start"] = json!("not-started");
                return Err(error);
            }
            Err(StartFailure::Retained(mut failure)) => {
                facts["start"] = json!("retained-failure");
                let contained = failure
                    .owner
                    .contain(Instant::now() + Duration::from_millis(CLEANUP_MS));
                facts["retained_contained"] = json!(contained.is_ok());
                return Err(failure.error);
            }
        };
        facts["start"] = json!("resumed-in-job");
        let super::PrebuiltActor {
            owner: mut process,
            mut creator,
            output,
            error,
        } = actor;
        let stdout = bounded_reader(output, CHILD_LINE_LIMIT);
        let stderr = bounded_reader(error, CHILD_LINE_LIMIT);
        let written = creator.write_all(&frame).and_then(|()| creator.flush());
        // The parent's only write end closes: the child observes EOF only if
        // no other write end of this creator pipe is alive.
        drop(creator);
        facts["frame_written"] = json!(written.is_ok());
        let run_until = issued + Duration::from_millis(RUN_MS);
        let cleanup_until = run_until + Duration::from_millis(CLEANUP_MS);
        // The child owns the run budget for its harnesses; the parent waits
        // for it only up to the original cleanup deadline, never longer.
        let run = process.wait_run(cleanup_until);
        let proof = process.contain(cleanup_until);
        let (exit_code, natural, job_scope) = match (&run, &proof) {
            (Ok(exit), Ok(proof)) => (
                Some(exit.code),
                exit.natural && proof.exit.natural,
                proof.scope == CleanupTarget::OwnedJob,
            ),
            _ => (None, false, false),
        };
        drop(proof);
        facts["actor_exit_code"] = json!(exit_code);
        facts["natural_exit"] = json!(natural);
        facts["job_empty_observed"] = json!(job_scope);
        facts["elapsed_ms_from_issue"] = json!(issued.elapsed().as_millis() as u64);
        if job_scope {
            order.main_empty()?;
        }
        let (out, err) = (
            collect(&stdout, cleanup_until, CHILD_LINE_LIMIT),
            collect(&stderr, cleanup_until, CHILD_LINE_LIMIT),
        );
        order.readers_joined = out.is_some() && err.is_some();
        facts["readers_joined"] = json!(order.readers_joined);
        if let Some(bytes) = err.as_deref().filter(|bytes| !bytes.is_empty()) {
            fs::write(logs.join("dotnet-prebuilt-actor.stderr.log"), bytes)?;
        }
        child_line = out
            .as_deref()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(|text| {
                let line = text.strip_suffix('\n')?;
                let line = line.strip_suffix('\r').unwrap_or(line);
                (!line.contains('\n')).then(|| serde_json::from_str::<Value>(line).ok())?
            })
            .unwrap_or(Value::Null);
        drop(process);
        written?;
        run?;
        // Handoff: bounded logs the actor wrote before its private cleanup.
        for name in input.selector.tests() {
            let stage = stage_name(name);
            let mut row = json!({"name": name});
            let mut passed = false;
            for (suffix, log) in [("stdout", ".log"), ("stderr", ".stderr.log")] {
                let path = handoff.join(format!("{stage}.{suffix}"));
                let bytes = fs::read(&path)
                    .ok()
                    .filter(|b| b.len() <= HARNESS_OUTPUT_LIMIT);
                row[format!("{suffix}_sha256")] = json!(bytes.as_deref().map(hex));
                if let Some(bytes) = &bytes {
                    fs::write(logs.join(format!("dotnet-{stage}{log}")), bytes)?;
                    if suffix == "stdout" {
                        passed =
                            prebuilt::single_test_passed(&String::from_utf8_lossy(bytes), name);
                    }
                }
            }
            row["passed"] = json!(passed);
            tests.push(row);
        }
        let handoff_files = fs::read_dir(&handoff)?.count();
        facts["handoff_files"] = json!(handoff_files);
        let private_absent = !private_work.exists();
        facts["private_absent"] = json!(private_absent);
        if private_absent && child_line["private_removed"] == true {
            order.private_removed()?;
        }
        order.outcome_bound = prebuilt::child_line_ok(&child_line, input.selector);
        if exit_code != Some(0) || !natural || !job_scope || !order.outcome_bound {
            return Err(other("prebuilt actor outcome incomplete"));
        }
        Ok(())
    })();
    let carrier_finished = match published.take() {
        Some(carrier) => carrier.finish().is_ok(),
        None => false,
    };
    let ungranted = icacls(&exe, &["/remove", &format!("*{sid}"), "/Q", "/L"]);
    let profile_after = matches!(profile_absent(&sid, &profile_directory), Ok(true));
    if profile_after && order.profile_absent_after().is_err() {
        facts["cleanup_order"] = json!("profile absence observed out of order");
    }
    let deletion = if order.delete_account().is_ok() || !order.started() {
        // Before launch no ordering fact applies; the run already failed.
        owner.delete_legacy_account()
    } else {
        // Out of order: the account is still deleted, but the run cannot pass.
        let result = owner.delete_legacy_account();
        result.and(Err(other("account deleted before every cleanup fact")))
    };
    let art_removed = remove_tree(&art, &base, &art_name);
    let scratch_removed = remove_tree(&scratch, &base, &name);
    if deletion.is_ok() && art_removed.is_ok() && scratch_removed.is_ok() {
        let _ = order.delete_scratch();
    }
    let mut failures = Vec::new();
    for (label, result) in [
        ("prebuilt", outcome.as_ref().map(|_| ())),
        ("exe grant removal", ungranted.as_ref().map(|_| ())),
        ("account deletion", deletion.as_ref().map(|_| ())),
        ("artifact root removal", art_removed.as_ref().map(|_| ())),
        ("scratch removal", scratch_removed.as_ref().map(|_| ())),
    ] {
        if let Err(error) = result {
            failures.push(format!("{label}: {error}"));
        }
    }
    if !carrier_finished {
        failures.push("actor context carrier not finished".into());
    }
    if !profile_after {
        failures.push("profile present or unreadable after NoProfile launch".into());
    }
    let executed = tests.iter().all(|row| row["passed"] == true) && !tests.is_empty();
    let clean = order.clean_pass(executed) && failures.is_empty();
    if !clean && failures.is_empty() {
        failures.push("clean-pass facts incomplete".into());
    }
    facts["profile_absent_after"] = json!(profile_after);
    facts["account_deleted"] = json!(deletion.is_ok());
    facts["artifact_root_removed"] = json!(art_removed.is_ok());
    facts["scratch_deleted"] = json!(scratch_removed.is_ok());
    facts["carrier_finished"] = json!(carrier_finished);
    let value = json!({
        "schema": REPORT_SCHEMA,
        "sot": "sot:root/dotnet-protected-host-launch N1-N5",
        "decision": "D5(a) issue 75 comment 6998447599",
        "profile": prebuilt::PROFILE,
        "selector": input.selector.label(),
        "status": if clean { "passed" } else { "failed" },
        "budget_ms": {"run": RUN_MS, "cleanup": CLEANUP_MS, "renewed": false},
        "facts": facts,
        "child": child_line,
        "tests": tests,
        "failures": failures,
    });
    evidence::write(report, &value)?;
    Ok(if clean { 0 } else { 1 })
}

fn stage_name(test: &str) -> &'static str {
    match test {
        KESTREL => "kestrel",
        PACKAGE => "package",
        _ => "generic",
    }
}

/// Exactly one bounded frame then EOF from this process's inherited stdin.
fn creator_frame() -> io::Result<(bool, bool, Vec<u8>)> {
    let pipe = humanymous_platform::windows_object::stdin_is_pipe()
        .map_err(|_| other("stdin type query failed"))?;
    if !pipe {
        return Ok((false, false, Vec::new()));
    }
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = io::stdin()
            .lock()
            .take(MAX_ENVELOPE as u64 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = sender.send(result);
    });
    match receiver.recv_timeout(CHANNEL_BUDGET) {
        Ok(Ok(bytes)) if bytes.len() <= MAX_ENVELOPE => Ok((true, true, bytes)),
        _ => Ok((true, false, Vec::new())),
    }
}

struct ChildOptions {
    context: PathBuf,
    selector: Selector,
    harness: PathBuf,
    generic_input: Option<PathBuf>,
}
fn child_options(args: &mut impl Iterator<Item = String>) -> io::Result<ChildOptions> {
    let mut fields: [Option<String>; 4] = [None, None, None, None];
    while let Some(flag) = args.next() {
        let index = match flag.as_str() {
            "--context" => 0,
            "--selector" => 1,
            "--harness" => 2,
            "--generic-input" => 3,
            _ => return Err(other("unknown actor option")),
        };
        let value = args.next().ok_or_else(|| other("actor option value"))?;
        if fields[index].replace(value).is_some() {
            return Err(other("duplicate actor option"));
        }
    }
    let [Some(context), Some(selector), Some(harness), generic] = fields else {
        return Err(other("--context --selector --harness required"));
    };
    let selector = Selector::parse(&selector)?;
    if generic.is_some() != (selector == Selector::Hosted) {
        return Err(other("--generic-input belongs to the hosted selector only"));
    }
    Ok(ChildOptions {
        context: context.into(),
        selector,
        harness: harness.into(),
        generic_input: generic.map(PathBuf::from),
    })
}

/// The actor's own private root: created by the actor, so owned by it, with
/// a protected DACL naming only the actor (the harness's private-parent rule).
fn actor_private_root(path: &Path, sid: &str) -> io::Result<()> {
    fs::create_dir(path)?;
    set_dacl(path, &format!("D:P(A;OICI;FA;;;{sid})"))
}

/// One harness run under the actor, inside the inherited Job.
fn run_harness(
    options: &ChildOptions,
    test: &str,
    environment: &[(String, String)],
    stdin: Option<&[u8]>,
    work: &Path,
    run_until: Instant,
) -> io::Result<(Value, Vec<u8>, Vec<u8>)> {
    let started = Instant::now();
    let mut child = Command::new(&options.harness)
        .args(prebuilt::harness_arguments(options.selector, test)?)
        .env_clear()
        .envs(environment.iter().map(|(k, v)| (k, v)))
        .current_dir(work)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let channel = match (stdin, child.stdin.take()) {
        (Some(frame), Some(mut pipe)) => pipe.write_all(frame).is_ok(),
        (None, None) => true,
        _ => false,
    };
    let stdout = bounded_reader(
        child.stdout.take().ok_or_else(|| other("stdout"))?,
        HARNESS_OUTPUT_LIMIT,
    );
    let stderr = bounded_reader(
        child.stderr.take().ok_or_else(|| other("stderr"))?,
        HARNESS_OUTPUT_LIMIT,
    );
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if Instant::now() >= run_until {
            timed_out = true;
            let _ = child.kill();
            break child.wait().ok();
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let reader_until = Instant::now() + Duration::from_secs(10);
    let out = collect(&stdout, reader_until, HARNESS_OUTPUT_LIMIT);
    let err = collect(&stderr, reader_until, HARNESS_OUTPUT_LIMIT);
    let row = json!({
        "name": test,
        "exit_code": status.and_then(|status| status.code()),
        "timed_out": timed_out,
        "readers_joined": out.is_some() && err.is_some(),
        "creator_channel": stdin.is_some() && channel,
        "elapsed_ms": started.elapsed().as_millis() as u64,
    });
    Ok((row, out.unwrap_or_default(), err.unwrap_or_default()))
}

/// Actor child: bind the creator channel (N3), then run the selector's tests.
pub(crate) fn child(args: &mut impl Iterator<Item = String>) -> io::Result<i32> {
    let options = child_options(args)?;
    let (pipe, single, frame) = creator_frame()?;
    let received = Instant::now();
    let wall_ms = now_ms()?;
    let envelope = decode_envelope(&frame).map_err(|_| other("malformed creator envelope"))?;
    let mut inspected = inspect_actor_context(&options.context)?;
    inspected.bind_creator_channel(&CreatorChannelFrame {
        stdin_pipe: pipe,
        single_frame_eof: single,
        run_nonce: &envelope.run_nonce,
        context: &envelope.context,
        binding: &envelope.binding,
    })?;
    inspected.require_exact_producer_identity()?;
    if envelope.profile != AdmissionProfile::StandardUserDotnetPrebuilt
        || envelope.derived.is_some()
    {
        return Err(other("original prebuilt envelope required"));
    }
    let context = decode_host(&envelope.context).map_err(|_| other("malformed context"))?;
    prebuilt::host_paths_ok(&context)?;
    if context.actor
        != (ActorIdentity::Windows {
            sid: inspected.actor_sid().into(),
        })
    {
        return Err(other("actual actor differs from the context"));
    }
    let budget = context
        .map_budget(wall_ms, received)
        .map_err(|_| other("original run budget exhausted"))?;
    let run_until = budget
        .run_deadline()
        .ok_or_else(|| other("run deadline absent"))?;
    let harness_bytes = fs::read(&options.harness)?;
    let harness_verified = hex(&harness_bytes) == context.provenance.harness.receipt_sha256;
    drop(harness_bytes);
    if !harness_verified {
        return Err(other("harness differs from its provenance receipt"));
    }
    let work = native(&context.private_work);
    let sid = inspected.actor_sid().to_owned();
    actor_private_root(&work, &sid)?;
    let mut private = true;
    for name in ["temp", "profile", "audit", "generic-report"] {
        private &= create_user_private_directory(&work, name).is_ok();
    }
    let system = context
        .system_root
        .clone()
        .ok_or_else(|| other("system root"))?;
    let system_native = system.replace('/', "\\");
    let mut environment = vec![
        ("SystemRoot".to_owned(), system_native.clone()),
        ("WINDIR".to_owned(), system_native.clone()),
        (
            "PATH".to_owned(),
            format!("{system_native}\\System32;{system_native}"),
        ),
        ("TEMP".to_owned(), format!("{}\\temp", work.display())),
        ("TMP".to_owned(), format!("{}\\temp", work.display())),
        (
            "USERPROFILE".to_owned(),
            format!("{}\\profile", work.display()),
        ),
        ("RUST_BACKTRACE".to_owned(), "0".to_owned()),
        ("GITHUB_ACTIONS".to_owned(), "true".to_owned()),
        ("RUNNER_ENVIRONMENT".to_owned(), "github-hosted".to_owned()),
    ];
    let mut rows = Vec::new();
    let handoff = native(&context.handoff_root);
    let mut handoff_written = true;
    if private {
        for test in options.selector.tests() {
            let mut env = environment.clone();
            let stdin = if *test == KESTREL || *test == PACKAGE {
                Some(frame.as_slice())
            } else {
                let input = options
                    .generic_input
                    .as_ref()
                    .ok_or_else(|| other("generic input"))?;
                env.extend([
                    (
                        "HUMANYMOUS_DOTNET_GENERIC_INPUT".to_owned(),
                        input.display().to_string(),
                    ),
                    (
                        "HUMANYMOUS_DOTNET_GENERIC_REPORT_DIR".to_owned(),
                        format!("{}\\generic-report", work.display()),
                    ),
                ]);
                None
            };
            let (row, out, err) = run_harness(&options, test, &env, stdin, &work, run_until)?;
            let stage = stage_name(test);
            for (suffix, bytes) in [("stdout", &out), ("stderr", &err)] {
                handoff_written &= fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(handoff.join(format!("{stage}.{suffix}")))
                    .and_then(|mut file| file.write_all(bytes))
                    .is_ok();
            }
            rows.push(row);
        }
    }
    environment.clear();
    // Same-user private cleanup before the actor exits; no admin repair.
    let private_removed = fs::remove_dir_all(&work).is_ok() && !work.exists();
    let line = json!({
        "schema": CHILD_SCHEMA,
        "selector": options.selector.label(),
        "producer_bound": true,
        "harness_verified": harness_verified,
        "private_created": private,
        "private_removed": private_removed,
        "handoff_written": handoff_written,
        "tests": rows,
    });
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{line}")?;
    stdout.flush()?;
    Ok(if prebuilt::child_line_ok(&line, options.selector) {
        0
    } else {
        3
    })
}

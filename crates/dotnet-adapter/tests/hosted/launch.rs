//! L4 creator-channel admission (SoT dotnet-protected-host-launch 3.1-3.3).
//! The only input `Host::start()` accepts. Built once per process from the
//! stdin pipe this process inherited at creation; never from a path, an
//! environment locator, a hash match or a caller DTO. Every refusal happens
//! before any `Command` exists.
use humanymous_platform::testing::parent_context::{
    self, UnverifiedMappedBudget, UnverifiedParentContext,
    admission::{
        AdmissionProfile, Derivation, DerivedMode, UnverifiedEnvelope, decode_envelope,
        encode_envelope,
    },
    host::{ActorIdentity, UnverifiedHostContext, decode_host, decode_host_binding},
};
use sha2::{Digest, Sha256};
use std::{
    io::{self, ErrorKind},
    path::{Component, Path},
    sync::OnceLock,
    time::{Duration, Instant},
};

/// The one custody provider profile this consumer admits.
const PROFILE: AdmissionProfile = AdmissionProfile::StandardUserDotnetPrebuilt;
/// Upper bound for the creator to deliver one frame and close its write end.
const CHANNEL_BUDGET: Duration = Duration::from_secs(30);

/// Native facts this process observed itself. None come from the envelope.
pub struct Facts {
    /// Standard input is a pipe object (Linux FIFO via fstat on a safe clone of
    /// fd 0; Windows `GetFileType` == `FILE_TYPE_PIPE`).
    pub stdin_pipe: bool,
    /// Actual identity: real and effective uid (Linux) or TokenUser SID.
    pub actor: Option<ActorIdentity>,
    /// An `ACT` emulation marker is present.
    pub act_marker: bool,
    /// Classified execution venue, if any.
    pub venue: Option<&'static str>,
    /// OS backend prerequisites observed (Linux: systemd PID 1, cgroup v2 and
    /// system bus). macOS and unknown OSes are never observed.
    pub backend_observed: bool,
    /// Linux: the leaf unit of this process's unified cgroup, if a scope.
    pub scope: Option<String>,
    /// Linux backend applies; Windows uses the provider's Job instead.
    pub requires_scope: bool,
}

/// Private, `!Clone`, process-wide. No public constructor besides [`admit`].
pub struct Admission {
    context: UnverifiedHostContext,
    budget: UnverifiedMappedBudget,
    derived: Option<Derivation>,
    /// Exact received bytes and nonce; only a narrowing [`Admission::derive`]
    /// re-sends them, unchanged, to a same-actor child.
    run_nonce: String,
    context_bytes: Vec<u8>,
    binding_bytes: Vec<u8>,
}
impl std::fmt::Debug for Admission {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("creator-channel admission")
    }
}
impl Admission {
    pub fn apphost(&self) -> &Path {
        Path::new(
            self.derived
                .as_ref()
                .map_or(&self.context.apphost, |derived| &derived.apphost),
        )
    }
    pub fn audit_root(&self) -> &Path {
        Path::new(
            self.derived
                .as_ref()
                .map_or(&self.context.audit_root, |derived| &derived.audit_root),
        )
    }
    /// Root the apphost must stay under: app_root, or private_work if derived.
    pub fn apphost_root(&self) -> &Path {
        Path::new(if self.derived.is_some() {
            &self.context.private_work
        } else {
            &self.context.app_root
        })
    }
    pub fn private_work(&self) -> &Path {
        Path::new(&self.context.private_work)
    }
    pub fn fixed_environment(&self) -> Vec<(&'static str, String)> {
        self.context.fixed_environment()
    }
    /// Derived envelope for one package-matrix mode (SoT 3.5). The context and
    /// binding bytes, actor, nonce and original budget are re-sent unchanged;
    /// only the apphost and audit root narrow under the ORIGINAL private_work.
    /// A derived admission never derives again.
    #[allow(dead_code, reason = "used by the package-matrix controller only")]
    pub fn derive(
        &self,
        mode: DerivedMode,
        apphost: &Path,
        audit_root: &Path,
    ) -> io::Result<Vec<u8>> {
        let text = |path: &Path| {
            path.to_str()
                .map(str::to_owned)
                .ok_or_else(|| refuse(ErrorKind::PermissionDenied, "non-UTF-8 derived path"))
        };
        let (apphost, audit_root) = (text(apphost)?, text(audit_root)?);
        if self.derived.is_some()
            || !within(&apphost, &self.context.private_work)
            || !within(&audit_root, &self.context.private_work)
        {
            return Err(refuse(
                ErrorKind::PermissionDenied,
                "derived admission widens parent",
            ));
        }
        encode_envelope(&UnverifiedEnvelope {
            profile: PROFILE,
            run_nonce: self.run_nonce.clone(),
            derived: Some(Derivation {
                mode,
                apphost,
                audit_root,
            }),
            context: self.context_bytes.clone(),
            binding: self.binding_bytes.clone(),
        })
        .map_err(|_| refuse(ErrorKind::PermissionDenied, "derived envelope bounds"))
    }
    /// The ONE original mapping; sequential profiles borrow it unchanged.
    pub fn deadlines(&self) -> io::Result<(Instant, Instant)> {
        let run = self
            .budget
            .run_deadline()
            .ok_or_else(|| refuse(ErrorKind::PermissionDenied, "cleanup-only context"))?;
        Ok((run, self.budget.cleanup_deadline()))
    }
}

fn refuse(kind: ErrorKind, reason: &'static str) -> io::Error {
    io::Error::new(kind, reason)
}

/// Lexical containment with only normal components; equality is not inside.
fn within(child: &str, root: &str) -> bool {
    let plain = |path: &Path| {
        path.is_absolute()
            && path.components().all(|component| {
                matches!(
                    component,
                    Component::RootDir | Component::Prefix(_) | Component::Normal(_)
                )
            })
    };
    let (child, root) = (Path::new(child), Path::new(root));
    plain(child) && plain(root) && child != root && child.starts_with(root)
}

fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Pure admission over one received frame and this process's own facts.
/// Order follows the SoT 3.3 table; the first failing row decides the kind.
pub fn admit(
    frame: &[u8],
    facts: &Facts,
    wall_ms: u64,
    received: Instant,
) -> io::Result<Admission> {
    use ErrorKind::{PermissionDenied, TimedOut, Unsupported};
    if !facts.stdin_pipe {
        return Err(refuse(Unsupported, "creator channel is not a pipe"));
    }
    let envelope =
        decode_envelope(frame).map_err(|_| refuse(Unsupported, "malformed creator envelope"))?;
    let context = match decode_host(&envelope.context) {
        Ok(context) => context,
        Err(_) => {
            return Err(match parent_context::decode(&envelope.context) {
                Ok(UnverifiedParentContext::CleanupOnly(_)) => {
                    refuse(PermissionDenied, "cleanup-only context")
                }
                _ => refuse(Unsupported, "malformed launch context"),
            });
        }
    };
    if !facts.backend_observed {
        return Err(refuse(Unsupported, "OS backend not qualified"));
    }
    if facts.requires_scope
        && facts.scope.as_deref() != Some(&format!("hm-dotnet-{}.scope", envelope.run_nonce))
    {
        return Err(refuse(Unsupported, "provider scope not observed"));
    }
    if envelope.profile != PROFILE {
        return Err(refuse(PermissionDenied, "profile mismatch"));
    }
    let binding = match decode_host_binding(&envelope.binding, &context) {
        Ok(binding) => binding,
        Err(_) if serde_json::from_slice::<serde_json::Value>(&envelope.binding).is_err() => {
            return Err(refuse(Unsupported, "malformed binding"));
        }
        Err(_) => return Err(refuse(PermissionDenied, "binding mismatch")),
    };
    let provenance = parent_context::encode_provenance(&context.provenance)
        .map_err(|_| refuse(PermissionDenied, "provenance commitment"))?;
    if binding.run_nonce != envelope.run_nonce
        || binding.context_sha256 != hex_digest(&envelope.context)
        || binding.provenance_sha256 != hex_digest(&provenance)
    {
        return Err(refuse(PermissionDenied, "binding commitment mismatch"));
    }
    if facts.act_marker || facts.venue.is_none() {
        return Err(refuse(PermissionDenied, "unclassified venue"));
    }
    match &facts.actor {
        Some(ActorIdentity::Linux { uid: 0 }) | None => {
            return Err(refuse(PermissionDenied, "ordinary actor required"));
        }
        Some(actual) if *actual != context.actor => {
            return Err(refuse(PermissionDenied, "actual actor mismatch"));
        }
        Some(_) => {}
    }
    if !within(&context.audit_root, &context.private_work) {
        return Err(refuse(PermissionDenied, "audit root outside private work"));
    }
    match &envelope.derived {
        Some(derived) => {
            if !within(&derived.apphost, &context.private_work)
                || !within(&derived.audit_root, &context.private_work)
            {
                return Err(refuse(PermissionDenied, "derived admission widens parent"));
            }
        }
        None => {
            if !within(&context.apphost, &context.app_root) {
                return Err(refuse(PermissionDenied, "apphost outside app root"));
            }
        }
    }
    let budget = context
        .map_budget(wall_ms, received)
        .map_err(|_| refuse(TimedOut, "original run budget exhausted"))?;
    match budget.run_deadline() {
        Some(run) if budget.cleanup_deadline() > run && run > received => {}
        _ => return Err(refuse(TimedOut, "cleanup reserve not positive")),
    }
    Ok(Admission {
        context,
        budget,
        derived: envelope.derived,
        run_nonce: envelope.run_nonce,
        context_bytes: envelope.context,
        binding_bytes: envelope.binding,
    })
}

#[cfg(unix)]
fn creator_channel() -> io::Result<(bool, std::fs::File)> {
    use std::os::{fd::AsFd, unix::fs::FileTypeExt};
    let file = std::fs::File::from(std::io::stdin().as_fd().try_clone_to_owned()?);
    let pipe = file.metadata()?.file_type().is_fifo();
    Ok((pipe, file))
}
#[cfg(windows)]
fn creator_channel() -> io::Result<(bool, std::fs::File)> {
    use std::os::windows::io::AsHandle;
    let pipe = humanymous_platform::windows_object::stdin_is_pipe()
        .map_err(|_| refuse(ErrorKind::Unsupported, "stdin type unavailable"))?;
    let file = std::fs::File::from(std::io::stdin().as_handle().try_clone_to_owned()?);
    Ok((pipe, file))
}

/// Exactly one bounded frame then EOF, inside a fixed delivery budget.
fn read_frame(file: std::fs::File) -> io::Result<Vec<u8>> {
    use std::io::Read;
    let limit = parent_context::admission::MAX_ENVELOPE;
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("dotnet-admission-channel".into())
        .spawn(move || {
            let mut bytes = Vec::new();
            let result = file
                .take(limit as u64 + 1)
                .read_to_end(&mut bytes)
                .map(|_| bytes);
            let _ = sender.send(result);
        })?;
    let bytes = receiver
        .recv_timeout(CHANNEL_BUDGET)
        .map_err(|_| refuse(ErrorKind::Unsupported, "creator channel EOF not observed"))??;
    if bytes.len() > limit {
        return Err(refuse(ErrorKind::Unsupported, "oversize creator envelope"));
    }
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn backend() -> (bool, Option<String>, bool) {
    use humanymous_platform::testing::linux_tree_probe::{Availability, inspect};
    let probe = inspect();
    let observed = [
        probe.systemd_pid1,
        probe.unified_cgroup,
        probe.system_bus_socket,
    ]
    .iter()
    .all(|value| *value == Availability::Observed);
    let scope = std::fs::read_to_string("/proc/self/cgroup")
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.strip_prefix("0::"))
                .and_then(|path| path.rsplit('/').next())
                .filter(|leaf| leaf.ends_with(".scope"))
                .map(str::to_owned)
        });
    (observed, scope, true)
}
#[cfg(windows)]
fn backend() -> (bool, Option<String>, bool) {
    (true, None, false)
}
#[cfg(not(any(target_os = "linux", windows)))]
fn backend() -> (bool, Option<String>, bool) {
    (false, None, true)
}

#[cfg(unix)]
fn actual_actor() -> Option<ActorIdentity> {
    let (real, effective) = humanymous_runtime::native_security::process::current_uids();
    (real == effective).then_some(ActorIdentity::Linux { uid: real })
}
#[cfg(windows)]
fn actual_actor() -> Option<ActorIdentity> {
    humanymous_runtime::native_security::process::current_identity()
        .ok()
        .map(|identity| ActorIdentity::Windows {
            sid: identity.sid().to_owned(),
        })
}

fn observe_and_admit() -> io::Result<Admission> {
    let (pipe, file) = creator_channel()?;
    let received = Instant::now();
    let wall_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| refuse(ErrorKind::TimedOut, "wall clock before epoch"))?
        .as_millis()
        .try_into()
        .map_err(|_| refuse(ErrorKind::TimedOut, "wall clock out of range"))?;
    let frame = if pipe { read_frame(file)? } else { Vec::new() };
    let (backend_observed, scope, requires_scope) = backend();
    let facts = Facts {
        stdin_pipe: pipe,
        actor: actual_actor(),
        act_marker: std::env::var_os("ACT").is_some(),
        venue: super::support::venue(),
        backend_observed,
        scope,
        requires_scope,
    };
    admit(&frame, &facts, wall_ms, received)
}

/// Built once on first call; a refusal is sticky for the process lifetime.
pub fn admission() -> io::Result<&'static Admission> {
    static ADMISSION: OnceLock<Result<Admission, (ErrorKind, String)>> = OnceLock::new();
    ADMISSION
        .get_or_init(|| observe_and_admit().map_err(|error| (error.kind(), error.to_string())))
        .as_ref()
        .map_err(|(kind, reason)| io::Error::new(*kind, reason.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use humanymous_platform::testing::parent_context::{
        ManagedLineage, NativeLineage, PreflightEnvironment, UnverifiedBudget, UnverifiedCleanup,
        UnverifiedProvenance,
        host::{UnverifiedHostBinding, encode_host, encode_host_binding},
    };

    const NONCE: &str = "0123456789abcdef0123456789abcdef";
    /// Fixture path that is absolute on the host OS: `within()` requires
    /// `Path::is_absolute()`, and a drive-less rooted path is not on Windows.
    #[cfg(windows)]
    fn at(posix: &str) -> String {
        format!("C:{}", posix.replace('/', "\\"))
    }
    #[cfg(not(windows))]
    fn at(posix: &str) -> String {
        posix.to_owned()
    }
    fn provenance() -> UnverifiedProvenance {
        let native = || NativeLineage {
            source_sha: "a".repeat(40),
            cargo_lock_sha256: "b".repeat(64),
            receipt_sha256: "c".repeat(64),
        };
        UnverifiedProvenance {
            managed: ManagedLineage {
                source_sha: "d".repeat(40),
                required_source_sha: "e".repeat(40),
                receipt_sha256: "f".repeat(64),
            },
            native: native(),
            harness: native(),
        }
    }
    fn budget() -> UnverifiedBudget {
        UnverifiedBudget {
            issued_utc_ms: 1000,
            run_expires_utc_ms: 181000,
            cleanup_expires_utc_ms: 241000,
            parent_elapsed_ms_at_dispatch: 100,
        }
    }
    fn context() -> UnverifiedHostContext {
        UnverifiedHostContext {
            schema: "humanymous.dotnet-host-launch/2".into(),
            apphost: at("/srv/app/Humanymous.HostPilot"),
            app_root: at("/srv/app"),
            runtime_root: at("/srv/runtime"),
            system_root: None,
            private_work: at("/srv/work"),
            audit_root: at("/srv/work/audit"),
            handoff_root: at("/srv/handoff"),
            actor: ActorIdentity::Linux { uid: 1000 },
            provenance: provenance(),
            budget: budget(),
        }
    }
    struct Frame {
        context: Vec<u8>,
        binding: UnverifiedHostBinding,
        derived: Option<Derivation>,
        nonce: String,
    }
    impl Frame {
        fn new(context: &UnverifiedHostContext) -> Self {
            let bytes = encode_host(context).unwrap();
            let binding = UnverifiedHostBinding {
                schema: "humanymous.parent-context-binding/2".into(),
                run_nonce: NONCE.into(),
                actor: context.actor.clone(),
                source_sha: "9".repeat(40),
                context_sha256: hex_digest(&bytes),
                verified_manifest_sha256: "5".repeat(64),
                provenance_sha256: hex_digest(
                    &parent_context::encode_provenance(&context.provenance).unwrap(),
                ),
                original_budget: budget(),
                preflight_environment: PreflightEnvironment::GithubHosted,
            };
            Self {
                context: bytes,
                binding,
                derived: None,
                nonce: NONCE.into(),
            }
        }
        fn bytes(&self) -> Vec<u8> {
            let decoded = decode_host(&self.context).unwrap();
            encode_envelope(&UnverifiedEnvelope {
                profile: PROFILE,
                run_nonce: self.nonce.clone(),
                derived: self.derived.clone(),
                context: self.context.clone(),
                binding: encode_host_binding(&self.binding, &decoded).unwrap(),
            })
            .unwrap()
        }
    }
    fn facts() -> Facts {
        Facts {
            stdin_pipe: true,
            actor: Some(ActorIdentity::Linux { uid: 1000 }),
            act_marker: false,
            venue: Some("github-hosted"),
            backend_observed: true,
            scope: Some(format!("hm-dotnet-{NONCE}.scope")),
            requires_scope: true,
        }
    }
    fn kind(frame: &[u8], facts: &Facts, wall: u64) -> Option<ErrorKind> {
        admit(frame, facts, wall, Instant::now())
            .err()
            .map(|e| e.kind())
    }

    #[test]
    fn consistent_frame_admits_and_borrows_one_original_budget() {
        let received = Instant::now();
        let admission = admit(&Frame::new(&context()).bytes(), &facts(), 1100, received).unwrap();
        assert_eq!(
            admission.apphost(),
            Path::new(&at("/srv/app/Humanymous.HostPilot"))
        );
        assert_eq!(admission.apphost_root(), Path::new(&at("/srv/app")));
        let (run, cleanup) = admission.deadlines().unwrap();
        assert_eq!(run.duration_since(received), Duration::from_millis(179_900));
        assert_eq!(cleanup.duration_since(run), Duration::from_secs(60));
        // Two sequential profiles see the SAME instants; nothing renews 180 s.
        assert_eq!(admission.deadlines().unwrap(), (run, cleanup));
        let env = admission.fixed_environment();
        assert!(env.iter().all(|(key, _)| !matches!(
            *key,
            "PATH" | "DOTNET_STARTUP_HOOKS" | "HUMANYMOUS_DOTNET_PILOT"
        )));
    }

    #[test]
    fn unsupported_rows_no_pipe_malformed_trailing_oversize_or_backend() {
        let good = Frame::new(&context()).bytes();
        let mut no_pipe = facts();
        no_pipe.stdin_pipe = false;
        assert_eq!(kind(&good, &no_pipe, 1100), Some(ErrorKind::Unsupported));
        let mut trailing = good.clone();
        trailing.push(b'\n');
        let doubled = [good.as_slice(), good.as_slice()].concat();
        for frame in [
            b"".as_slice(),
            b"{}".as_slice(),
            &trailing,
            &doubled,
            &good[..good.len() - 1],
            &vec![b' '; parent_context::admission::MAX_ENVELOPE + 1],
        ] {
            assert_eq!(kind(frame, &facts(), 1100), Some(ErrorKind::Unsupported));
        }
        let mut unqualified = facts();
        unqualified.backend_observed = false;
        assert_eq!(
            kind(&good, &unqualified, 1100),
            Some(ErrorKind::Unsupported)
        );
        for scope in [None, Some("session-1.scope".to_owned())] {
            let mut other = facts();
            other.scope = scope;
            assert_eq!(kind(&good, &other, 1100), Some(ErrorKind::Unsupported));
        }
        let mut windows = facts();
        windows.requires_scope = false;
        windows.scope = None;
        assert!(admit(&good, &windows, 1100, Instant::now()).is_ok());
    }

    #[test]
    fn permission_rows_nonce_digest_provenance_actor_venue_and_cleanup() {
        let mut frame = Frame::new(&context());
        frame.binding.run_nonce = "f".repeat(32);
        assert_eq!(
            kind(&frame.bytes(), &facts(), 1100),
            Some(ErrorKind::PermissionDenied)
        );
        let mut frame = Frame::new(&context());
        frame.binding.context_sha256 = "0".repeat(64);
        assert_eq!(
            kind(&frame.bytes(), &facts(), 1100),
            Some(ErrorKind::PermissionDenied)
        );
        let mut frame = Frame::new(&context());
        let mut lf = parent_context::encode_provenance(&context().provenance).unwrap();
        lf.push(b'\n');
        frame.binding.provenance_sha256 = hex_digest(&lf);
        assert_eq!(
            kind(&frame.bytes(), &facts(), 1100),
            Some(ErrorKind::PermissionDenied)
        );
        let good = Frame::new(&context()).bytes();
        for actor in [
            None,
            Some(ActorIdentity::Linux { uid: 0 }),
            Some(ActorIdentity::Linux { uid: 1001 }),
            Some(ActorIdentity::Windows {
                sid: "S-1-5-21-1-2-3-1000".into(),
            }),
        ] {
            let mut other = facts();
            other.actor = actor;
            assert_eq!(kind(&good, &other, 1100), Some(ErrorKind::PermissionDenied));
        }
        let mut act = facts();
        act.act_marker = true;
        assert_eq!(kind(&good, &act, 1100), Some(ErrorKind::PermissionDenied));
        let mut venue = facts();
        venue.venue = None;
        assert_eq!(kind(&good, &venue, 1100), Some(ErrorKind::PermissionDenied));
        let cleanup =
            parent_context::encode(&UnverifiedParentContext::CleanupOnly(UnverifiedCleanup {
                system_root: "C:/Windows".into(),
                private_work: "C:/work".into(),
                handoff_root: "C:/handoff".into(),
                actor_sid: "S-1-5-21-1-2-3-1001".into(),
                provenance: provenance(),
                budget: UnverifiedBudget {
                    parent_elapsed_ms_at_dispatch: 180000,
                    ..budget()
                },
            }))
            .unwrap();
        let frame = encode_envelope(&UnverifiedEnvelope {
            profile: PROFILE,
            run_nonce: NONCE.into(),
            derived: None,
            context: cleanup,
            binding: b"{}\n".to_vec(),
        })
        .unwrap();
        assert_eq!(
            kind(&frame, &facts(), 181000),
            Some(ErrorKind::PermissionDenied)
        );
    }

    #[test]
    fn apphost_and_derived_narrowing_never_widen_the_parent() {
        let mut outside = context();
        outside.apphost = at("/srv/other/Humanymous.HostPilot");
        assert_eq!(
            kind(&Frame::new(&outside).bytes(), &facts(), 1100),
            Some(ErrorKind::PermissionDenied)
        );
        let mut dotted = context();
        dotted.apphost = at("/srv/app/../work/Humanymous.HostPilot");
        assert_eq!(
            kind(&Frame::new(&dotted).bytes(), &facts(), 1100),
            Some(ErrorKind::PermissionDenied)
        );
        let mut audit = context();
        audit.audit_root = at("/srv/audit");
        assert_eq!(
            kind(&Frame::new(&audit).bytes(), &facts(), 1100),
            Some(ErrorKind::PermissionDenied)
        );
        let derived = |apphost: &str, audit_root: &str| {
            let mut frame = Frame::new(&context());
            frame.derived = Some(Derivation {
                mode: DerivedMode::AotSource,
                apphost: at(apphost),
                audit_root: at(audit_root),
            });
            frame.bytes()
        };
        let narrowed = admit(
            &derived(
                "/srv/work/aot-source/Humanymous.HostPilot",
                "/srv/work/aot-source/audit",
            ),
            &facts(),
            1100,
            Instant::now(),
        )
        .unwrap();
        assert_eq!(narrowed.apphost_root(), Path::new(&at("/srv/work")));
        assert_eq!(
            narrowed.audit_root(),
            Path::new(&at("/srv/work/aot-source/audit"))
        );
        for (apphost, audit_root) in [
            ("/srv/app/Humanymous.HostPilot", "/srv/work/a/audit"),
            ("/srv/work/a/Humanymous.HostPilot", "/srv/handoff/audit"),
            ("/srv/work", "/srv/work/a/audit"),
            ("/srv/work/a/../../x", "/srv/work/a/audit"),
        ] {
            assert_eq!(
                kind(&derived(apphost, audit_root), &facts(), 1100),
                Some(ErrorKind::PermissionDenied)
            );
        }
    }

    #[test]
    fn derive_resends_exact_bytes_narrows_once_and_keeps_the_budget() {
        let received = Instant::now();
        let original = Frame::new(&context());
        let parent = admit(&original.bytes(), &facts(), 1100, received).unwrap();
        let frame = parent
            .derive(
                DerivedMode::JitPacked,
                Path::new(&at("/srv/work/jit-packed/published/Humanymous.HostPilot")),
                Path::new(&at("/srv/work/jit-packed/audit")),
            )
            .unwrap();
        let decoded = decode_envelope(&frame).unwrap();
        assert_eq!(decoded.context, original.context);
        assert_eq!(decoded.run_nonce, NONCE);
        assert_eq!(
            decoded.derived.as_ref().unwrap().mode,
            DerivedMode::JitPacked
        );
        let child = admit(&frame, &facts(), 1100, received).unwrap();
        assert_eq!(child.deadlines().unwrap(), parent.deadlines().unwrap());
        assert_eq!(child.apphost_root(), Path::new(&at("/srv/work")));
        // A derived admission never derives again, and never widens.
        assert_eq!(
            child
                .derive(
                    DerivedMode::AotSource,
                    Path::new(&at("/srv/work/a/x")),
                    Path::new(&at("/srv/work/a/b"))
                )
                .unwrap_err()
                .kind(),
            ErrorKind::PermissionDenied
        );
        for (apphost, audit_root) in [
            ("/srv/app/Humanymous.HostPilot", "/srv/work/a/audit"),
            ("/srv/work/a/x", "/srv/handoff/audit"),
            ("/srv/work/../x", "/srv/work/a/audit"),
            ("/srv/work", "/srv/work/a/audit"),
        ] {
            assert_eq!(
                parent
                    .derive(
                        DerivedMode::AotSource,
                        Path::new(&at(apphost)),
                        Path::new(&at(audit_root))
                    )
                    .unwrap_err()
                    .kind(),
                ErrorKind::PermissionDenied
            );
        }
        // The budget is borrowed, not renewed: a late child still times out.
        assert_eq!(kind(&frame, &facts(), 181000), Some(ErrorKind::TimedOut));
    }

    #[test]
    fn timed_out_rows_exhausted_budget_and_no_renewal() {
        let good = Frame::new(&context()).bytes();
        for wall in [181000, 181100, u64::MAX] {
            assert_eq!(kind(&good, &facts(), wall), Some(ErrorKind::TimedOut));
        }
        let received = Instant::now();
        let late = admit(&good, &facts(), 180_000, received).unwrap();
        let (run, _) = late.deadlines().unwrap();
        assert_eq!(run.duration_since(received), Duration::from_millis(1000));
    }
}

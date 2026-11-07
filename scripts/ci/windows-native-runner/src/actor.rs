//! Quarantined hosted account and local controlled-fixture Job boundaries.
//! Local controlled fixtures use the actual current user, with no account/SCM or
//! privilege operation. Hosted actor credentials never enter argv/env/log/files.
//! SAFETY: the FFI is limited to Win32 account/process/job calls. Every pointer
//! references an owned live UTF16/struct buffer for the synchronous call; handles
//! have one RAII owner, and the suspended child enters a kill-on-close job before
//! any authored workload can execute. No production crate enables unsafe.
#![allow(unsafe_code)]
use crate::{Options, evidence, harness};
use serde_json::json;
use std::{
    ffi::{OsStr, OsString},
    fs, io,
    mem::{size_of, zeroed},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::Command,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
    NetworkManagement::NetManagement::*,
    System::{JobObjects::*, Threading::*},
    UI::WindowsAndMessaging::SW_HIDE,
};
use zeroize::Zeroizing;
mod scm;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Workload {
    Native,
    Workspace,
    Service,
}

fn wide(text: impl AsRef<OsStr>) -> Vec<u16> {
    text.as_ref().encode_wide().chain(Some(0)).collect()
}
fn error(operation: &str, code: u32) -> io::Error {
    io::Error::other(format!("native {operation} failed ({code})"))
}
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

fn create_job() -> io::Result<Handle> {
    let job = Handle(unsafe { CreateJobObjectW(null(), null()) });
    if job.0.is_null() {
        return Err(io::Error::last_os_error());
    }
    Ok(job)
}
fn configure_kill_job(job: &Handle) -> io::Result<()> {
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if unsafe {
        SetInformationJobObject(
            job.0,
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn assign_job(job: &Handle, child: HANDLE) -> io::Result<()> {
    if unsafe { AssignProcessToJobObject(job.0, child) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn terminate_job(job: &Handle) -> io::Result<()> {
    if unsafe { TerminateJobObject(job.0, 1) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn active_job(job: &Handle) -> io::Result<u32> {
    let mut accounting: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { zeroed() };
    if unsafe {
        QueryInformationJobObject(
            job.0,
            JobObjectBasicAccountingInformation,
            (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
            size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
            null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(accounting.ActiveProcesses)
}

/// Current-user cooperative fixture Job. No account or privilege changes.
/// Safe methods borrow the REAL owned std Child; no PID or raw-handle constructor.
pub(crate) struct ControlledFixtureJob(Handle);
impl ControlledFixtureJob {
    pub(crate) fn create() -> io::Result<Self> {
        let job = create_job()?;
        configure_kill_job(&job)?;
        Ok(Self(job))
    }
    pub(crate) fn assign(&self, child: &std::process::Child) -> io::Result<()> {
        use std::os::windows::io::AsRawHandle;
        // Borrow only; std Child remains the sole owner of this process handle.
        assign_job(&self.0, child.as_raw_handle())
    }
    pub(crate) fn terminate(&self) -> io::Result<()> {
        terminate_job(&self.0)
    }
    pub(crate) fn active(&self) -> io::Result<u32> {
        active_job(&self.0)
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum AccountState {
    Uncreated,
    Present,
    Deleted,
}
struct Account {
    name: Vec<u16>,
    state: AccountState,
    delete_on_drop: bool,
}
impl Account {
    fn delete(&mut self) -> io::Result<()> {
        if self.state == AccountState::Present {
            let result = unsafe { NetUserDel(null(), self.name.as_ptr()) };
            if result != 0 {
                return Err(error("account deletion", result));
            }
            self.state = AccountState::Deleted;
        }
        Ok(())
    }
}
impl Drop for Account {
    fn drop(&mut self) {
        if self.delete_on_drop {
            let _ = self.delete();
        }
    }
}
/// Unique account/password owner prepared before the first native mutation.
/// No Debug/Serialize and no mutable resource getter. New retained callers must
/// keep OwnedFailure; legacy adapter below deliberately preserves old teardown.
struct OwnedActor {
    account: Account,
    password: Zeroizing<Vec<u16>>,
}
struct OwnedFailure {
    owner: OwnedActor,
    error: io::Error,
}
impl OwnedActor {
    // Private policy is selected by the native adapter, never an input DTO fact.
    fn prepare(name: &str, password: Zeroizing<Vec<u16>>, delete_on_drop: bool) -> Self {
        Self {
            account: Account {
                name: wide(name),
                state: AccountState::Uncreated,
                delete_on_drop,
            },
            password,
        }
    }
    fn provision(mut self) -> Result<Self, OwnedFailure> {
        if let Err(error) = self.provision_account() {
            return Err(OwnedFailure { owner: self, error });
        }
        Ok(self)
    }
    fn delete_legacy_account(&mut self) -> io::Result<()> {
        if !self.account.delete_on_drop {
            return Err(io::Error::other(
                "retained account requires native cleanup proofs",
            ));
        }
        self.account.delete()
    }
    fn provision_account(&mut self) -> io::Result<()> {
        let account = &mut self.account;
        let password = &self.password;
        let mut info: USER_INFO_1 = unsafe { zeroed() };
        info.usri1_name = account.name.as_mut_ptr();
        info.usri1_password = password.as_ptr().cast_mut();
        info.usri1_priv = USER_PRIV_USER;
        info.usri1_flags = UF_SCRIPT | UF_NORMAL_ACCOUNT | UF_PASSWD_CANT_CHANGE;
        let result =
            unsafe { NetUserAdd(null(), 1, (&info as *const USER_INFO_1).cast(), null_mut()) };
        if result != 0 {
            return Err(error("account creation", result));
        }
        account.state = AccountState::Present;
        let users: windows_permissions::LocalBox<windows_permissions::Sid> =
            "S-1-5-32-545".parse()?;
        let (group, _) = windows_permissions::wrappers::LookupAccountSid(&users)?;
        let group = wide(group);
        let member = LOCALGROUP_MEMBERS_INFO_3 {
            lgrmi3_domainandname: account.name.as_ptr().cast_mut(),
        };
        let status = unsafe {
            NetLocalGroupAddMembers(
                null(),
                group.as_ptr(),
                3,
                (&member as *const LOCALGROUP_MEMBERS_INFO_3).cast(),
                1,
            )
        };
        if status != 0 && status != 1378 {
            return Err(error("Users membership", status));
        }
        let mut buffer = null_mut();
        let mut count = 0;
        let mut total = 0;
        let result = unsafe {
            NetUserGetLocalGroups(
                null(),
                account.name.as_ptr(),
                0,
                LG_INCLUDE_INDIRECT,
                &mut buffer,
                MAX_PREFERRED_LENGTH,
                &mut count,
                &mut total,
            )
        };
        if result != 0 {
            return Err(error("local group enumeration", result));
        }
        // OS-returned array has count entries and is freed only after all borrows.
        let valid = if count == 1 && !buffer.is_null() {
            let entry = unsafe { &*buffer.cast::<LOCALGROUP_USERS_INFO_0>() };
            let mut len = 0;
            while len < 256 && unsafe { *entry.lgrui0_name.add(len) } != 0 {
                len += 1;
            }
            len < 256
                && unsafe { std::slice::from_raw_parts(entry.lgrui0_name, len) }
                    == &group[..group.len() - 1]
        } else {
            false
        };
        unsafe {
            NetApiBufferFree(buffer.cast());
        }
        if !valid {
            return Err(io::Error::other(
                "account local groups must be exactly Users",
            ));
        }
        Ok(())
    }
}
fn account(name: &str, password: Zeroizing<Vec<u16>>) -> io::Result<OwnedActor> {
    // Legacy mode intentionally keeps its existing delete-on-Drop behavior.
    // A prebuilt adapter must select retention before provisioning and must not
    // use this lossy legacy conversion for post-mutation failures.
    OwnedActor::prepare(name, password, true)
        .provision()
        .map_err(|failure| {
            let OwnedFailure { owner, error } = failure;
            drop(owner);
            error
        })
}
fn acl(path: &Path, sid: &str, owner: &str) -> io::Result<()> {
    use windows_permissions::{
        LocalBox, SecurityDescriptor,
        constants::{SeObjectType, SecurityInformation},
        wrappers,
    };
    let descriptor: LocalBox<SecurityDescriptor> =
        format!("D:P(A;OICI;FA;;;{sid})(A;OICI;FA;;;{owner})(A;OICI;FA;;;SY)").parse()?;
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
fn quote(arg: &OsStr) -> io::Result<String> {
    let value = arg
        .to_str()
        .ok_or_else(|| io::Error::other("non-Unicode actor argument"))?;
    if value.contains(['\0', '\r', '\n', '"']) || value.ends_with('\\') {
        return Err(io::Error::other("unsafe actor argument"));
    }
    Ok(format!("\"{value}\""))
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ProcessPhase {
    Created,
    JobCreated,
    Configured,
    Assigned,
    Resumed,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum CleanupTarget {
    OwnedDirectChild,
    OwnedJob,
}
impl ProcessPhase {
    fn cleanup_target(self) -> CleanupTarget {
        match self {
            Self::Created | Self::JobCreated | Self::Configured => CleanupTarget::OwnedDirectChild,
            Self::Assigned | Self::Resumed => CleanupTarget::OwnedJob,
        }
    }
}
#[derive(Clone, Copy)]
struct ExitObservation {
    code: u32,
    natural: bool,
}
/// Existing Handle guards remain the sole native handle owners. No numeric PID
/// constructor, generic executable, copied handle or detached identity exists.
struct OwnedProcessAttempt {
    child: Handle,
    thread: Handle,
    job: Option<Handle>,
    direct_exit: Option<ExitObservation>,
    phase: ProcessPhase,
    force_requested: bool,
    legacy_drop_cleanup: bool,
}
struct OwnedProcessFailure {
    owner: OwnedProcessAttempt,
    error: io::Error,
}
enum StartFailure {
    NotStarted(io::Error),
    Retained(OwnedProcessFailure),
}
/// Borrowing proof keeps the exact native owner alive. Not public/serializable.
struct ConvergenceProof<'a> {
    _owner: &'a OwnedProcessAttempt,
    exit: ExitObservation,
    scope: CleanupTarget,
}
impl OwnedProcessAttempt {
    fn configure_and_resume(&mut self) -> io::Result<()> {
        if self.phase != ProcessPhase::Created || self.job.is_some() {
            return Err(io::Error::other("owned job already configured"));
        }
        self.job = Some(create_job()?);
        let job = self
            .job
            .as_ref()
            .ok_or_else(|| io::Error::other("owned job absent"))?;
        if job.0.is_null() {
            return Err(io::Error::last_os_error());
        }
        self.phase = ProcessPhase::JobCreated;
        configure_kill_job(job)?;
        self.phase = ProcessPhase::Configured;
        assign_job(job, self.child.0)?;
        self.phase = ProcessPhase::Assigned;
        if unsafe { ResumeThread(self.thread.0) } == u32::MAX {
            return Err(io::Error::last_os_error());
        }
        self.phase = ProcessPhase::Resumed;
        Ok(())
    }
    fn wait_run(&mut self, until: std::time::Instant) -> io::Result<ExitObservation> {
        if std::time::Instant::now() >= until {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "standard-user deadline",
            ));
        }
        if let Some(code) = self.direct_exit {
            return Ok(code);
        }
        let remaining = until.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero()
            || unsafe {
                WaitForSingleObject(
                    self.child.0,
                    remaining.as_millis().min(u128::from(u32::MAX)) as u32,
                )
            } != WAIT_OBJECT_0
        {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "standard-user deadline",
            ));
        }
        let mut code = 0;
        if unsafe { GetExitCodeProcess(self.child.0, &mut code) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let exit = ExitObservation {
            code,
            natural: !self.force_requested,
        };
        self.direct_exit = Some(exit);
        if std::time::Instant::now() >= until {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "standard-user deadline",
            ));
        }
        Ok(exit)
    }
    // Every failure borrows self: actual child/thread/optional Job stay owned.
    fn contain(&mut self, until: std::time::Instant) -> io::Result<ConvergenceProof<'_>> {
        if until <= std::time::Instant::now() {
            return Err(io::Error::other("owned cleanup deadline"));
        }
        let scope = self.phase.cleanup_target();
        if scope == CleanupTarget::OwnedDirectChild {
            // Only partial, UNASSIGNED creation/configuration failures use the
            // exact retained child HANDLE. An empty Job cannot contain it.
            self.force_requested = true;
            if unsafe { TerminateProcess(self.child.0, 1) } == 0 {
                return Err(io::Error::last_os_error());
            }
            let exit = self.wait_run(until)?;
            return Ok(ConvergenceProof {
                _owner: self,
                exit,
                scope,
            });
        }
        let job = self
            .job
            .as_ref()
            .ok_or_else(|| io::Error::other("owned job absent"))?;
        if job.0.is_null() {
            return Err(io::Error::other("owned job absent"));
        }
        self.force_requested = true;
        terminate_job(job)?;
        let exit = self.wait_run(until)?;
        loop {
            if std::time::Instant::now() >= until {
                return Err(io::Error::other("owned job cleanup deadline"));
            }
            let active = active_job(
                self.job
                    .as_ref()
                    .ok_or_else(|| io::Error::other("owned job absent"))?,
            )?;
            if std::time::Instant::now() >= until {
                return Err(io::Error::other("owned job cleanup deadline"));
            }
            if active == 0 {
                return Ok(ConvergenceProof {
                    _owner: self,
                    exit,
                    scope,
                });
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
impl Drop for OwnedProcessAttempt {
    fn drop(&mut self) {
        // Preserve legacy failure behavior only. Retained-profile Drop must not
        // renew its cleanup deadline or claim synchronous native cleanup.
        if self.legacy_drop_cleanup || self.direct_exit.is_none() {
            unsafe {
                TerminateProcess(self.child.0, 1);
            }
        }
        if self.legacy_drop_cleanup {
            unsafe {
                WaitForSingleObject(self.child.0, 10000);
            }
        }
        // Exact owned Job closes before process/thread guards, as in old launch.
        drop(self.job.take());
    }
}
fn start_owned_runner(
    account: &Account,
    password: &[u16],
    arguments: &[OsString],
    environment: &[(String, String)],
    cwd: &Path,
    legacy_cleanup: bool,
) -> Result<OwnedProcessAttempt, StartFailure> {
    let prepared = (|| -> io::Result<_> {
        let exe = std::env::current_exe()?.canonicalize()?;
        let application = wide(&exe);
        let text = std::iter::once(exe.into_os_string())
            .chain(arguments.iter().cloned())
            .map(|s| quote(&s))
            .collect::<io::Result<Vec<_>>>()?
            .join(" ");
        let command = wide(text);
        let domain = wide(".");
        let directory = wide(cwd);
        let mut environment: Vec<_> = environment
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        environment.sort_by_key(|s| s.to_ascii_uppercase());
        let environment: Vec<u16> = environment
            .iter()
            .flat_map(|s| s.encode_utf16().chain(Some(0)))
            .chain(Some(0))
            .collect();
        Ok((application, command, domain, directory, environment))
    })()
    .map_err(StartFailure::NotStarted)?;
    let (application, mut command, domain, directory, environment) = prepared;
    let mut startup: STARTUPINFOW = unsafe { zeroed() };
    startup.cb = size_of::<STARTUPINFOW>() as u32;
    startup.dwFlags = STARTF_USESHOWWINDOW;
    startup.wShowWindow = SW_HIDE as u16;
    let mut process: PROCESS_INFORMATION = unsafe { zeroed() };
    let result = unsafe {
        CreateProcessWithLogonW(
            account.name.as_ptr(),
            domain.as_ptr(),
            password.as_ptr(),
            LOGON_WITH_PROFILE,
            application.as_ptr(),
            command.as_mut_ptr(),
            CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW,
            environment.as_ptr().cast(),
            directory.as_ptr(),
            &startup,
            &mut process,
        )
    };
    if result == 0 {
        return Err(StartFailure::NotStarted(io::Error::last_os_error()));
    }
    // Retain the suspended child/thread BEFORE the next fallible Job operation.
    let mut owner = OwnedProcessAttempt {
        child: Handle(process.hProcess),
        thread: Handle(process.hThread),
        job: None,
        direct_exit: None,
        phase: ProcessPhase::Created,
        force_requested: false,
        legacy_drop_cleanup: legacy_cleanup,
    };
    if let Err(error) = owner.configure_and_resume() {
        return Err(StartFailure::Retained(OwnedProcessFailure { owner, error }));
    }
    Ok(owner)
}
fn launch(
    account: &Account,
    password: &[u16],
    arguments: &[OsString],
    environment: &[(String, String)],
    cwd: &Path,
) -> io::Result<(u32, u32)> {
    let mut owner = start_owned_runner(account, password, arguments, environment, cwd, true)
        .map_err(|failure| match failure {
            StartFailure::NotStarted(error) => error,
            StartFailure::Retained(OwnedProcessFailure { owner, error }) => {
                drop(owner);
                error
            }
        })?;
    let natural =
        owner.wait_run(std::time::Instant::now() + std::time::Duration::from_secs(1200))?;
    let proof = owner.contain(std::time::Instant::now() + std::time::Duration::from_secs(5))?;
    if proof.scope != CleanupTarget::OwnedJob || !natural.natural || !proof.exit.natural {
        return Err(io::Error::other(
            "legacy natural exit and assigned Job required",
        ));
    }
    let code = natural.code;
    owner.legacy_drop_cleanup = false;
    Ok((code, 0))
}
pub fn run(options: &Options, workload: Workload) -> io::Result<i32> {
    let service = workload == Workload::Service;
    let workspace = workload == Workload::Workspace;
    if !service && options.actor_case != "normal" {
        return Err(io::Error::other(
            "SCM negative case requires service actor mode",
        ));
    }
    if !evidence::disposable_host() {
        return Err(io::Error::other("disposable GitHub-hosted actor required"));
    }
    let identity = humanymous_runtime::native_security::process::current_identity()?;
    if !identity.elevated() || !identity.enabled_administrator() {
        return Err(io::Error::other(
            "hosted provisioning administrator required",
        ));
    }
    let source = evidence::source(workspace)?;
    let manifest = harness::prepare(
        &options.report,
        &options.binaries,
        if workspace {
            harness::Scope::Workspace
        } else {
            harness::Scope::Native
        },
        &source,
    )?;
    evidence::verify_snapshot(&source)?;
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce).map_err(io::Error::other)?;
    let suffix = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let name = format!("hmrun{}", &suffix[..12]);
    let mut random = Zeroizing::new([0u8; 32]);
    getrandom::fill(&mut *random).map_err(io::Error::other)?;
    let mut secret = Zeroizing::new(String::from("aA1!"));
    {
        use std::fmt::Write;
        for byte in random.iter() {
            write!(&mut *secret, "{byte:02x}").map_err(io::Error::other)?;
        }
    }
    let password = Zeroizing::new(wide(secret.as_str()));
    let mut account = account(&name, password)?;
    let (sid, _, _) =
        windows_permissions::wrappers::LookupAccountName(Option::<&OsStr>::None, &name)?;
    let sid = sid.to_string();
    let base = PathBuf::from(
        std::env::var_os("ProgramData").ok_or_else(|| io::Error::other("ProgramData missing"))?,
    )
    .canonicalize()?;
    let text = base
        .to_str()
        .ok_or_else(|| io::Error::other("non-Unicode ProgramData"))?;
    let text = text.strip_prefix(r"\\?\").unwrap_or(text);
    let bytes = text.as_bytes();
    if bytes.len() < 3 || bytes[1] != b':' || bytes[2] != b'\\' {
        return Err(io::Error::other("local drive-letter ProgramData required"));
    }
    humanymous_runtime::native_security::volume::require_fixed(&format!(
        "{}:\\",
        bytes[0] as char
    ))?;
    let base = PathBuf::from(text);
    let scratch = base.join(&name);
    fs::create_dir(&scratch)?;
    let mut scm_proof = None;
    let mut job_empty_observed = None;
    let mut child_launch_attempted = false;
    let mut recovered_coverage = None;
    let execution = (|| -> io::Result<u32> {
        acl(&scratch, &sid, identity.sid())?;
        let manifest_path = scratch.join("manifest.json");
        let source_path = scratch.join("source.json");
        let report = scratch.join("report.json");
        evidence::write(&manifest_path, &manifest)?;
        evidence::write(&source_path, &source)?;
        let workspace = std::env::current_dir()?.canonicalize()?;
        for (flag, rights) in [
            ("/grant", "(OI)(CI)(RX)"),
            ("/deny", "(OI)(CI)(WD,AD,WEA,WA,DE,DC,WDAC,WO)"),
        ] {
            let status = Command::new("icacls.exe")
                .arg(&workspace)
                .args([flag, &format!("*{sid}:{rights}"), "/T", "/Q", "/L"])
                .stdout(std::process::Stdio::null())
                .status()?;
            if !status.success() {
                return Err(io::Error::other(
                    "account workspace ACL provisioning failed",
                ));
            }
        }
        let system = std::env::var("SystemRoot").map_err(io::Error::other)?;
        let temp = scratch.join("tmp");
        // The child itself creates and owns TEMP; no admin-owned product fixtures.
        let mut environment = vec![
            ("SystemRoot".into(), system.clone()),
            ("WINDIR".into(), system.clone()),
            ("PATH".into(), format!("{system}\\System32;{system}")),
            ("TEMP".into(), temp.to_string_lossy().into_owned()),
            ("TMP".into(), temp.to_string_lossy().into_owned()),
        ];
        if service {
            environment.extend([
                ("GITHUB_ACTIONS".into(), "true".into()),
                ("RUNNER_ENVIRONMENT".into(), "github-hosted".into()),
            ]);
        }
        let arguments = vec![
            match workload {
                Workload::Native => "native".into(),
                Workload::Workspace => "workspace".into(),
                Workload::Service => "service".into(),
            },
            "--report".into(),
            report.clone().into_os_string(),
            "--binaries".into(),
            options.binaries.clone().into_os_string(),
            "--manifest".into(),
            manifest_path.into_os_string(),
            "--source".into(),
            source_path.into_os_string(),
            "--expected-sid".into(),
            sid.clone().into(),
        ];
        let mut delegation = if service {
            Some(scm::Delegation::grant(&sid)?)
        } else {
            None
        };
        let launched = if options.actor_case == "after-grant" {
            Err(io::Error::other("injected failure after SCM grant"))
        } else {
            child_launch_attempted = true;
            launch(
                &account.account,
                &account.password,
                &arguments,
                &environment,
                &workspace,
            )
        };
        job_empty_observed = launched.as_ref().ok().map(|(_, active)| *active == 0);
        let recovery = (|| -> io::Result<()> {
            let coverage_path = report.with_extension("coverage.json");
            if !report.is_file() && coverage_path.is_file() {
                let bytes = evidence::recovery::read(&coverage_path)?;
                fs::write(options.report.with_extension("coverage.json"), &bytes)?;
                recovered_coverage = Some(evidence::recovery::recover(&bytes, &manifest)?);
            }
            Ok(())
        })();
        let copied = if report.is_file() {
            fs::copy(&report, &options.report).map(|_| ())
        } else {
            Err(io::Error::other("standard child report absent"))
        };
        let copied_logs = (|| -> io::Result<()> {
            let harness_count = manifest["harnesses"].as_array().map_or(0, Vec::len);
            for index in 0..harness_count {
                for stream in ["list-stdout", "list-stderr", "run-stdout", "run-stderr"] {
                    let extension = format!("harness-{index}-{stream}.log");
                    let path = report.with_extension(&extension);
                    if path.is_file() {
                        fs::copy(path, options.report.with_extension(extension))?;
                    }
                }
            }
            if service {
                for case in &crate::service::catalog::CASES {
                    for stream in ["list-stdout", "list-stderr", "stdout", "stderr"] {
                        let extension = format!("{}-{stream}.log", case.log_prefix);
                        let path = report.with_extension(&extension);
                        if path.is_file() {
                            fs::copy(path, options.report.with_extension(extension))?;
                        }
                    }
                }
            }
            Ok(())
        })();
        if let Some(guard) = &mut delegation {
            let restored = guard.restore();
            scm_proof = Some(
                json!({"rights":3,"single_generated_sid":true,"original_descriptor_sha256":guard.original_sha256(),"restored":restored.is_ok(),"scope":"SCM manager; service-name restriction is CLI signed policy, not OS ACL"}),
            );
            restored?;
        }
        let (code, _) = launched?;
        recovery?;
        copied?;
        copied_logs?;
        Ok(code)
    })();
    // Closing the job above killed descendants before account deletion/cleanup.
    let deletion = account.delete_legacy_account();
    let cleanup = if scratch.parent() == Some(base.as_path())
        && scratch.file_name() == Some(OsStr::new(&name))
    {
        fs::remove_dir_all(&scratch)
    } else {
        Err(io::Error::other("unsafe generated cleanup path"))
    };
    let mut failures = Vec::new();
    let account_deleted = deletion.is_ok();
    let scratch_deleted = cleanup.is_ok();
    let code = execution.as_ref().ok().copied();
    if let Err(error) = execution {
        failures.push(format!("child execution: {error}"));
    }
    if let Err(error) = deletion {
        failures.push(format!("account deletion: {error}"));
    }
    if let Err(error) = cleanup {
        failures.push(format!("scratch cleanup: {error}"));
    }
    let mut report:serde_json::Value=fs::read(&options.report).ok().and_then(|bytes|serde_json::from_slice(&bytes).ok()).unwrap_or_else(|| {
        let mut failed = json!({"schema":1,"kind":"native","status":"failed","qualification":"component-only","source_sha":source["source_sha"],"source":source,"full_runtime_qualified":false,"errors":0,"skips":0,"cases":[]});
        if workload == Workload::Workspace {
            failed["schema"] = "humanymous.workspace-probe/1".into();
            failed["kind"] = "workspace".into();
            failed["scope"] = "workspace-default-features".into();
            failed["qualification"] = "implemented-workspace-component".into();
        }
        if let Some(coverage) = recovered_coverage {
            failed["coverage"] = coverage;
        }
        failed
    });
    report["standard_user_lifecycle"] = json!({"sid":sid,"account_deleted":account_deleted,"scratch_deleted":scratch_deleted,"child_exit_code":code,"provisioning":"native-Win32","job_kill_on_close":true,"job_empty_observed":job_empty_observed,"scm_delegation_requested":service,"actor_case":options.actor_case});
    report["standard_user_lifecycle"]["child_launch_attempted"] = json!(child_launch_attempted);
    report["standard_user_lifecycle"]["job_kill_on_close"] =
        json!(job_empty_observed.map(|_| true));
    if let Some(proof) = scm_proof {
        report["scm_delegation"] = proof;
    }
    if !failures.is_empty() {
        report["status"] = "failed".into();
        report["errors"] = json!(report["errors"].as_u64().unwrap_or(0) + failures.len() as u64);
        report["lifecycle_errors"] = json!(failures);
    }
    evidence::write(&options.report, &report)?;
    Ok(if report["status"] == "failed" {
        1
    } else {
        code.unwrap_or(1) as i32
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_command_line_is_literal_and_bounded() {
        assert_eq!(
            quote(OsStr::new(r"C:\owned path\runner.exe")).unwrap(),
            r#""C:\owned path\runner.exe""#
        );
        for value in ["a\"b", "a\nb", "a\0b", r"C:\tail\"] {
            assert!(quote(OsStr::new(value)).is_err());
        }
    }
    #[test]
    fn local_provisioning_is_refused_before_any_account_api() {
        for (actions, environment) in [
            (None, None),
            (Some("true"), Some("self-hosted")),
            (Some("false"), Some("github-hosted")),
            (None, Some("github-hosted")),
        ] {
            assert!(!evidence::host_allowed(actions, environment, None));
        }
        assert!(evidence::host_allowed(
            Some("true"),
            Some("github-hosted"),
            None
        ));
    }
}

#[cfg(test)]
mod owner_tests {
    use super::*;
    #[test]
    fn partial_setup_failure_cannot_select_an_empty_unassigned_job() {
        // Failure immediately after each successful native setup operation must
        // select the last actually established boundary, not a present handle.
        for phase in [
            ProcessPhase::Created,
            ProcessPhase::JobCreated,
            ProcessPhase::Configured,
        ] {
            assert!(phase.cleanup_target() == CleanupTarget::OwnedDirectChild);
        }
        for phase in [ProcessPhase::Assigned, ProcessPhase::Resumed] {
            assert!(phase.cleanup_target() == CleanupTarget::OwnedJob);
        }
    }
}

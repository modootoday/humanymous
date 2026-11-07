//! Hosted-only ephemeral account boundary. No password in argv/env/log/files.
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
struct Account {
    name: Vec<u16>,
    deleted: bool,
}
impl Account {
    fn delete(&mut self) -> io::Result<()> {
        if !self.deleted {
            let result = unsafe { NetUserDel(null(), self.name.as_ptr()) };
            if result != 0 {
                return Err(error("account deletion", result));
            }
            self.deleted = true;
        }
        Ok(())
    }
}
impl Drop for Account {
    fn drop(&mut self) {
        let _ = self.delete();
    }
}
fn account(name: &str, password: &[u16]) -> io::Result<Account> {
    let mut name = wide(name);
    let mut info: USER_INFO_1 = unsafe { zeroed() };
    info.usri1_name = name.as_mut_ptr();
    info.usri1_password = password.as_ptr().cast_mut();
    info.usri1_priv = USER_PRIV_USER;
    info.usri1_flags = UF_SCRIPT | UF_NORMAL_ACCOUNT | UF_PASSWD_CANT_CHANGE;
    let result = unsafe { NetUserAdd(null(), 1, (&info as *const USER_INFO_1).cast(), null_mut()) };
    if result != 0 {
        return Err(error("account creation", result));
    }
    let account = Account {
        name,
        deleted: false,
    };
    let users: windows_permissions::LocalBox<windows_permissions::Sid> = "S-1-5-32-545".parse()?;
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
    Ok(account)
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
fn launch(
    account: &Account,
    password: &[u16],
    arguments: &[OsString],
    environment: &[(String, String)],
    cwd: &Path,
) -> io::Result<(u32, u32)> {
    let exe = std::env::current_exe()?.canonicalize()?;
    let application = wide(&exe);
    let text = std::iter::once(exe.into_os_string())
        .chain(arguments.iter().cloned())
        .map(|s| quote(&s))
        .collect::<io::Result<Vec<_>>>()?
        .join(" ");
    let mut command = wide(text);
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
        return Err(io::Error::last_os_error());
    }
    let child = Handle(process.hProcess);
    let thread = Handle(process.hThread);
    let job = Handle(unsafe { CreateJobObjectW(null(), null()) });
    let managed = (|| -> io::Result<(u32, u32)> {
        if job.0.is_null() {
            return Err(io::Error::last_os_error());
        }
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
            || unsafe { AssignProcessToJobObject(job.0, child.0) } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if unsafe { ResumeThread(thread.0) } == u32::MAX {
            return Err(io::Error::last_os_error());
        }
        if unsafe { WaitForSingleObject(child.0, 1_200_000) } != WAIT_OBJECT_0 {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "standard-user deadline",
            ));
        }
        let mut code = 0;
        if unsafe { GetExitCodeProcess(child.0, &mut code) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { TerminateJobObject(job.0, 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
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
            if accounting.ActiveProcesses == 0 {
                return Ok((code, 0));
            }
            if std::time::Instant::now() >= until {
                return Err(io::Error::other(
                    "owned job still contains active processes",
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    })();
    if managed.is_err() {
        unsafe {
            TerminateProcess(child.0, 1);
            WaitForSingleObject(child.0, 10000);
        }
    }
    drop(job);
    managed
}
pub fn run(options: &Options, service: bool) -> io::Result<i32> {
    if !service && options.actor_case != "normal" {
        return Err(io::Error::other(
            "SCM negative case requires service actor mode",
        ));
    }
    if !evidence::disposable_host() {
        return Err(io::Error::other("disposable GitHub-hosted actor required"));
    }
    let identity = humanymous_native_security::process::current_identity()?;
    if !identity.elevated() || !identity.enabled_administrator() {
        return Err(io::Error::other(
            "hosted provisioning administrator required",
        ));
    }
    let source = evidence::source()?;
    let manifest = harness::prepare(&options.report, &options.binaries)?;
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
    let mut account = account(&name, &password)?;
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
    humanymous_native_security::volume::require_fixed(&format!("{}:\\", bytes[0] as char))?;
    let base = PathBuf::from(text);
    let scratch = base.join(&name);
    fs::create_dir(&scratch)?;
    let mut scm_proof = None;
    let mut job_empty_observed = None;
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
            if service {
                "service".into()
            } else {
                "native".into()
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
            launch(&account, &password, &arguments, &environment, &workspace)
        };
        job_empty_observed = launched.as_ref().ok().map(|(_, active)| *active == 0);
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
                for stream in ["service-stdout.log", "service-stderr.log"] {
                    let path = report.with_extension(stream);
                    if path.is_file() {
                        fs::copy(path, options.report.with_extension(stream))?;
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
        copied?;
        copied_logs?;
        Ok(code)
    })();
    // Closing the job above killed descendants before account deletion/cleanup.
    let deletion = account.delete();
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
    let mut report:serde_json::Value=fs::read(&options.report).ok().and_then(|bytes|serde_json::from_slice(&bytes).ok()).unwrap_or_else(||json!({"schema":1,"kind":"native","status":"failed","qualification":"component-only","source_sha":source["source_sha"],"source":source,"full_runtime_qualified":false,"errors":0,"skips":0,"cases":[]}));
    report["standard_user_lifecycle"] = json!({"sid":sid,"account_deleted":account_deleted,"scratch_deleted":scratch_deleted,"child_exit_code":code,"provisioning":"native-Win32","job_kill_on_close":true,"job_empty_observed":job_empty_observed,"scm_delegation_requested":service,"actor_case":options.actor_case});
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
            assert!(!evidence::host_allowed(actions, environment));
        }
        assert!(evidence::host_allowed(Some("true"), Some("github-hosted")));
    }
}

//! UNAPPROVED bodies: exactly ten proposed function-local native scopes.
#![deny(unsafe_code)]

use super::launch::PreparedLaunch;
use std::{ffi::c_void, marker::PhantomData, mem::size_of, os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle}, ptr::{null, null_mut}};
use windows_sys::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::{
        JobObjects::{CreateJobObjectW, IsProcessInJob, QueryInformationJobObject, SetInformationJobObject, TerminateJobObject,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JobObjectExtendedLimitInformation, JobObjectBasicAccountingInformation},
        Threading::{InitializeProcThreadAttributeList, UpdateProcThreadAttribute, DeleteProcThreadAttributeList,
            CreateProcessW, WaitForSingleObject, GetExitCodeProcess, TerminateProcess,
            LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_JOB_LIST, STARTUPINFOEXW, PROCESS_INFORMATION,
            EXTENDED_STARTUPINFO_PRESENT, CREATE_UNICODE_ENVIRONMENT, DETACHED_PROCESS},
    },
};

#[derive(Clone, Copy, Debug)]
pub(super) enum NativeError { Call, Output, AttributeSize }
pub(super) struct Job(OwnedHandle);
pub(super) struct Process {
    process: Option<OwnedHandle>,
    thread: Option<OwnedHandle>,
    pub(super) pid: u32,
}
impl Process {
    pub(super) fn complete(&self) -> bool { self.process.is_some() && self.thread.is_some() && self.pid != 0 }
}
pub(super) enum Query { Policy, Population }
pub(super) enum JobObservation { Policy(u32), Population(u32) }
pub(super) enum ProcessObservation { Pending, Exited(u32) }

#[repr(C, align(16))]
#[derive(Clone)]
struct Aligned([u8; 16]);
pub(super) struct Attributes<'j> {
    pointer: LPPROC_THREAD_ATTRIBUTE_LIST,
    _storage: Box<[Aligned]>,
    _jobs: Box<[HANDLE; 1]>,
    initialized: bool,
    _job: PhantomData<&'j Job>,
}
impl Drop for Attributes<'_> {
    fn drop(&mut self) { drop_job_attribute_list(self); }
}
fn failure() -> NativeError { let _immediate = std::io::Error::last_os_error(); NativeError::Call }

#[allow(unsafe_code)]
pub(super) fn create_job_owned() -> Result<Job, NativeError> {
    // SAFETY: no name/security pointer, new unnamed noninherited owned handle.
    let raw = unsafe { CreateJobObjectW(null(), null()) };
    if raw.is_null() { return Err(failure()); }
    // SAFETY: adopt this exact successful native owner once, with no gap.
    Ok(Job(unsafe { OwnedHandle::from_raw_handle(raw) }))
}

#[allow(unsafe_code)]
pub(super) fn configure_job(job: &Job) -> Result<(), NativeError> {
    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    // SAFETY: fixed valid initialized input, exact class/size, borrowed owned Job.
    let ok = unsafe { SetInformationJobObject(job.0.as_raw_handle(), JobObjectExtendedLimitInformation,
        std::ptr::from_ref(&info).cast(), size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32) };
    if ok == 0 { return Err(failure()); }
    Ok(())
}

#[allow(unsafe_code)]
pub(super) fn query_job(job: &Job, query: Query) -> Result<JobObservation, NativeError> {
    let mut returned = 0;
    match query {
        Query::Policy => {
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            // SAFETY: closed class, initialized correctly sized fixed output.
            let ok = unsafe { QueryInformationJobObject(job.0.as_raw_handle(), JobObjectExtendedLimitInformation,
                std::ptr::from_mut(&mut info).cast(), size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32, &mut returned) };
            if ok == 0 { return Err(failure()); }
            if returned as usize != size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() { return Err(NativeError::Output); }
            Ok(JobObservation::Policy(info.BasicLimitInformation.LimitFlags))
        }
        Query::Population => {
            let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
            // SAFETY: second and only other closed class/output.
            let ok = unsafe { QueryInformationJobObject(job.0.as_raw_handle(), JobObjectBasicAccountingInformation,
                std::ptr::from_mut(&mut info).cast(), size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32, &mut returned) };
            if ok == 0 { return Err(failure()); }
            if returned as usize != size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() { return Err(NativeError::Output); }
            Ok(JobObservation::Population(info.ActiveProcesses))
        }
    }
}

#[allow(unsafe_code)]
pub(super) fn terminate_owned_job(job: &Job) -> Result<(), NativeError> {
    // SAFETY: SAME owned Job only, no completion inferred.
    if unsafe { TerminateJobObject(job.0.as_raw_handle(), 1) } == 0 { return Err(failure()); }
    Ok(())
}

#[allow(unsafe_code)]
pub(super) fn is_owned_process_in_job(job: &Job, process: &Process) -> Result<bool, NativeError> {
    let process = process.process.as_ref().ok_or(NativeError::Output)?;
    let mut inside = 0;
    // SAFETY: both borrowed actual owners, valid BOOL output, no imported PID.
    if unsafe { IsProcessInJob(process.as_raw_handle(), job.0.as_raw_handle(), &mut inside) } == 0 { return Err(failure()); }
    Ok(inside != 0)
}

#[allow(unsafe_code)]
pub(super) fn make_job_attribute_list(job: &Job) -> Result<Attributes<'_>, NativeError> {
    let mut bytes = 0;
    // SAFETY: documented required-size probe, exactly one attribute, flags0.
    let probe = unsafe { InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut bytes) };
    let error = std::io::Error::last_os_error(); // capture BEFORE allocation/API.
    if probe != 0 || error.raw_os_error() != Some(122) || bytes == 0 || bytes > 65536 {
        return Err(NativeError::AttributeSize);
    }
    let blocks = bytes.div_ceil(size_of::<Aligned>());
    let mut storage = vec![Aligned([0; 16]); blocks].into_boxed_slice();
    let jobs = Box::new([job.0.as_raw_handle()]);
    let pointer = storage[0].0.as_mut_ptr().cast::<c_void>();
    let mut owner = Attributes { pointer, _storage: storage, _jobs: jobs, initialized: false, _job: PhantomData };
    let capacity = owner._storage.len() * size_of::<Aligned>();
    let mut provided = capacity;
    // SAFETY: stable16-aligned allocation has at least required bytes; no move of
    // backing allocations, exact one-attribute init. Job borrow outlives Delete.
    if unsafe { InitializeProcThreadAttributeList(owner.pointer, 1, 0, &mut provided) } == 0 { return Err(failure()); }
    owner.initialized = true; // only actual native success enables deletion.
    if provided == 0 || provided > capacity { return Err(NativeError::AttributeSize); }
    // SAFETY: stable boxed actual Job handle array, valid through Create and Delete.
    let ok = unsafe { UpdateProcThreadAttribute(owner.pointer, 0, PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
        owner._jobs.as_ptr().cast(), size_of::<HANDLE>(), null_mut(), null()) };
    if ok == 0 { return Err(failure()); } // Drop deletes initialized list once.
    Ok(owner)
}

#[allow(unsafe_code)]
fn drop_job_attribute_list(attributes: &mut Attributes<'_>) {
    if attributes.initialized {
        attributes.initialized = false;
        // SAFETY: exact successfully initialized list, last use has ended; backing
        // storage, boxed Job array and borrowed Job remain alive through Delete.
        unsafe { DeleteProcThreadAttributeList(attributes.pointer) };
    }
}

pub(super) const FLAGS: u32 = EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT | DETACHED_PROCESS;

#[allow(unsafe_code)]
pub(super) fn create_process_in_job(attributes: &Attributes<'_>, launch: &mut PreparedLaunch) -> Result<Process, NativeError> {
    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.lpAttributeList = attributes.pointer;
    // All default standard handles are NULL and STARTF_USESTDHANDLES is absent.
    let mut info = PROCESS_INFORMATION::default();
    // SAFETY: explicit bounded owned UTF16 strings/environment and initialized
    // structures live throughout synchronous call; stable JOB_LIST backing/job
    // lifetime, inherit=false, exact0x80408/no other attribute or parent/token.
    let ok = unsafe { CreateProcessW(launch.image.as_ptr(), launch.command.as_mut_ptr(), null(), null(), 0,
        FLAGS, launch.environment.as_ptr().cast(), launch.cwd.as_ptr(), &startup.StartupInfo, &mut info) };
    if ok == 0 { return Err(failure()); } // failed outputs are NOT interpreted.
    // SAFETY: immediately adopt each exact successful valid returned owner ONCE.
    // Success contract provides both; anomalous partial output is retained as
    // Options for safe caller cleanup, never filled by reopening its PID.
    let process = if !info.hProcess.is_null() && info.hProcess != INVALID_HANDLE_VALUE {
        Some(unsafe { OwnedHandle::from_raw_handle(info.hProcess) })
    } else { None };
    let thread = if !info.hThread.is_null() && info.hThread != INVALID_HANDLE_VALUE {
        Some(unsafe { OwnedHandle::from_raw_handle(info.hThread) })
    } else { None };
    Ok(Process { process, thread, pid: info.dwProcessId })
}

#[allow(unsafe_code)]
pub(super) fn poll_owned_process(process: &Process) -> Result<ProcessObservation, NativeError> {
    let process = process.process.as_ref().ok_or(NativeError::Output)?;
    // SAFETY: same retained actual handle, zero-time observation only.
    let wait = unsafe { WaitForSingleObject(process.as_raw_handle(), 0) };
    if wait == WAIT_TIMEOUT { return Ok(ProcessObservation::Pending); }
    if wait != WAIT_OBJECT_0 { return Err(failure()); }
    let mut code = 0;
    // SAFETY: only AFTER process signal;259 is a valid actual terminal exit.
    if unsafe { GetExitCodeProcess(process.as_raw_handle(), &mut code) } == 0 { return Err(failure()); }
    Ok(ProcessObservation::Exited(code))
}

#[allow(unsafe_code)]
pub(super) fn terminate_owned_process(process: &Process) -> Result<(), NativeError> {
    let process = process.process.as_ref().ok_or(NativeError::Output)?;
    // SAFETY: SAME returned actual owner, partial-failure cleanup request only.
    if unsafe { TerminateProcess(process.as_raw_handle(), 1) } == 0 { return Err(failure()); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::System::Threading::{CREATE_BREAKAWAY_FROM_JOB, CREATE_NEW_CONSOLE, CREATE_NO_WINDOW, CREATE_SUSPENDED, STARTF_USESTDHANDLES};
    #[test]
    fn exact_detached_profile_has_no_breakaway_console_or_stdio_bits() {
        assert_eq!(FLAGS, 0x80408);
        assert_eq!(FLAGS & (CREATE_BREAKAWAY_FROM_JOB | CREATE_NEW_CONSOLE | CREATE_NO_WINDOW | CREATE_SUSPENDED), 0);
        let startup = STARTUPINFOEXW::default();
        assert_eq!(startup.StartupInfo.dwFlags & STARTF_USESTDHANDLES, 0);
        assert!(startup.StartupInfo.hStdInput.is_null() && startup.StartupInfo.hStdOutput.is_null() && startup.StartupInfo.hStdError.is_null());
        assert_eq!(std::mem::align_of::<Aligned>(), 16);
    }
}

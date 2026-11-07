//! Narrow Win32 inspection boundary; only this pilot's owned child/processes.
use cef::{Browser, ImplBrowser, ImplBrowserHost, Rect};
use meari_native::infrastructure::window::NativeScopedParent;
use std::io;
use windows::Win32::{
    Foundation::*,
    Security::*,
    System::{Diagnostics::ToolHelp::*, Threading::*},
    UI::{HiDpi::*, WindowsAndMessaging::*},
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub fn parent_geometry(parent: &NativeScopedParent<'_>) -> io::Result<(HWND, Rect, u32)> {
    dock_geometry(parent, humanymous_browser::viewport::InspectorDock::Hidden)
}
#[allow(unsafe_code)]
pub fn dock_geometry(
    parent: &NativeScopedParent<'_>,
    dock: humanymous_browser::viewport::InspectorDock,
) -> io::Result<(HWND, Rect, u32)> {
    let RawWindowHandle::Win32(handle) = parent.window_handle().map_err(io::Error::other)?.as_raw()
    else {
        return Err(io::Error::other("Win32 parent required"));
    };
    let hwnd = HWND(handle.hwnd.get() as *mut _);
    let mut rect = RECT::default();
    // SAFETY: scoped, live parent lease on its UI thread; only read its geometry.
    unsafe { GetClientRect(hwnd, &mut rect) }.map_err(io::Error::other)?;
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 {
        return Err(io::Error::other("parent DPI unavailable"));
    }
    let allocation =
        humanymous_browser::viewport::content_bounds(rect.right, rect.bottom, dpi, dock)
            .map_err(io::Error::other)?;
    let bounds = Rect {
        x: allocation.x,
        y: allocation.y,
        width: allocation.width,
        height: allocation.height,
    };
    if bounds.width < 1 || bounds.height < 1 || bounds.width > 16384 || bounds.height > 16384 {
        return Err(io::Error::other(
            "window too small or unbounded for browser child",
        ));
    }
    Ok((hwnd, bounds, dpi))
}
pub fn child_hwnd(browser: &Browser) -> io::Result<HWND> {
    Ok(HWND(
        browser
            .host()
            .ok_or_else(|| io::Error::other("CEF host absent"))?
            .window_handle()
            .0
            .cast(),
    ))
}
#[allow(unsafe_code)]
pub fn prepare_parent(parent: &NativeScopedParent<'_>) -> io::Result<()> {
    crate::window_frame::remove_decorations(parent)
}
#[allow(unsafe_code)]
pub fn validate_parent(browser: &Browser, parent: HWND) -> io::Result<()> {
    let child = child_hwnd(browser)?;
    // SAFETY: a child just created synchronously by this adapter.
    if unsafe { GetParent(child) }.map_err(io::Error::other)? != parent {
        return Err(io::Error::other("observed CEF child parent mismatch"));
    }
    Ok(())
}
#[allow(unsafe_code)]
pub fn resize_child(browser: &Browser, bounds: &Rect) -> io::Result<()> {
    let child = child_hwnd(browser)?;
    // SAFETY: operate only on the CEF child owned by this adapter; no reparenting.
    unsafe {
        SetWindowPos(
            child,
            None,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
    }
    .map_err(io::Error::other)?;
    let mut observed = RECT::default();
    unsafe { GetClientRect(child, &mut observed) }.map_err(io::Error::other)?;
    if observed.right != bounds.width || observed.bottom != bounds.height {
        return Err(io::Error::other("native child resize readback mismatch"));
    }
    Ok(())
}
#[allow(unsafe_code)]
pub fn destroy_child(browser: &Browser) -> io::Result<()> {
    if cef::currently_on(cef::ThreadId::UI) != 1 {
        return Err(io::Error::other("CEF destruction requires its UI thread"));
    }
    // SAFETY: only the internal CEF child window, on its creating thread, after
    // CEF DoClose confirms unload handling. Never destroy the Meari parent here.
    unsafe { DestroyWindow(child_hwnd(browser)?) }.map_err(io::Error::other)
}
#[allow(unsafe_code)]
pub fn focus_observed(browser: &Browser) -> io::Result<bool> {
    let child = child_hwnd(browser)?;
    let thread = unsafe { GetWindowThreadProcessId(child, None) };
    let mut info = GUITHREADINFO {
        cbSize: size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    unsafe { GetGUIThreadInfo(thread, &mut info) }.map_err(io::Error::other)?;
    let focus = info.hwndFocus;
    Ok(focus == child || unsafe { IsChild(child, focus) }.as_bool())
}

#[derive(Clone, PartialEq, Eq, serde::Serialize)]
pub struct InputSnapshot {
    language_id: u16,
    installed_language_ids: Vec<u16>,
    focus_within_child: bool,
    ime_context: bool,
    ime_open: bool,
    conversion_mode: Option<u32>,
    composition_bytes: i32,
    candidate_style: Option<u32>,
    candidate_position: Option<[i32; 2]>,
}

/// Inspect only this child's input context; composition text is never read.
#[allow(unsafe_code)]
pub fn input_snapshot(browser: &Browser) -> io::Result<InputSnapshot> {
    use windows::Win32::UI::Input::{Ime::*, KeyboardAndMouse::*};
    let child = child_hwnd(browser)?;
    let thread = unsafe { GetWindowThreadProcessId(child, None) };
    let mut info = GUITHREADINFO {
        cbSize: size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    let mut layouts = [HKL::default(); 16];
    // SAFETY: only read system layout metadata and our own child thread/window.
    unsafe {
        GetGUIThreadInfo(thread, &mut info).map_err(io::Error::other)?;
        let count = GetKeyboardLayoutList(Some(&mut layouts)).clamp(0, 16) as usize;
        let mut snapshot = InputSnapshot {
            language_id: (GetKeyboardLayout(thread).0 as usize & 0xffff) as u16,
            installed_language_ids: layouts[..count]
                .iter()
                .map(|h| (h.0 as usize & 0xffff) as u16)
                .collect(),
            focus_within_child: info.hwndFocus == child || IsChild(child, info.hwndFocus).as_bool(),
            ime_context: false,
            ime_open: false,
            conversion_mode: None,
            composition_bytes: -1,
            candidate_style: None,
            candidate_position: None,
        };
        if snapshot.focus_within_child {
            let context = ImmGetContext(info.hwndFocus);
            if !context.0.is_null() {
                snapshot.ime_context = true;
                snapshot.ime_open = ImmGetOpenStatus(context).as_bool();
                let mut conversion = IME_CONVERSION_MODE::default();
                if ImmGetConversionStatus(context, Some(&mut conversion), None).as_bool() {
                    snapshot.conversion_mode = Some(conversion.0);
                }
                snapshot.composition_bytes =
                    ImmGetCompositionStringW(context, GCS_COMPSTR, None, 0);
                let mut form = CANDIDATEFORM::default();
                if ImmGetCandidateWindow(context, 0, &mut form).as_bool() {
                    snapshot.candidate_style = Some(form.dwStyle);
                    snapshot.candidate_position = Some([form.ptCurrentPos.x, form.ptCurrentPos.y]);
                }
                let _ = ImmReleaseContext(info.hwndFocus, context);
            }
        }
        Ok(snapshot)
    }
}

/// Opt-in pilot setup on CEF's UI thread, using an already installed Korean IME.
/// No layout installation, global/process-wide flag, settings or text injection.
#[allow(unsafe_code)]
pub fn prepare_korean_ime(browser: &Browser) -> io::Result<()> {
    use windows::Win32::UI::Input::{Ime::*, KeyboardAndMouse::*};
    if cef::currently_on(cef::ThreadId::UI) != 1 {
        return Err(io::Error::other("IME setup requires CEF UI thread"));
    }
    let child = child_hwnd(browser)?;
    let mut layouts = [HKL::default(); 16];
    // SAFETY: select only an existing layout for the calling owned UI thread.
    unsafe {
        let count = GetKeyboardLayoutList(Some(&mut layouts)).clamp(0, 16) as usize;
        let layout = layouts[..count]
            .iter()
            .find(|h| h.0 as usize & 0xffff == 0x412)
            .ok_or_else(|| io::Error::other("Korean IME not installed"))?;
        ActivateKeyboardLayout(*layout, ACTIVATE_KEYBOARD_LAYOUT_FLAGS(0))
            .map_err(io::Error::other)?;
        let mut info = GUITHREADINFO {
            cbSize: size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        GetGUIThreadInfo(GetWindowThreadProcessId(child, None), &mut info)
            .map_err(io::Error::other)?;
        if info.hwndFocus != child && !IsChild(child, info.hwndFocus).as_bool() {
            return Err(io::Error::other("IME focus outside owned child"));
        }
        let context = ImmGetContext(info.hwndFocus);
        if context.0.is_null() {
            return Err(io::Error::other("IME context unavailable"));
        }
        let opened = ImmSetOpenStatus(context, true).as_bool();
        let mut conversion = IME_CONVERSION_MODE::default();
        let mut sentence = IME_SENTENCE_MODE::default();
        let converted = ImmGetConversionStatus(context, Some(&mut conversion), Some(&mut sentence))
            .as_bool()
            && ImmSetConversionStatus(
                context,
                IME_CONVERSION_MODE(conversion.0 | IME_CMODE_NATIVE.0),
                sentence,
            )
            .as_bool();
        let _ = ImmReleaseContext(info.hwndFocus, context);
        if !opened || !converted {
            return Err(io::Error::other("IME open request rejected"));
        }
    }
    Ok(())
}

/// Ask the real IME to convert/open candidates; a successful request is not proof
/// that a candidate window became visible or was placed correctly.
#[allow(unsafe_code)]
pub fn request_ime_candidates(browser: &Browser) -> io::Result<String> {
    use windows::Win32::UI::Input::Ime::*;
    if cef::currently_on(cef::ThreadId::UI) != 1 {
        return Err(io::Error::other("candidate request requires CEF UI thread"));
    }
    let child = child_hwnd(browser)?;
    let mut info = GUITHREADINFO {
        cbSize: size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: context belongs to the owned focused child; release on all paths.
    unsafe {
        GetGUIThreadInfo(GetWindowThreadProcessId(child, None), &mut info)
            .map_err(io::Error::other)?;
        if info.hwndFocus != child && !IsChild(child, info.hwndFocus).as_bool() {
            return Err(io::Error::other("candidate focus outside child"));
        }
        let context = ImmGetContext(info.hwndFocus);
        if context.0.is_null() {
            return Err(io::Error::other("candidate context unavailable"));
        }
        let convert = ImmNotifyIME(context, NI_COMPOSITIONSTR, CPS_CONVERT, 0).as_bool();
        let opened = ImmNotifyIME(context, NI_OPENCANDIDATE, NOTIFY_IME_INDEX(0), 0).as_bool();
        let _ = ImmReleaseContext(info.hwndFocus, context);
        Ok(format!(
            "conversion_request={convert}; open_request={opened}; visible_window_unqualified"
        ))
    }
}

#[derive(Clone, serde::Serialize)]
pub struct ProcessToken {
    pub pid: u32,
    pub parent: u32,
    pub integrity_rid: u32,
    pub restricted: bool,
    pub app_container: bool,
    pub role: Option<String>,
    pub role_error: Option<i32>,
}

#[allow(unsafe_code)]
pub fn owned_process_tokens() -> io::Result<Vec<ProcessToken>> {
    // SAFETY: snapshot only process IDs/names; open tokens only for descendants
    // whose executable name matches this pilot. No other process memory/profile.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(io::Error::other)?;
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut rows = Vec::new();
        let own_name = std::env::current_exe()?
            .file_name()
            .ok_or_else(|| io::Error::other("no filename"))?
            .to_string_lossy()
            .to_lowercase();
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|v| *v == 0)
                    .unwrap_or(entry.szExeFile.len());
                if String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase() == own_name {
                    rows.push((entry.th32ProcessID, entry.th32ParentProcessID));
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
        let mut owned = vec![std::process::id()];
        loop {
            let before = owned.len();
            for (pid, parent) in &rows {
                if owned.contains(parent) && !owned.contains(pid) {
                    owned.push(*pid);
                }
            }
            if before == owned.len() {
                break;
            }
        }
        let mut tokens = Vec::new();
        for (pid, parent) in rows.into_iter().filter(|(pid, _)| owned.contains(pid)) {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
                .map_err(io::Error::other)?;
            let mut image = vec![0u16; 32768];
            let mut image_length = image.len() as u32;
            let checked = QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(image.as_mut_ptr()),
                &mut image_length,
            );
            let image_matches = checked.is_ok()
                && String::from_utf16_lossy(&image[..image_length as usize])
                    .eq_ignore_ascii_case(&std::env::current_exe()?.to_string_lossy());
            if !image_matches {
                let _ = CloseHandle(process);
                return Err(io::Error::other(
                    "owned process identity changed or could not be read",
                ));
            }
            let mut token = HANDLE::default();
            let role = process_role(process, pid == std::process::id());
            let opened = OpenProcessToken(process, TOKEN_QUERY, &mut token);
            let _ = CloseHandle(process);
            opened.map_err(io::Error::other)?;
            let result = read_token(token, pid, parent);
            let _ = CloseHandle(token);
            let mut info = result?;
            match role {
                Ok(role) => info.role = Some(role),
                Err(error) => info.role_error = Some(error),
            }
            tokens.push(info);
        }
        Ok(tokens)
    }
}

#[allow(unsafe_code)]
fn read_token(token: HANDLE, pid: u32, parent: u32) -> io::Result<ProcessToken> {
    // SAFETY: token is a live TOKEN_QUERY handle, buffer usize alignment is
    // sufficient for TOKEN_MANDATORY_LABEL and its embedded SID pointer.
    unsafe {
        let mut needed = 0;
        let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut needed);
        if needed == 0 || needed > 65536 {
            return Err(io::Error::other("invalid token size"));
        }
        let mut buffer = vec![0usize; (needed as usize).div_ceil(size_of::<usize>())];
        GetTokenInformation(
            token,
            TokenIntegrityLevel,
            Some(buffer.as_mut_ptr().cast()),
            needed,
            &mut needed,
        )
        .map_err(io::Error::other)?;
        let label = &*buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>();
        let count = *GetSidSubAuthorityCount(label.Label.Sid);
        if count == 0 {
            return Err(io::Error::other("empty token integrity SID"));
        }
        let rid = *GetSidSubAuthority(label.Label.Sid, u32::from(count - 1));
        let mut app_container = 0u32;
        GetTokenInformation(
            token,
            TokenIsAppContainer,
            Some((&mut app_container as *mut u32).cast()),
            4,
            &mut needed,
        )
        .map_err(io::Error::other)?;
        SetLastError(ERROR_SUCCESS);
        let restricted = IsTokenRestricted(token).is_ok();
        if !restricted && GetLastError() != ERROR_SUCCESS {
            return Err(io::Error::last_os_error());
        }
        Ok(ProcessToken {
            pid,
            parent,
            integrity_rid: rid,
            restricted,
            app_container: app_container != 0,
            role: None,
            role_error: None,
        })
    }
}

#[allow(unsafe_code)]
fn process_role(process: HANDLE, is_browser: bool) -> Result<String, i32> {
    use windows::Wdk::System::Threading::{
        NtQueryInformationProcess, ProcessCommandLineInformation,
    };
    // SAFETY: process is already constrained to this pilot's descendants. Query
    // bounded OS-provided command-line bytes, validate the in-buffer UNICODE_STRING
    // before reading. Only retain --type, never the command line/profile path.
    unsafe {
        let mut needed = 0;
        let _ = NtQueryInformationProcess(
            process,
            ProcessCommandLineInformation,
            std::ptr::null_mut(),
            0,
            &mut needed,
        );
        if needed < size_of::<UNICODE_STRING>() as u32 || needed > 65536 {
            return Err(-1);
        }
        let mut buffer = vec![0usize; (needed as usize).div_ceil(size_of::<usize>())];
        let status = NtQueryInformationProcess(
            process,
            ProcessCommandLineInformation,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        );
        if status.0 < 0 {
            return Err(status.0);
        }
        let string = &*buffer.as_ptr().cast::<UNICODE_STRING>();
        let start = buffer.as_ptr() as usize;
        let data = string.Buffer.0 as usize;
        let length = usize::from(string.Length);
        if string.Length > string.MaximumLength
            || !unicode_in_buffer(start, buffer.len() * size_of::<usize>(), data, length)
        {
            return Err(-3);
        }
        let command = String::from_utf16(std::slice::from_raw_parts(string.Buffer.0, length / 2))
            .map_err(|_| -4)?;
        parse_role(&command, is_browser)
    }
}

fn unicode_in_buffer(start: usize, bytes: usize, data: usize, length: usize) -> bool {
    length.is_multiple_of(2)
        && data.is_multiple_of(2)
        && data >= start
        && start
            .checked_add(bytes)
            .zip(data.checked_add(length))
            .is_some_and(|(end, last)| last <= end)
}
fn parse_role(command: &str, is_browser: bool) -> Result<String, i32> {
    let roles: Vec<_> = command
        .split_whitespace()
        .map(|part| part.trim_matches('"'))
        .filter_map(|part| part.strip_prefix("--type="))
        .collect();
    match roles.as_slice() {
        [] if is_browser => Ok("browser".into()),
        [role]
            if !role.is_empty()
                && role.len() <= 64
                && role.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') =>
        {
            Ok((*role).into())
        }
        _ => Err(-5),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_unicode_header_cannot_escape_the_returned_allocation() {
        assert!(unicode_in_buffer(1000, 64, 1016, 48));
        for (data, length) in [
            (999, 2),
            (1001, 2),
            (1016, 49),
            (1064, 2),
            (usize::MAX - 1, 8),
        ] {
            assert!(!unicode_in_buffer(1000, 64, data, length));
        }
        assert!(!unicode_in_buffer(usize::MAX - 3, 8, usize::MAX - 1, 2));
    }
    #[test]
    fn only_unambiguous_owned_process_roles_are_retained() {
        assert_eq!(
            parse_role("pilot.exe --type=renderer --synthetic", false).unwrap(),
            "renderer"
        );
        assert_eq!(
            parse_role("pilot.exe \"--type=gpu-process\"", false).unwrap(),
            "gpu-process"
        );
        assert_eq!(parse_role("pilot.exe", true).unwrap(), "browser");
        for command in [
            "pilot.exe",
            "--type=",
            "--type=renderer --type=utility",
            "--type=renderer;bad",
        ] {
            assert!(parse_role(command, false).is_err());
        }
    }
}

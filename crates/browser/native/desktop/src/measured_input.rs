//! User-authorized measured Hanja replay, confined to the foreground pilot.
use cef::Browser;
use std::io;
use windows::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

fn measured_pair() -> [KEYBDINPUT; 2] {
    let down = KEYBDINPUT {
        wVk: VK_HANJA,
        wScan: 0xf1,
        dwFlags: KEYEVENTF_EXTENDEDKEY,
        ..Default::default()
    };
    [
        down,
        KEYBDINPUT {
            dwFlags: down.dwFlags | KEYEVENTF_KEYUP,
            ..down
        },
    ]
}

fn plain_modifiers(flags: u32) -> bool {
    use cef::sys::cef_event_flags_t as F;
    let mask = F::EVENTFLAG_SHIFT_DOWN.0
        | F::EVENTFLAG_CONTROL_DOWN.0
        | F::EVENTFLAG_ALT_DOWN.0
        | F::EVENTFLAG_COMMAND_DOWN.0;
    flags & mask as u32 == 0
}

#[derive(Clone, PartialEq, Eq, serde::Serialize)]
pub struct Surface {
    class_name: String,
    screen_rect: [i32; 4],
    same_process: bool,
    ime_class_match: bool,
}

fn ime_class(name: &str) -> bool {
    name.to_ascii_lowercase().starts_with("mscandui")
        || matches!(
            name,
            "Microsoft.IME.CandidateWindow.View" | "Microsoft.IME.UIManager.CandidateWindow.Host"
        )
}

/// Own/owned-popup geometry plus known IME classes while this pilot is foreground.
/// IME class matching is a correlation cue, not proof of ownership or visibility
/// of a particular candidate. Never read any foreign window title or text.
#[allow(unsafe_code)]
pub fn owned_surfaces() -> io::Result<Vec<Surface>> {
    #[allow(unsafe_code)]
    unsafe extern "system" fn visit(hwnd: HWND, storage: LPARAM) -> windows::core::BOOL {
        // SAFETY: synchronous EnumWindows callback, borrowing the caller's live
        // Vec. Read only geometry/classes after bounding ownership attribution.
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() {
                return TRUE;
            }
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let same_process = pid == std::process::id();
            let mut attributed = same_process;
            let mut owner = hwnd;
            for _ in 0..8 {
                if attributed {
                    break;
                }
                let Ok(next) = GetWindow(owner, GW_OWNER) else {
                    break;
                };
                if next.0.is_null() || next == owner {
                    break;
                }
                GetWindowThreadProcessId(next, Some(&mut pid));
                attributed = pid == std::process::id();
                owner = next;
            }
            let mut name = [0u16; 128];
            let count = GetClassNameW(hwnd, &mut name).clamp(0, 128) as usize;
            let class_name = String::from_utf16_lossy(&name[..count]);
            let ime_class_match = ime_class(&class_name);
            if !attributed && !ime_class_match {
                return TRUE;
            }
            let surfaces = &mut *(storage.0 as *mut Vec<Surface>);
            if surfaces.len() >= 32 {
                return TRUE;
            }
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_ok() {
                surfaces.push(Surface {
                    class_name,
                    screen_rect: [rect.left, rect.top, rect.right, rect.bottom],
                    same_process,
                    ime_class_match,
                });
            }
            TRUE
        }
    }
    let mut surfaces = Vec::<Surface>::new();
    let mut foreground_pid = 0;
    unsafe {
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut foreground_pid));
    }
    if foreground_pid != std::process::id() {
        return Ok(surfaces);
    }
    // SAFETY: callback is synchronous; the storage pointer never escapes.
    unsafe {
        EnumWindows(
            Some(visit),
            LPARAM((&mut surfaces as *mut Vec<Surface>) as isize),
        )
    }
    .map_err(io::Error::other)?;
    Ok(surfaces)
}

/// Reproduce the user's VK19/scanF1/extended observation as an OS key pair.
/// This is opt-in test input, not a CEF key message, Unicode commit or broker.
#[allow(unsafe_code)]
pub fn replay_hanja(browser: &Browser, cef_modifiers: u32) -> io::Result<String> {
    if !plain_modifiers(cef_modifiers) {
        return Err(io::Error::other(
            "modifier in trigger event; measured input refused",
        ));
    }
    if cef::currently_on(cef::ThreadId::UI) != 1 {
        return Err(io::Error::other("measured input requires CEF UI thread"));
    }
    let child = crate::native::child_hwnd(browser)?;
    let mut info = GUITHREADINFO {
        cbSize: size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: only query the current foreground/owned child. Never change
    // foreground, attach foreign input queues, elevate or install global hooks.
    unsafe {
        let mut foreground_pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut foreground_pid));
        if foreground_pid != std::process::id() {
            return Err(io::Error::other(
                "foreground is outside pilot; input refused",
            ));
        }
        GetGUIThreadInfo(GetWindowThreadProcessId(child, None), &mut info)
            .map_err(io::Error::other)?;
        if info.hwndFocus != child && !IsChild(child, info.hwndFocus).as_bool() {
            return Err(io::Error::other(
                "focus is outside CEF child; input refused",
            ));
        }
        for modifier in [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN] {
            if GetAsyncKeyState(i32::from(modifier.0)) < 0 {
                return Err(io::Error::other("modifier held; measured input refused"));
            }
        }
        let packets = measured_pair().map(|ki| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki },
        });
        let inserted = SendInput(&packets, size_of::<INPUT>() as i32);
        if inserted != 2 {
            return Err(io::Error::other(format!(
                "measured input inserted {inserted}/2; OS error {}",
                GetLastError().0
            )));
        }
        Ok("VK=0x19 scan=0xF1 extended=true; OS inserted2/2; candidate display unqualified".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measured_pair_preserves_vk_scan_extended_and_releases_the_key() {
        let [down, up] = measured_pair();
        assert_eq!((down.wVk.0, down.wScan), (0x19, 0xf1));
        assert_eq!((up.wVk, up.wScan), (down.wVk, down.wScan));
        assert_eq!(down.dwFlags, KEYEVENTF_EXTENDEDKEY);
        assert_eq!(up.dwFlags, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP);
        assert_eq!(
            down.dwFlags.0 & (KEYEVENTF_UNICODE | KEYEVENTF_SCANCODE).0,
            0
        );
    }
    #[test]
    fn trigger_modifiers_are_refused_even_after_os_key_release() {
        assert!(plain_modifiers(0));
        for bit in [2, 4, 8, 128] {
            assert!(!plain_modifiers(bit));
        }
        assert!(plain_modifiers(1)); // Caps-lock state is not a held modifier.
    }
    #[test]
    fn ime_class_cues_are_bounded_and_do_not_match_arbitrary_windows() {
        assert!(ime_class("Microsoft.IME.CandidateWindow.View"));
        assert!(ime_class("MSCandUIWindow_Candidate"));
        for name in [
            "Window Class",
            "Notepad",
            "Candidate",
            "Microsoft.IME.unrelated",
        ] {
            assert!(!ime_class(name));
        }
    }
}

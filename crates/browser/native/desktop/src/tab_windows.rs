//! Owned CEF child visibility and focus; no foreign windows or thread attachment.
use crate::child::Shared;
use cef::*;
use std::io;
use windows::Win32::UI::WindowsAndMessaging::*;

#[allow(unsafe_code)]
pub fn visible(browser: &Browser, visible: bool) -> io::Result<()> {
    let child = crate::native::child_hwnd(browser)?;
    // SAFETY: this handle comes from the current owned CEF Browser; do not reparent.
    unsafe {
        SetWindowPos(
            child,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE
                | SWP_NOSIZE
                | SWP_NOZORDER
                | SWP_NOACTIVATE
                | if visible {
                    SWP_SHOWWINDOW
                } else {
                    SWP_HIDEWINDOW
                },
        )
    }
    .map_err(io::Error::other)?;
    let shown = unsafe { GetWindowLongPtrW(child, GWL_STYLE) } & WS_VISIBLE.0 as isize != 0;
    if shown != visible {
        return Err(io::Error::other("tab visibility readback mismatch"));
    }
    Ok(())
}
#[allow(unsafe_code)]
pub fn focus_address(
    parent: &meari_native::infrastructure::window::NativeScopedParent<'_>,
) -> io::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetFocus, SetFocus};
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let RawWindowHandle::Win32(handle) = parent.window_handle().map_err(io::Error::other)?.as_raw()
    else {
        return Err(io::Error::other("Win32 parent required"));
    };
    // SAFETY: live scoped Meari host on its own UI thread, never a foreign HWND.
    let hwnd = windows::Win32::Foundation::HWND(handle.hwnd.get() as *mut _);
    let _ = unsafe { SetFocus(Some(hwnd)) };
    if unsafe { GetFocus() } != hwnd {
        return Err(io::Error::other("address native focus readback failed"));
    }
    Ok(())
}
wrap_task! {
    struct Focus { browser:Browser,state:Shared,focused:bool, }
    impl Task {
        fn execute(&self) {
            if !self.state.lock().is_ok_and(|s|!s.closed && (!self.focused || (s.is_live() && s.active))) { return; }
            if let Some(host)=self.browser.host() { host.set_focus(i32::from(self.focused)); }
        }
    }
}
pub fn focus(browser: Browser, state: Shared, focused: bool) -> io::Result<()> {
    let mut task = Focus::new(browser, state, focused);
    if post_task(ThreadId::UI, Some(&mut task)) != 1 {
        return Err(io::Error::other("tab focus UI task rejected"));
    }
    Ok(())
}

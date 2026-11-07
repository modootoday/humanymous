//! Owned-window chrome only. No global hooks or target-page control bridge.
use meari_native::infrastructure::window::NativeScopedParent;
use std::io;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

fn hwnd(parent: &NativeScopedParent<'_>) -> io::Result<HWND> {
    let RawWindowHandle::Win32(handle) = parent.window_handle().map_err(io::Error::other)?.as_raw()
    else {
        return Err(io::Error::other("Win32 parent required"));
    };
    Ok(HWND(handle.hwnd.get() as *mut _))
}

#[allow(unsafe_code)]
pub fn remove_decorations(parent: &NativeScopedParent<'_>) -> io::Result<()> {
    let hwnd = hwnd(parent)?;
    // SAFETY: live scoped Meari parent on its UI thread. Retain system menu and
    // minimize/maximize capabilities; do not alter CEF or any foreign window.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        SetLastError(ERROR_SUCCESS);
        SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            (style | WS_CLIPCHILDREN.0 as isize) & !((WS_CAPTION | WS_THICKFRAME).0 as isize),
        );
        if GetLastError() != ERROR_SUCCESS {
            return Err(io::Error::last_os_error());
        }
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
        .map_err(io::Error::other)?;
        if GetWindowLongPtrW(hwnd, GWL_STYLE) & ((WS_CAPTION | WS_THICKFRAME).0 as isize) != 0 {
            return Err(io::Error::other(
                "native decoration removal readback failed",
            ));
        }
    }
    Ok(())
}

#[allow(unsafe_code)]
pub fn apply(
    parent: &NativeScopedParent<'_>,
    command: humanymous_browser::window_chrome::WindowCommand,
) -> io::Result<()> {
    use humanymous_browser::window_chrome::WindowCommand;
    let hwnd = hwnd(parent)?;
    // SAFETY: UI-thread access to this host only. Close uses lifecycle wake instead.
    unsafe {
        let show = match command {
            WindowCommand::Minimize => SW_MINIMIZE,
            WindowCommand::ToggleMaximize if IsZoomed(hwnd).as_bool() => SW_RESTORE,
            WindowCommand::ToggleMaximize => SW_MAXIMIZE,
            WindowCommand::Close => {
                return Err(io::Error::other("close requires lifecycle coordinator"));
            }
        };
        let _ = ShowWindow(hwnd, show);
    }
    Ok(())
}

/// UI-thread drag state stores geometry, never an HWND or foreign owner.
pub struct Drag {
    cursor: POINT,
    rect: RECT,
    edge: Option<humanymous_browser::frame_geometry::Edge>,
}

#[allow(unsafe_code)]
pub fn pointer_event(
    parent: &NativeScopedParent<'_>,
    event: &winit::event::WindowEvent,
    drag: &mut Option<Drag>,
) -> io::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
    use winit::event::{ButtonSource, ElementState, MouseButton, WindowEvent};
    let hwnd = hwnd(parent)?;
    // SAFETY: scoped live host on its UI thread. Capture only starts in blank
    // chrome; no global hook. Every move rechecks current owned capture. Release
    // on button-up/focus loss; no HWND survives this callback or enters drag state.
    unsafe {
        match event {
            WindowEvent::PointerButton {
                state: ElementState::Pressed,
                button: ButtonSource::Mouse(MouseButton::Left),
                position,
                ..
            } => {
                let mut client = RECT::default();
                GetClientRect(hwnd, &mut client).map_err(io::Error::other)?;
                let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd) as f64 / 96.0;
                let edge = humanymous_browser::frame_geometry::hit(
                    client.right,
                    client.bottom,
                    (dpi * 96.0) as u32,
                    position.x,
                    position.y,
                );
                let title = position.x >= 330.0 * dpi
                    && position.x < f64::from(client.right) - 150.0 * dpi
                    && position.y >= 0.0
                    && position.y < 34.0 * dpi;
                if !IsZoomed(hwnd).as_bool() && (edge.is_some() || title) {
                    let mut start = Drag {
                        cursor: POINT::default(),
                        rect: RECT::default(),
                        edge,
                    };
                    GetWindowRect(hwnd, &mut start.rect).map_err(io::Error::other)?;
                    // Use the event's press point, not a later global cursor sample:
                    // queued/fast drags may already have reached their endpoint.
                    start.cursor = POINT {
                        x: start.rect.left.saturating_add(position.x.round() as i32),
                        y: start.rect.top.saturating_add(position.y.round() as i32),
                    };
                    SetCapture(hwnd);
                    if GetCapture() == hwnd {
                        *drag = Some(start);
                    }
                }
            }
            WindowEvent::PointerMoved { position, .. } => {
                let mut client = RECT::default();
                GetClientRect(hwnd, &mut client).map_err(io::Error::other)?;
                let maximized = IsZoomed(hwnd).as_bool();
                let edge = humanymous_browser::frame_geometry::hit(
                    client.right,
                    client.bottom,
                    windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd),
                    position.x,
                    position.y,
                );
                if let Some(edge) = drag.as_ref().and_then(|d| d.edge).or(edge) {
                    SetCursor(Some(
                        LoadCursorW(None, cursor_for(edge, maximized)).map_err(io::Error::other)?,
                    ));
                }
                if GetCapture() != hwnd {
                    *drag = None;
                }
                if let Some(start) = drag {
                    let mut cursor = POINT::default();
                    GetCursorPos(&mut cursor).map_err(io::Error::other)?;
                    let dx = cursor.x.saturating_sub(start.cursor.x);
                    let dy = cursor.y.saturating_sub(start.cursor.y);
                    let original = [
                        start.rect.left,
                        start.rect.top,
                        start.rect.right,
                        start.rect.bottom,
                    ];
                    let rect = match start.edge {
                        Some(edge) => humanymous_browser::frame_geometry::resize(
                            original,
                            edge,
                            dx,
                            dy,
                            windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd),
                        )
                        .map_err(io::Error::other)?,
                        None => [
                            original[0].saturating_add(dx),
                            original[1].saturating_add(dy),
                            original[2].saturating_add(dx),
                            original[3].saturating_add(dy),
                        ],
                    };
                    SetWindowPos(
                        hwnd,
                        None,
                        rect[0],
                        rect[1],
                        rect[2] - rect[0],
                        rect[3] - rect[1],
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    )
                    .map_err(io::Error::other)?;
                }
            }
            WindowEvent::PointerButton {
                state: ElementState::Released,
                button: ButtonSource::Mouse(MouseButton::Left),
                ..
            }
            | WindowEvent::ScaleFactorChanged { .. }
            | WindowEvent::Focused(false)
                if drag.take().is_some() && GetCapture() == hwnd =>
            {
                let _ = ReleaseCapture();
            }
            _ => (),
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Observation {
    pub minimized: bool,
    pub maximized: bool,
    pub screen_rect: [i32; 4],
    pub cursor: &'static str,
}
#[allow(unsafe_code)]
pub fn observe(parent: &NativeScopedParent<'_>) -> io::Result<Observation> {
    let hwnd = hwnd(parent)?;
    let mut rect = RECT::default();
    // SAFETY: read-only current owned parent observation under the UI-thread lease.
    unsafe {
        GetWindowRect(hwnd, &mut rect).map_err(io::Error::other)?;
        Ok(Observation {
            minimized: IsIconic(hwnd).as_bool(),
            maximized: IsZoomed(hwnd).as_bool(),
            screen_rect: [rect.left, rect.top, rect.right, rect.bottom],
            cursor: cursor_name(),
        })
    }
}

fn cursor_for(
    edge: humanymous_browser::frame_geometry::Edge,
    maximized: bool,
) -> windows::core::PCWSTR {
    use humanymous_browser::frame_geometry::Edge::*;
    if maximized {
        return IDC_ARROW;
    }
    match edge {
        West | East => IDC_SIZEWE,
        North | South => IDC_SIZENS,
        NorthWest | SouthEast => IDC_SIZENWSE,
        NorthEast | SouthWest => IDC_SIZENESW,
    }
}
#[allow(unsafe_code)]
fn cursor_name() -> &'static str {
    // SAFETY: read OS cursor identity; load shared system cursors, never destroy them.
    unsafe {
        let mut info = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            ..Default::default()
        };
        if GetCursorInfo(&mut info).is_err() {
            return "unknown";
        }
        for (id, name) in [
            (IDC_ARROW, "arrow"),
            (IDC_IBEAM, "text"),
            (IDC_HAND, "pointer"),
            (IDC_NO, "not_allowed"),
            (IDC_SIZEWE, "ew_resize"),
            (IDC_SIZENS, "ns_resize"),
            (IDC_SIZENWSE, "nwse_resize"),
            (IDC_SIZENESW, "nesw_resize"),
        ] {
            if LoadCursorW(None, id).is_ok_and(|cursor| cursor == info.hCursor) {
                return name;
            }
        }
        "other"
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn maximized_edges_use_arrow_instead_of_resize_cursor() {
        use humanymous_browser::frame_geometry::Edge::*;
        for edge in [
            West, East, North, South, NorthWest, NorthEast, SouthWest, SouthEast,
        ] {
            assert_eq!(super::cursor_for(edge, true), super::IDC_ARROW);
            assert_ne!(super::cursor_for(edge, false), super::IDC_ARROW);
        }
    }
}

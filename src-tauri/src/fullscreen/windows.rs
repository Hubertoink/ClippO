use std::{mem::size_of, ptr::null_mut};
use tauri::WebviewWindow;
use windows_sys::Win32::{
    Foundation::{POINT, RECT},
    Graphics::Gdi::{
        ClientToScreen, CreateRectRgn, DeleteObject, GetMonitorInfoW, MonitorFromWindow,
        SetWindowRgn, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    },
    UI::WindowsAndMessaging::{
        GetAncestor, GetClassNameW, GetClientRect, GetDesktopWindow, GetShellWindow,
        GetWindowLongW, IsZoomed, WindowFromPoint, GA_ROOT, GWL_STYLE, WS_CAPTION,
    },
};

pub struct Probe;
impl Probe {
    pub fn new() -> Option<Self> {
        Some(Self)
    }

    pub fn fullscreen_on_monitor(&self, widget: &WebviewWindow) -> bool {
        let Ok(hwnd) = widget.hwnd() else {
            return false;
        };
        unsafe {
            let monitor = MonitorFromWindow(hwnd.0.cast(), MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                return false;
            }
            let screen = info.rcMonitor;
            // Sample the visible application on this display, even if the user
            // is working on a different monitor. The resting drop is at the edge.
            let point = POINT {
                x: screen.left + (screen.right - screen.left) / 2,
                y: screen.top + (screen.bottom - screen.top) / 2,
            };
            let app = GetAncestor(WindowFromPoint(point), GA_ROOT);
            if app.is_null()
                || app == hwnd.0.cast()
                || app == GetDesktopWindow()
                || app == GetShellWindow()
            {
                return false;
            }
            let mut class = [0u16; 64];
            let n = GetClassNameW(app, class.as_mut_ptr(), class.len() as i32);
            if matches!(
                String::from_utf16_lossy(&class[..n as usize]).as_str(),
                "WorkerW" | "Progman" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
            ) {
                return false;
            }
            // Maximized, captioned windows can cover a display with no taskbar.
            if IsZoomed(app) != 0
                && GetWindowLongW(app, GWL_STYLE) as u32 & WS_CAPTION == WS_CAPTION
            {
                return false;
            }
            let mut client = RECT::default();
            let mut origin = POINT::default();
            GetClientRect(app, &mut client) != 0
                && ClientToScreen(app, &mut origin) != 0
                && origin.x <= screen.left
                && origin.y <= screen.top
                && origin.x + client.right >= screen.right
                && origin.y + client.bottom >= screen.bottom
        }
    }
}

pub fn set_mask(window: &WebviewWindow, hidden: bool) -> bool {
    let Ok(hwnd) = window.hwnd() else {
        return false;
    };
    unsafe {
        let region = if hidden {
            let empty = CreateRectRgn(0, 0, 0, 0);
            if empty.is_null() {
                return false;
            }
            empty
        } else {
            null_mut()
        };
        // On success Windows owns the region.
        if SetWindowRgn(hwnd.0.cast(), region, 1) != 0 {
            return true;
        }
        if !region.is_null() {
            DeleteObject(region);
        }
        false
    }
}

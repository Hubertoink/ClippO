use crate::jobs::Session;
use std::{sync::atomic::Ordering, thread, time::Duration};
use tauri::{AppHandle, Manager, WebviewWindow};

#[cfg(target_os = "linux")]
#[path = "fullscreen/linux.rs"]
mod platform;
#[cfg(windows)]
#[path = "fullscreen/windows.rs"]
mod platform;

// An empty native window region removes both the pixels and the mouse target.
// The window's visibility and focus continue to belong to the user.
fn mask(window: &WebviewWindow, hidden: bool) {
    let state = window.state::<Session>();
    if hidden && !state.collapsed.load(Ordering::Acquire) {
        return;
    }
    if state.teardrop_masked.load(Ordering::Relaxed) != hidden && platform::set_mask(window, hidden)
    {
        state.teardrop_masked.store(hidden, Ordering::Release);
    }
}

pub fn reveal(window: &WebviewWindow) {
    if window
        .state::<Session>()
        .teardrop_masked
        .load(Ordering::Acquire)
    {
        let widget = window.clone();
        let _ = window.run_on_main_thread(move || mask(&widget, false));
    }
}

pub fn monitor(app: AppHandle) {
    thread::spawn(move || {
        let Some(probe) = platform::Probe::new() else {
            return;
        };
        loop {
            thread::sleep(Duration::from_millis(300));
            let state = app.state::<Session>();
            if !state.ready.load(Ordering::Acquire) || !state.collapsed.load(Ordering::Acquire) {
                continue;
            }
            let Some(window) = app.get_webview_window("main") else {
                break;
            };
            let hidden = probe.fullscreen_on_monitor(&window);
            if hidden != state.teardrop_masked.load(Ordering::Acquire) {
                let _ = app.run_on_main_thread(move || mask(&window, hidden));
            }
        }
    });
}

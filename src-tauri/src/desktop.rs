use crate::{
    engine::{self, AppResult, Explain},
    jobs::Session,
};
use notify::{EventKind, RecursiveMode, Watcher};
use serde::Serialize;
#[cfg(windows)]
use std::os::windows::ffi::OsStringExt;
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::PathBuf,
    sync::{atomic::Ordering, mpsc},
    thread,
    time::{Duration, Instant, SystemTime},
};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};
#[cfg(windows)]
use windows_sys::Win32::{
    System::{
        Com::CoTaskMemFree,
        DataExchange::{GetClipboardSequenceNumber, IsClipboardFormatAvailable},
    },
    UI::Shell::{FOLDERID_Screenshots, SHGetKnownFolderPath},
};

pub fn show(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        let _ = app.emit("widget-opened", ());
    }
}
#[tauri::command]
pub fn hide_widget(window: WebviewWindow) -> tauri::Result<()> {
    window.hide()?;
    #[cfg(windows)]
    if window.is_visible()? {
        // A non-activating screenshot popup can be visible before Tauri has
        // marked the window as shown. Hide the native popup in that case.
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
        unsafe {
            ShowWindow(window.hwnd()?.0.cast(), SW_HIDE);
        }
    }
    Ok(())
}

pub fn toggle(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) && !win.is_minimized().unwrap_or(false) {
            let state = app.state::<Session>();
            if !state.docked.load(Ordering::Relaxed) {
                let _ = hide_widget(win);
            } else if state.collapsed.load(Ordering::Relaxed) {
                show(app);
            } else {
                let _ = app.emit("collapse-widget", ());
            }
        } else {
            show(app);
        }
    }
}
fn position_file(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join("position.json"))
}
pub fn position(win: &WebviewWindow) {
    let Ok(monitors) = win.available_monitors() else {
        return;
    };
    let size = win
        .outer_size()
        .unwrap_or(tauri::PhysicalSize::new(420, 260));
    let stored = position_file(win.app_handle())
        .and_then(|p| fs::read(p).ok())
        .and_then(|v| serde_json::from_slice::<(i32, i32)>(&v).ok());
    if let Some((x, y)) = stored {
        if monitors.iter().any(|m| {
            let a = m.work_area();
            x >= a.position.x
                && y >= a.position.y
                && x + size.width as i32 <= a.position.x + a.size.width as i32
                && y + size.height as i32 <= a.position.y + a.size.height as i32
        }) {
            let _ = win.set_position(PhysicalPosition::new(x, y));
            return;
        }
    }
    if let Ok(Some(m)) = win.primary_monitor() {
        let a = m.work_area();
        let pad = (18. * m.scale_factor()) as i32;
        let _ = win.set_position(PhysicalPosition::new(
            a.position.x + a.size.width as i32 - size.width as i32 - pad,
            a.position.y + a.size.height as i32 - size.height as i32 - pad,
        ));
    }
}
pub fn keep_visible(win: &tauri::Window) {
    let (Ok(Some(m)), Ok(size), Ok(pos)) = (
        win.current_monitor(),
        win.outer_size(),
        win.outer_position(),
    ) else {
        return;
    };
    let a = m.work_area();
    let x = pos.x.clamp(
        a.position.x,
        (a.position.x + a.size.width as i32 - size.width as i32).max(a.position.x),
    );
    let y = pos.y.clamp(
        a.position.y,
        (a.position.y + a.size.height as i32 - size.height as i32).max(a.position.y),
    );
    if x != pos.x || y != pos.y {
        let _ = win.set_position(PhysicalPosition::new(x, y));
    }
}
pub fn remember(app: &AppHandle, x: i32, y: i32) {
    if app.state::<Session>().docked.load(Ordering::Relaxed) {
        return;
    }
    if let Some(p) = position_file(app) {
        if let Some(dir) = p.parent() {
            let _ = fs::create_dir_all(dir);
            if let Ok(mut temp) = tempfile::NamedTempFile::new_in(dir) {
                if write!(temp, "[{x},{y}]").is_ok() {
                    let _ = temp.persist(&p);
                }
            }
        }
    }
}

#[tauri::command]
pub fn layout_widget(
    window: WebviewWindow,
    docked: bool,
    collapsed: bool,
    height: u32,
) -> AppResult<()> {
    let state = window.state::<Session>();
    let previous = state.docked.swap(docked, Ordering::Relaxed);
    state
        .collapsed
        .store(docked && collapsed, Ordering::Relaxed);
    #[cfg(windows)]
    if previous != docked {
        let effects = if docked {
            None
        } else {
            window.config().app.windows[0].window_effects.clone()
        };
        let _ = window.set_effects(effects);
        let _ = window.set_shadow(!docked);
    }
    let height = height.clamp(200, 520);
    if !docked {
        window
            .set_size(tauri::LogicalSize::new(420, height))
            .explain()?;
        if previous {
            position(&window);
        }
        return Ok(());
    }
    let monitor = window
        .current_monitor()
        .explain()?
        .or(window.primary_monitor().explain()?)
        .ok_or("Display unavailable")?;
    let area = monitor.work_area();
    let (w, h) = if collapsed { (64, 24) } else { (420, height) };
    let size = tauri::LogicalSize::new(w, h).to_physical::<u32>(monitor.scale_factor());
    let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
    let y = area.position.y;
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER,
        };
        if SetWindowPos(
            window.hwnd().explain()?.0.cast(),
            std::ptr::null_mut(),
            x,
            y,
            size.width as i32,
            size.height as i32,
            SWP_NOACTIVATE | SWP_NOZORDER,
        ) == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    #[cfg(not(windows))]
    {
        window.set_size(size).explain()?;
        window.set_position(PhysicalPosition::new(x, y)).explain()?;
    }
    Ok(())
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Capture {
    path: Option<PathBuf>,
    was_hidden: bool,
}
fn capture(app: &AppHandle, path: Option<PathBuf>) {
    let s = app.state::<Session>();
    if !s.watching.load(Ordering::Relaxed) || !s.ready.load(Ordering::Acquire) {
        return;
    }
    if let Some(win) = app.get_webview_window("main") {
        let hidden = !win.is_visible().unwrap_or(true);
        if hidden {
            if show_capture(&win).is_err() {
                return;
            }
        }
        let _ = app.emit(
            "capture",
            Capture {
                path,
                was_hidden: hidden,
            },
        );
    }
}

#[cfg(windows)]
fn show_capture(win: &WebviewWindow) -> tauri::Result<()> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
    unsafe {
        ShowWindow(win.hwnd()?.0.cast(), SW_SHOWNOACTIVATE);
    }
    Ok(())
}

#[cfg(not(windows))]
fn show_capture(win: &WebviewWindow) -> tauri::Result<()> {
    win.show()
}
#[cfg(windows)]
fn screenshot_dir() -> Option<PathBuf> {
    unsafe {
        let mut p = std::ptr::null_mut();
        if SHGetKnownFolderPath(&FOLDERID_Screenshots, 0x4000, std::ptr::null_mut(), &mut p) < 0
            || p.is_null()
        {
            return None;
        }
        let mut n = 0;
        while *p.add(n) != 0 {
            n += 1
        }
        let path = PathBuf::from(std::ffi::OsString::from_wide(std::slice::from_raw_parts(
            p, n,
        )));
        CoTaskMemFree(p.cast());
        Some(path)
    }
}
#[cfg(target_os = "linux")]
fn screenshot_dir() -> Option<PathBuf> {
    let dirs = directories::UserDirs::new()?;
    Some(
        dirs.picture_dir()
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs.home_dir().join("Pictures"))
            .join("Screenshots"),
    )
}
fn watch_folder(app: AppHandle) {
    let Some(folder) = screenshot_dir().or_else(|| {
        directories::UserDirs::new()?
            .picture_dir()
            .map(|p| p.join("Screenshots"))
    }) else {
        return;
    };
    // Watching the parent also handles Windows creating Screenshots on the first capture.
    let Some(parent) = folder.parent() else {
        return;
    };
    let (send, recv) = mpsc::channel();
    let Ok(mut watcher) = notify::recommended_watcher(move |ev| {
        let _ = send.send(ev);
    }) else {
        return;
    };
    let root = if parent.is_dir() { parent } else { return };
    if watcher.watch(root, RecursiveMode::Recursive).is_err() {
        return;
    }
    let mut pending: HashMap<PathBuf, (u64, Instant)> = HashMap::new();
    let mut seen: HashMap<PathBuf, SystemTime> = HashMap::new();
    loop {
        if let Ok(Ok(e)) = recv.recv_timeout(Duration::from_millis(150)) {
            if matches!(e.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                for p in e.paths {
                    if p.parent() == Some(&folder)
                        && engine::is_image(&p)
                        && !p
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .contains("-clippo")
                    {
                        if let Ok(m) = p.metadata() {
                            pending.insert(p, (m.len(), Instant::now()));
                        }
                    }
                }
            }
        }
        pending.retain(|p, (size, at)| {
            if at.elapsed() < Duration::from_millis(280) {
                return true;
            }
            let Ok(meta) = fs::metadata(p) else {
                return false;
            };
            if meta.len() != *size {
                *size = meta.len();
                *at = Instant::now();
                return true;
            }
            let Ok(stamp) = meta.modified() else {
                return false;
            };
            let ignored = app
                .state::<Session>()
                .ignored_path
                .lock()
                .ok()
                .is_some_and(|last| {
                    last.as_ref().is_some_and(|(path, t)| {
                        p == path && t.elapsed().is_ok_and(|age| age < Duration::from_secs(5))
                    })
                });
            if *size > 0 && !ignored && seen.get(p) != Some(&stamp) {
                seen.insert(p.clone(), stamp);
                capture(&app, Some(p.clone()));
            }
            false
        });
        seen.retain(|_, time| {
            time.elapsed()
                .is_ok_and(|age| age < Duration::from_secs(60))
        });
    }
}
pub fn monitor(app: &AppHandle) {
    let app1 = app.clone();
    thread::spawn(move || watch_folder(app1));
    #[cfg(windows)]
    monitor_clipboard(app.clone());
}
#[cfg(windows)]
fn monitor_clipboard(app2: AppHandle) {
    thread::spawn(move || {
        let mut previous = unsafe { GetClipboardSequenceNumber() };
        loop {
            thread::sleep(Duration::from_millis(240));
            let current = unsafe { GetClipboardSequenceNumber() };
            if current == previous {
                continue;
            }
            previous = current;
            let state = app2.state::<Session>();
            if current == state.clipboard_sequence.load(Ordering::Relaxed) {
                continue;
            }
            if unsafe {
                IsClipboardFormatAvailable(8) != 0
                    || IsClipboardFormatAvailable(17) != 0
                    || IsClipboardFormatAvailable(2) != 0
            } {
                capture(&app2, None)
            }
        }
    });
}
#[tauri::command]
pub fn set_detection(app: AppHandle, enabled: bool) {
    app.state::<Session>()
        .watching
        .store(enabled, Ordering::Relaxed)
}
#[tauri::command]
pub fn widget_ready(app: AppHandle) -> AppResult<()> {
    let state = app.state::<Session>();
    state.ready.store(true, Ordering::Release);
    let win = app.get_webview_window("main").ok_or("Missing window")?;
    win.show().explain()?;
    if !state.collapsed.load(Ordering::Relaxed) {
        win.set_focus().explain()?;
    }
    if state.shortcut_failed.load(Ordering::Relaxed) {
        return Err(
            "Global shortcut unavailable. Open ClippO from the application menu or tray.".into(),
        );
    }
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    fn check_capture_visibility(app: &AppHandle) -> AppResult<()> {
        let win = app
            .get_webview_window("main")
            .ok_or("Missing test window")?;
        let state = app.state::<Session>();
        state.watching.store(true, Ordering::Relaxed);
        state.ready.store(true, Ordering::Release);

        // A CI desktop may have no foreground app before the first window is
        // shown. Give the screenshot notification an existing window to leave focused.
        let foreground = tauri::WebviewWindowBuilder::new(
            app,
            "foreground-fixture",
            tauri::WebviewUrl::External("about:blank".parse().unwrap()),
        )
        .title("ClippO focus test")
        .position(-3000., -3000.)
        .inner_size(120., 80.)
        .skip_taskbar(true)
        .build()
        .explain()?;
        foreground.set_focus().explain()?;
        if unsafe { GetForegroundWindow() } == win.hwnd().explain()?.0 {
            return Err("Could not establish the foreground test fixture".into());
        }
        for from_folder in [false, true] {
            capture(app, from_folder.then(|| PathBuf::from("Screenshot.png")));
            if !win.is_visible().explain()? {
                return Err("Screenshot did not show the hidden widget".into());
            }
            if unsafe { GetForegroundWindow() } == win.hwnd().explain()?.0 {
                return Err("Screenshot took focus".into());
            }
            if from_folder {
                toggle(app); // global shortcut handler
            } else {
                hide_widget(win.clone()).explain()?; // same command as the minus button
            }
            if win.is_visible().explain()? {
                return Err("Widget stayed visible after dismissing the screenshot".into());
            }
        }
        toggle(app);
        if !win.is_visible().explain()? {
            return Err("Shortcut did not reopen the widget".into());
        }
        Ok(())
    }

    #[test]
    fn screenshot_popup_can_be_hidden_by_button_and_shortcut() {
        let mut context = tauri::generate_context!();
        let profile = tempfile::tempdir().unwrap();
        let window = &mut context.config_mut().app.windows[0];
        window.url = tauri::WebviewUrl::External("about:blank".parse().unwrap());
        window.data_directory = Some(profile.path().to_owned());
        window.x = Some(-3000.);
        window.y = Some(-3000.);
        let app = tauri::Builder::default()
            .any_thread()
            .manage(Session::default())
            .build(context)
            .unwrap();
        let status = app.run_return(|handle, event| {
            if matches!(event, tauri::RunEvent::Ready) {
                match check_capture_visibility(handle) {
                    Ok(()) => handle.exit(0),
                    Err(reason) => {
                        eprintln!("{reason}");
                        handle.exit(1);
                    }
                }
            }
        });
        assert_eq!(status, 0);
    }
}

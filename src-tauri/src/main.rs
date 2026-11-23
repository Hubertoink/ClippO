#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod audio;
mod cli;
mod desktop;
mod engine;
mod fullscreen;
mod jobs;
mod screenshot_fan;
#[cfg(target_os = "linux")]
mod omarchy;
#[cfg(windows)]
mod windows_drop;

use std::sync::atomic::Ordering;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[tauri::command]
fn portable_build() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("clippo-portable").is_file()))
        .unwrap_or(false)
}

#[tauri::command]
fn open_portable_release(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(
            "https://github.com/Hubertoink/ClippO/releases/latest",
            None::<&str>,
        )
        .map_err(|e| e.to_string())
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments
        .first()
        .is_some_and(|argument| argument == "compress")
    {
        std::process::exit(cli::compress(&arguments[1..]));
    }
    #[cfg(target_os = "linux")]
    if arguments
        .first()
        .is_some_and(|argument| argument == "panel")
    {
        let result = std::process::Command::new("omarchy-shell")
            .args(["hubertoink.clippo", "toggle"])
            .status();
        if result.is_ok_and(|status| status.success()) {
            return;
        }
    }
    tauri::Builder::default()
        .manage(jobs::Session::default())
        .manage(std::sync::Mutex::new(screenshot_fan::History::default()))
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            desktop::show(app)
        }))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let win = app.get_webview_window("main").unwrap();
            #[cfg(windows)]
            windows_drop::install(&win)?;
            desktop::position(&win);
            let open = MenuItem::with_id(app, "open", "Open ClippO", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit ClippO", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("ClippO")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => desktop::show(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        desktop::show(tray.app_handle())
                    }
                })
                .build(app)?;
            let shortcut_plugin = tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, e| {
                    if e.state == ShortcutState::Pressed {
                        desktop::toggle(app)
                    }
                })
                .build();
            let conflict = app.handle().plugin(shortcut_plugin).is_err()
                || app.global_shortcut().register("Ctrl+Shift+O").is_err();
            app.state::<jobs::Session>()
                .shortcut_failed
                .store(conflict, Ordering::Relaxed);
            jobs::clear_old(app.handle());
            desktop::monitor(app.handle());
            Ok(())
        })
        .on_window_event(|win, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                if let Some(widget) = win.get_webview_window(win.label()) {
                    let _ = desktop::hide_widget(widget);
                }
            }
            WindowEvent::Moved(pos) => desktop::remember(win.app_handle(), pos.x, pos.y),
            WindowEvent::Resized(_) => desktop::keep_visible(win),
            _ => (),
        })
        .invoke_handler(tauri::generate_handler![
            portable_build,
            #[cfg(target_os = "linux")]
            omarchy::omarchy_palette,
            open_portable_release,
            jobs::file_size,
            jobs::image_dimensions,
            jobs::compress_file,
            jobs::compress_clipboard,
            jobs::compress_capture,
            jobs::result_action,
            screenshot_fan::configure_screenshot_fan,
            screenshot_fan::drag_screenshot,
            jobs::reveal_saved,
            desktop::set_detection,
            desktop::layout_widget,
            desktop::hide_widget,
            desktop::widget_ready
        ])
        .run(tauri::generate_context!())
        .expect("ClippO could not start");
}

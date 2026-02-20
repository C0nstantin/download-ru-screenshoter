mod commands;
mod state;
mod upload;

use std::io::{Read, Seek, SeekFrom};
use tauri::{AppHandle, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri_plugin_global_shortcut::ShortcutState;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Custom protocol to serve local video files with byte-range support
        .register_uri_scheme_protocol("localfile", |_app, req| {
            let path = req.uri().path().to_string();
            let file_path = percent_decode(&path);

            let mime = if file_path.ends_with(".mp4") { "video/mp4" }
                       else if file_path.ends_with(".mov") || file_path.ends_with(".MOV") { "video/quicktime" }
                       else { "application/octet-stream" };

            let mut file = match std::fs::File::open(&file_path) {
                Ok(f) => f,
                Err(_) => return tauri::http::Response::builder()
                    .status(404).body(vec![]).unwrap(),
            };
            let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);

            // Handle Range header for video seeking
            if let Some(range_val) = req.headers().get("range") {
                let range_str = range_val.to_str().unwrap_or("bytes=0-");
                let (start, end) = parse_byte_range(range_str, file_size);
                let length = end - start + 1;
                let mut buf = vec![0u8; length as usize];
                let _ = file.seek(SeekFrom::Start(start));
                let _ = file.read_exact(&mut buf);
                return tauri::http::Response::builder()
                    .status(206)
                    .header("Content-Range", format!("bytes {}-{}/{}", start, end, file_size))
                    .header("Content-Length", length.to_string())
                    .header("Accept-Ranges", "bytes")
                    .header("Content-Type", mime)
                    .body(buf).unwrap();
            }

            let mut buf = Vec::new();
            let _ = file.read_to_end(&mut buf);
            tauri::http::Response::builder()
                .status(200)
                .header("Content-Length", file_size.to_string())
                .header("Accept-Ranges", "bytes")
                .header("Content-Type", mime)
                .body(buf).unwrap()
        })
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    commands::hotkeys::handle_shortcut(app, shortcut);
                }
            })
            .build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();

            // Create tray menu - without "show window" option
            let snap_i = MenuItem::with_id(&handle, "screenshot", "Скриншот области (Ctrl+Shift+4)", true, None::<&str>)?;
            let snap_full_i = MenuItem::with_id(&handle, "screenshot_full", "Скриншот экрана (Ctrl+Shift+3)", true, None::<&str>)?;
            let snap_win_i = MenuItem::with_id(&handle, "screenshot_window", "Скриншот окна (Ctrl+Shift+Alt+3)", true, None::<&str>)?;
            let video_i = MenuItem::with_id(&handle, "video_record", "Запись видео", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(&handle, "settings", "Настройки...", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(&handle, "quit", "Выйти", true, None::<&str>)?;

            let menu = Menu::with_items(&handle, &[&snap_i, &snap_full_i, &snap_win_i, &video_i, &settings_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app_handle: &AppHandle, event| {
                    match event.id.as_ref() {
                        "quit" => { app_handle.exit(0); }
                        "settings" => {
                            show_settings_window(app_handle);
                        }
                        "screenshot" => {
                            let _ = commands::screenshot::start_region_capture(app_handle.clone());
                        }
                        "screenshot_full" => {
                            let _ = commands::screenshot::capture_fullscreen_and_edit_internal(app_handle.clone(), None);
                        }
                        "screenshot_window" => {
                            let _ = commands::screenshot::capture_window_and_edit(app_handle.clone());
                        }
                        "video_record" => {
                            let state = app_handle.state::<AppState>();
                            let _ = commands::recording::start_video_capture(app_handle.clone(), state);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // Double click on tray icon opens settings
                    if let TrayIconEvent::DoubleClick { .. } = event {
                        let app_handle = tray.app_handle();
                        show_settings_window(app_handle);
                    }
                })
                .build(app)?;

            // Register global shortcuts from saved config
            if let Err(e) = commands::hotkeys::register_hotkeys(&app.handle()) {
                eprintln!("Failed to register hotkeys: {}", e);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let label = window.label();
                if label == "main" || label == "editor" || label == "video-result" {
                    window.hide().unwrap();
                    api.prevent_close();
                    // Hide from Dock and Cmd+Tab when no windows are visible
                    #[cfg(target_os = "macos")]
                    update_activation_policy(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::screenshot::capture_fullscreen,
            commands::screenshot::capture_fullscreen_and_edit,
            commands::screenshot::crop_image,
            commands::screenshot::get_current_screenshot,
            commands::screenshot::get_displays,
            commands::screenshot::save_screenshot,
            commands::screenshot::open_editor,
            commands::upload::upload_to_download,
            commands::upload::set_access_token,
            commands::upload::get_access_token,
            commands::upload::open_oauth_browser,
            commands::upload::refresh_oauth_token,
            commands::upload::load_saved_token,
            commands::upload::logout,
            commands::hotkeys::get_hotkeys,
            commands::hotkeys::set_hotkeys,
            commands::settings::open_system_settings,
            commands::recording::start_video_capture,
            commands::recording::stop_video_recording,
            commands::recording::is_recording,
            commands::recording::move_recording,
            commands::recording::get_video_info,
            commands::recording::get_last_recording_path,
            commands::recording::convert_to_mp4,
            commands::recording::upload_video_to_download,
            commands::recording::delete_recording,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn percent_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next().unwrap_or(b'0') as char;
            let h2 = chars.next().unwrap_or(b'0') as char;
            if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                result.push(byte as char);
            }
        } else {
            result.push(b as char);
        }
    }
    result
}

fn parse_byte_range(range: &str, file_size: u64) -> (u64, u64) {
    // Format: "bytes=start-end" or "bytes=start-"
    let range = range.trim_start_matches("bytes=");
    let mut parts = range.splitn(2, '-');
    let start: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let end: u64 = parts.next()
        .and_then(|s| if s.is_empty() { None } else { s.parse().ok() })
        .unwrap_or(file_size.saturating_sub(1));
    (start, end.min(file_size.saturating_sub(1)))
}

fn show_settings_window(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(target_os = "macos")]
fn update_activation_policy(app: &AppHandle) {
    let main_visible = app.get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    let editor_visible = app.get_webview_window("editor")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);

    if !main_visible && !editor_visible {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    }
}

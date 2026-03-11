mod commands;
pub mod i18n;
mod state;
mod upload;

use std::io::{Read, Seek, SeekFrom};
use tauri::{AppHandle, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri_plugin_global_shortcut::ShortcutState;

use state::AppState;

/// Activate app in Dock and Cmd+Tab (Accessory→Regular requires NSApp.activate).
#[cfg(target_os = "macos")]
pub fn activate_as_regular(app: &AppHandle) {
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    unsafe {
        use cocoa::appkit::NSApplication;
        let ns_app = cocoa::appkit::NSApp();
        ns_app.activateIgnoringOtherApps_(objc::runtime::YES);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // If user tries to launch again, show settings window
            show_settings_window(app);
        }))
        // Custom protocol to serve local video files with byte-range support
        .register_uri_scheme_protocol("localfile", |_app, req| {
            let path = req.uri().path().to_string();
            let file_path = percent_decode(&path);

            // Security: only allow video files from /tmp/recording_* (our own recordings)
            let canonical = std::path::Path::new(&file_path);
            let filename = canonical.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let is_allowed = canonical.parent() == Some(std::path::Path::new("/tmp"))
                && filename.starts_with("recording_")
                && matches!(
                    canonical.extension().and_then(|e| e.to_str()),
                    Some("mov" | "mp4" | "MOV" | "MP4")
                )
                && !file_path.contains("..");
            if !is_allowed {
                eprintln!("localfile:// blocked path: {}", file_path);
                return tauri::http::Response::builder()
                    .status(403).body(vec![]).unwrap();
            }

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
                // Cap single range response at 10MB to prevent OOM
                const MAX_RANGE_SIZE: u64 = 10 * 1024 * 1024;
                let length = length.min(MAX_RANGE_SIZE);
                let end = start + length - 1;
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
            let tr = i18n::current(&handle);
            let snap_i = MenuItem::with_id(&handle, "screenshot", format!("{} (Ctrl+Shift+4)", tr.screenshot_region), true, None::<&str>)?;
            let snap_full_i = MenuItem::with_id(&handle, "screenshot_full", format!("{} (Ctrl+Shift+3)", tr.screenshot_fullscreen), true, None::<&str>)?;
            let snap_win_i = MenuItem::with_id(&handle, "screenshot_window", format!("{} (Ctrl+Shift+Alt+3)", tr.screenshot_window), true, None::<&str>)?;
            let video_screen_i = MenuItem::with_id(&handle, "video_screen", tr.video_screen, true, None::<&str>)?;
            let video_region_i = MenuItem::with_id(&handle, "video_region", tr.video_region, true, None::<&str>)?;
            let video_window_i = MenuItem::with_id(&handle, "video_window", tr.video_window, true, None::<&str>)?;
            let settings_i = MenuItem::with_id(&handle, "settings", tr.settings, true, None::<&str>)?;
            let quit_i = MenuItem::with_id(&handle, "quit", tr.quit, true, None::<&str>)?;

            let menu = Menu::with_items(&handle, &[&snap_i, &snap_full_i, &snap_win_i, &video_screen_i, &video_region_i, &video_window_i, &settings_i, &quit_i])?;

            let tray_icon_bytes = include_bytes!("../icons/tray-icon.png");
            let tray_icon = tauri::image::Image::from_bytes(tray_icon_bytes)
                .expect("failed to load tray icon");

            let _tray = TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .icon_as_template(true)
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
                            let app = app_handle.clone();
                            std::thread::spawn(move || {
                                let _ = commands::screenshot::capture_fullscreen_and_edit_internal(app, None);
                            });
                        }
                        "screenshot_window" => {
                            let app = app_handle.clone();
                            std::thread::spawn(move || {
                                let _ = commands::screenshot::capture_window_and_edit(app);
                            });
                        }
                        "video_screen" => {
                            let state = app_handle.state::<AppState>();
                            let _ = commands::recording::start_video_capture(app_handle.clone(), state);
                        }
                        "stop_recording" => {
                            let _ = commands::recording::stop_recording_internal(app_handle);
                        }
                        "video_region" => {
                            let _ = commands::screenshot::start_region_capture_overlay_video(app_handle.clone());
                        }
                        "video_window" => {
                            let state = app_handle.state::<AppState>();
                            let _ = commands::recording::start_video_capture_window(app_handle.clone(), state);
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

            // Start as tray-only app (no Dock icon, no Cmd+Tab entry)
            #[cfg(target_os = "macos")]
            let _ = app.handle().set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Register global shortcuts from saved config
            if let Err(e) = commands::hotkeys::register_hotkeys(&app.handle()) {
                eprintln!("Failed to register hotkeys: {}", e);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    let label = window.label();
                    if label == "main" || label == "editor" || label == "video-result" {
                        // Re-register hotkeys in case user was editing and closed without saving
                        if label == "main" {
                            let _ = commands::hotkeys::register_hotkeys(window.app_handle());
                        }
                        window.hide().unwrap();
                        api.prevent_close();
                        #[cfg(target_os = "macos")]
                        update_activation_policy(window.app_handle());
                    }
                }
                tauri::WindowEvent::Focused(true) => {
                    #[cfg(target_os = "macos")]
                    {
                        // Only show in Dock/Cmd+Tab for actual UI windows, not overlays/oauth/hidden.
                        // "recording" is excluded — its focus would steal from screencapture's native picker.
                        let dominated = ["main", "editor", "video-result"];
                        if dominated.contains(&window.label()) && window.is_visible().unwrap_or(false) {
                            activate_as_regular(window.app_handle());
                        }
                    }
                }
                tauri::WindowEvent::Destroyed => {
                    #[cfg(target_os = "macos")]
                    update_activation_policy(window.app_handle());
                }
                _ => {}
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
            commands::hotkeys::unregister_hotkeys,
            commands::settings::open_system_settings,
            commands::recording::start_video_capture,
            commands::recording::start_video_recording,
            commands::recording::start_video_capture_window,
            commands::recording::stop_video_recording,
            commands::recording::is_recording,
            commands::recording::move_recording,
            commands::recording::get_video_info,
            commands::recording::get_last_recording_path,
            commands::recording::convert_to_mp4,
            commands::recording::upload_video_to_download,
            commands::recording::delete_recording,
            commands::recording::toggle_mute_mic,
            commands::recording::is_mic_muted,
            set_locale,
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

#[tauri::command]
fn set_locale(app: AppHandle, locale: String) {
    let state = app.state::<AppState>();
    {
        let mut l = state.locale.lock().unwrap();
        *l = locale;
    }
    // Rebuild tray menu in new language
    commands::recording::set_tray_recording_mode(&app, false);
}

fn show_settings_window(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    activate_as_regular(app);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(target_os = "macos")]
fn update_activation_policy(app: &AppHandle) {
    // Delay check so window hide/destroy fully completes before we inspect visibility.
    // Without delay, is_visible() may still return true for a window mid-hide.
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        let dominated_windows = ["main", "editor", "video-result"];
        let any_visible = dominated_windows.iter().any(|label| {
            app.get_webview_window(label)
                .and_then(|w| w.is_visible().ok())
                .unwrap_or(false)
        });

        if !any_visible {
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
    });
}

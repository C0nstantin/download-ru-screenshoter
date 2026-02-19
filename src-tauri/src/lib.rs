mod commands;
mod state;
mod upload;

use tauri::{AppHandle, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri_plugin_global_shortcut::ShortcutState;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            let settings_i = MenuItem::with_id(&handle, "settings", "Настройки...", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(&handle, "quit", "Выйти", true, None::<&str>)?;

            let menu = Menu::with_items(&handle, &[&snap_i, &snap_full_i, &settings_i, &quit_i])?;

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
                // Hide windows instead of closing (except overlay)
                let label = window.label();
                if label == "main" || label == "editor" {
                    window.hide().unwrap();
                    api.prevent_close();
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
            commands::upload::exchange_oauth_code,
            commands::upload::refresh_oauth_token,
            commands::upload::load_saved_token,
            commands::upload::logout,
            commands::hotkeys::get_hotkeys,
            commands::hotkeys::set_hotkeys,
            commands::settings::open_system_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn show_settings_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

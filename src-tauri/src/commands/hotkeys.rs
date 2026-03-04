use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tauri_plugin_store::StoreExt;

use crate::commands::screenshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub region: String,
    pub fullscreen: String,
    #[serde(default = "default_window_hotkey")]
    pub window: String,
}

fn default_window_hotkey() -> String {
    "Ctrl+Shift+Alt+3".to_string()
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            region: "Ctrl+Shift+4".to_string(),
            fullscreen: "Ctrl+Shift+3".to_string(),
            window: "Ctrl+Shift+Alt+3".to_string(),
        }
    }
}

fn parse_shortcut(shortcut_str: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = shortcut_str.split('+').map(|s| s.trim()).collect();

    let mut modifiers = Modifiers::empty();
    let mut code: Option<Code> = None;

    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "cmd" | "command" | "meta" | "super" => modifiers |= Modifiers::META,
            "1" => code = Some(Code::Digit1),
            "2" => code = Some(Code::Digit2),
            "3" => code = Some(Code::Digit3),
            "4" => code = Some(Code::Digit4),
            "5" => code = Some(Code::Digit5),
            "6" => code = Some(Code::Digit6),
            "7" => code = Some(Code::Digit7),
            "8" => code = Some(Code::Digit8),
            "9" => code = Some(Code::Digit9),
            "0" => code = Some(Code::Digit0),
            "a" => code = Some(Code::KeyA),
            "b" => code = Some(Code::KeyB),
            "c" => code = Some(Code::KeyC),
            "d" => code = Some(Code::KeyD),
            "e" => code = Some(Code::KeyE),
            "f" => code = Some(Code::KeyF),
            "g" => code = Some(Code::KeyG),
            "h" => code = Some(Code::KeyH),
            "i" => code = Some(Code::KeyI),
            "j" => code = Some(Code::KeyJ),
            "k" => code = Some(Code::KeyK),
            "l" => code = Some(Code::KeyL),
            "m" => code = Some(Code::KeyM),
            "n" => code = Some(Code::KeyN),
            "o" => code = Some(Code::KeyO),
            "p" => code = Some(Code::KeyP),
            "q" => code = Some(Code::KeyQ),
            "r" => code = Some(Code::KeyR),
            "s" => code = Some(Code::KeyS),
            "t" => code = Some(Code::KeyT),
            "u" => code = Some(Code::KeyU),
            "v" => code = Some(Code::KeyV),
            "w" => code = Some(Code::KeyW),
            "x" => code = Some(Code::KeyX),
            "y" => code = Some(Code::KeyY),
            "z" => code = Some(Code::KeyZ),
            "f1" => code = Some(Code::F1),
            "f2" => code = Some(Code::F2),
            "f3" => code = Some(Code::F3),
            "f4" => code = Some(Code::F4),
            "f5" => code = Some(Code::F5),
            "f6" => code = Some(Code::F6),
            "f7" => code = Some(Code::F7),
            "f8" => code = Some(Code::F8),
            "f9" => code = Some(Code::F9),
            "f10" => code = Some(Code::F10),
            "f11" => code = Some(Code::F11),
            "f12" => code = Some(Code::F12),
            _ => {}
        }
    }

    if let Some(key_code) = code {
        let mods = if modifiers.is_empty() { None } else { Some(modifiers) };
        Some(Shortcut::new(mods, key_code))
    } else {
        None
    }
}

#[tauri::command]
pub fn get_hotkeys(app: AppHandle) -> HotkeyConfig {
    let store = app.store("settings.json").ok();

    if let Some(store) = store {
        if let Some(config) = store.get("hotkeys") {
            if let Ok(hotkeys) = serde_json::from_value::<HotkeyConfig>(config.clone()) {
                return hotkeys;
            }
        }
    }

    HotkeyConfig::default()
}

/// Temporarily unregister all hotkeys (while user is editing hotkey bindings)
#[tauri::command]
pub fn unregister_hotkeys(app: AppHandle) -> Result<(), String> {
    let config = get_hotkeys(app.clone());
    for key in [&config.region, &config.fullscreen, &config.window] {
        if let Some(s) = parse_shortcut(key) {
            let _ = app.global_shortcut().unregister(s);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_hotkeys(app: AppHandle, config: HotkeyConfig) -> Result<(), String> {
    // Unregister old shortcuts
    let old_config = get_hotkeys(app.clone());

    if let Some(old_shortcut) = parse_shortcut(&old_config.region) {
        let _ = app.global_shortcut().unregister(old_shortcut);
    }
    if let Some(old_shortcut) = parse_shortcut(&old_config.fullscreen) {
        let _ = app.global_shortcut().unregister(old_shortcut);
    }
    if let Some(old_shortcut) = parse_shortcut(&old_config.window) {
        let _ = app.global_shortcut().unregister(old_shortcut);
    }

    // Parse and register new shortcuts
    let region_shortcut = parse_shortcut(&config.region)
        .ok_or_else(|| format!("Неверный формат горячей клавиши: {}", config.region))?;
    let fullscreen_shortcut = parse_shortcut(&config.fullscreen)
        .ok_or_else(|| format!("Неверный формат горячей клавиши: {}", config.fullscreen))?;
    let window_shortcut = parse_shortcut(&config.window)
        .ok_or_else(|| format!("Неверный формат горячей клавиши: {}", config.window))?;

    app.global_shortcut()
        .register(region_shortcut)
        .map_err(|e| format!("Не удалось зарегистрировать {}: {}", config.region, e))?;

    app.global_shortcut()
        .register(fullscreen_shortcut)
        .map_err(|e| format!("Не удалось зарегистрировать {}: {}", config.fullscreen, e))?;

    app.global_shortcut()
        .register(window_shortcut)
        .map_err(|e| format!("Не удалось зарегистрировать {}: {}", config.window, e))?;

    // Save to store
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set("hotkeys", serde_json::to_value(&config).map_err(|e| e.to_string())?);
    store.save().map_err(|e| e.to_string())?;

    // Update tray menu labels to reflect new hotkeys
    update_tray_menu_labels(&app, &config);

    Ok(())
}

fn update_tray_menu_labels(app: &AppHandle, config: &HotkeyConfig) {
    let updates = [
        ("screenshot", format!("Скриншот области ({})", config.region)),
        ("screenshot_full", format!("Скриншот экрана ({})", config.fullscreen)),
        ("screenshot_window", format!("Скриншот окна ({})", config.window)),
    ];
    for (id, text) in updates {
        if let Some(item) = app.menu().and_then(|m| m.get(id)) {
            if let tauri::menu::MenuItemKind::MenuItem(mi) = item {
                let _ = mi.set_text(text);
            }
        }
    }
}

pub fn register_hotkeys(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let config = get_hotkeys(app.clone());

    let region_shortcut = parse_shortcut(&config.region)
        .ok_or("Invalid region shortcut")?;
    let fullscreen_shortcut = parse_shortcut(&config.fullscreen)
        .ok_or("Invalid fullscreen shortcut")?;
    let window_shortcut = parse_shortcut(&config.window)
        .ok_or("Invalid window shortcut")?;

    // Unregister first to avoid "already registered" errors
    let _ = app.global_shortcut().unregister(region_shortcut);
    let _ = app.global_shortcut().unregister(fullscreen_shortcut);
    let _ = app.global_shortcut().unregister(window_shortcut);

    app.global_shortcut().register(region_shortcut)?;
    app.global_shortcut().register(fullscreen_shortcut)?;
    app.global_shortcut().register(window_shortcut)?;

    Ok(())
}

pub fn handle_shortcut(app: &AppHandle, shortcut: &Shortcut) {
    let config = get_hotkeys(app.clone());

    let region_shortcut = parse_shortcut(&config.region);
    let fullscreen_shortcut = parse_shortcut(&config.fullscreen);
    let window_shortcut = parse_shortcut(&config.window);

    if region_shortcut.as_ref() == Some(shortcut) {
        let _ = screenshot::start_region_capture(app.clone());
    } else if fullscreen_shortcut.as_ref() == Some(shortcut) {
        // Run in background thread — capture + PNG encoding is heavy on Retina
        let app = app.clone();
        std::thread::spawn(move || {
            if let Err(e) = screenshot::capture_fullscreen_and_edit_internal(app, None) {
                eprintln!("Error in fullscreen capture: {}", e);
            }
        });
    } else if window_shortcut.as_ref() == Some(shortcut) {
        let app = app.clone();
        std::thread::spawn(move || {
            if let Err(e) = screenshot::capture_window_and_edit(app) {
                eprintln!("Error in window capture: {}", e);
            }
        });
    }
}

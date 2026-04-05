use std::process::Command;
use screenshots::Screen;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn open_system_settings(url: String) -> Result<(), String> {
    // Only allow macOS system preference URLs and https
    let allowed = url.starts_with("x-apple.systempreferences:")
        || url.starts_with("https://");
    if !allowed {
        return Err(format!("Blocked URL scheme: {}", url));
    }
    Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| format!("Failed to open settings: {}", e))?;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct DiagnosticReport {
    pub app_version: String,
    pub os: String,
    pub arch: String,
    pub desktop_env: Option<String>,
    pub session_type: Option<String>,
    pub display_count: usize,
    pub displays: Vec<DisplayDiag>,
    pub screenshot_test: String,
    pub log_path: String,
    pub api_url: String,
}

#[derive(serde::Serialize)]
pub struct DisplayDiag {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub scale: f32,
    pub is_primary: bool,
}

#[tauri::command]
pub fn run_diagnostics(app: AppHandle) -> DiagnosticReport {
    let screens = Screen::all().unwrap_or_default();
    let displays: Vec<DisplayDiag> = screens.iter().map(|s| {
        let info = &s.display_info;
        DisplayDiag {
            id: info.id,
            width: info.width,
            height: info.height,
            x: info.x,
            y: info.y,
            scale: info.scale_factor,
            is_primary: info.is_primary,
        }
    }).collect();

    // Try a test capture
    let screenshot_test = if let Some(screen) = screens.first() {
        match screen.capture() {
            Ok(img) => format!("OK ({}x{})", img.width(), img.height()),
            Err(e) => format!("FAIL: {}", e),
        }
    } else {
        "FAIL: no screens found".to_string()
    };

    let log_path = app.path().app_log_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    DiagnosticReport {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        arch: std::env::consts::ARCH.to_string(),
        desktop_env: std::env::var("XDG_CURRENT_DESKTOP").ok(),
        session_type: std::env::var("XDG_SESSION_TYPE").ok(),
        display_count: displays.len(),
        displays,
        screenshot_test,
        log_path,
        api_url: crate::config::api_base_url(),
    }
}

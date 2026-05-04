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
    #[cfg(target_os = "linux")]
    pub gnome_shell_version: Option<String>,
    #[cfg(target_os = "linux")]
    pub has_ayatana_appindicator: Option<bool>,
    #[cfg(target_os = "linux")]
    pub gnome_extensions: Option<Vec<String>>,
    #[cfg(target_os = "linux")]
    pub appindicator_extension_enabled: Option<bool>,
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

#[cfg(target_os = "linux")]
fn parse_extensions_list(output: &str) -> Vec<String> {
    let trimmed = output.trim();
    let trimmed = trimmed.strip_prefix('\'').unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix('\'').unwrap_or(trimmed);
    let trimmed = trimmed.trim();

    if trimmed == "@as []" || trimmed.is_empty() {
        return Vec::new();
    }

    trimmed
        .split(',')
        .map(|s| s.trim().trim_matches('\'').trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
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

    #[cfg(target_os = "linux")]
    let gnome_shell_version = Command::new("gnome-shell")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok());

    #[cfg(target_os = "linux")]
    let has_ayatana_appindicator = match Command::new("sh")
        .args(["-c", "ldconfig -p | grep -q ayatana-appindicator"])
        .output()
    {
        Ok(o) => Some(o.status.success()),
        Err(e) => {
            tracing::warn!(error = %e, "could not run ldconfig to check ayatana-appindicator");
            None
        }
    };

    #[cfg(target_os = "linux")]
    let (gnome_extensions, appindicator_extension_enabled) = {
        let output = Command::new("gsettings")
            .args(["get", "org.gnome.shell", "enabled-extensions"])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout);
                let exts = parse_extensions_list(&text);
                let enabled = exts.iter().any(|e| {
                    e == "appindicatorsupport@rgcjonas.gmail.com"
                        || e == "ubuntu-appindicators@ubuntu.com"
                });
                (Some(exts), Some(enabled))
            }
            Ok(_) => {
                tracing::warn!("gsettings returned non-zero exit code");
                (None, None)
            }
            Err(e) => {
                tracing::warn!(error = %e, "could not run gsettings");
                (None, None)
            }
        }
    };

    DiagnosticReport {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        arch: std::env::consts::ARCH.to_string(),
        desktop_env: std::env::var("XDG_CURRENT_DESKTOP").ok(),
        session_type: std::env::var("XDG_SESSION_TYPE").ok(),
        #[cfg(target_os = "linux")]
        gnome_shell_version,
        #[cfg(target_os = "linux")]
        has_ayatana_appindicator,
        #[cfg(target_os = "linux")]
        gnome_extensions,
        #[cfg(target_os = "linux")]
        appindicator_extension_enabled,
        display_count: displays.len(),
        displays,
        screenshot_test,
        log_path,
        api_url: crate::config::api_base_url(),
    }
}

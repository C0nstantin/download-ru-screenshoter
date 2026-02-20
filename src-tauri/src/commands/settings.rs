use std::process::Command;

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

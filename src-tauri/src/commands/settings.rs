use std::process::Command;

#[tauri::command]
pub fn open_system_settings(url: String) -> Result<(), String> {
    Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| format!("Failed to open settings: {}", e))?;
    Ok(())
}

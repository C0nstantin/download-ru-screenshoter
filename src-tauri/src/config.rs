use std::sync::Mutex;

static API_BASE: Mutex<Option<String>> = Mutex::new(None);

const DEFAULT_URL: &str = "https://download.ru";
const BETA_URL: &str = "https://beta.download.ru";

/// Get current API base URL.
pub fn api_base_url() -> String {
    let guard = API_BASE.lock().unwrap();
    guard.clone().unwrap_or_else(|| DEFAULT_URL.to_string())
}

/// Set API base URL at runtime (called from settings or on startup).
pub fn set_api_base_url(url: &str) {
    let mut guard = API_BASE.lock().unwrap();
    *guard = Some(url.to_string());
}

/// Check if currently pointing to beta.
pub fn is_beta() -> bool {
    api_base_url().contains("beta.")
}

/// Toggle between prod and beta, return new URL.
pub fn toggle_beta(use_beta: bool) -> String {
    let url = if use_beta { BETA_URL } else { DEFAULT_URL };
    set_api_base_url(url);
    url.to_string()
}

/// Initialize from env var or stored preference.
pub fn init(app: &tauri::AppHandle) {
    // Env var takes priority
    if let Ok(url) = std::env::var("API_BASE_URL") {
        set_api_base_url(&url);
        return;
    }

    // Then check stored preference (only in dev_mode builds)
    #[cfg(feature = "dev_mode")]
    {
        use tauri_plugin_store::StoreExt;
        if let Ok(store) = app.store("settings.json") {
            if let Some(url) = store.get("api_base_url").and_then(|v| v.as_str().map(|s| s.to_string())) {
                set_api_base_url(&url);
                return;
            }
        }
    }

    let _ = app; // suppress unused warning in release builds
}

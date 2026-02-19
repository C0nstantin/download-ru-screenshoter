use std::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    /// Current screenshot as PNG bytes
    pub current_screenshot: Mutex<Option<Vec<u8>>>,
    /// Screenshot dimensions (width, height)
    pub screenshot_dimensions: Mutex<Option<(u32, u32)>>,
    /// OAuth access token for download.ru
    pub access_token: Mutex<Option<String>>,
}

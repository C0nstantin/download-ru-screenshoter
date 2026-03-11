use std::sync::{Arc, Mutex};

pub struct AppState {
    pub current_screenshot: Mutex<Option<Vec<u8>>>,
    pub screenshot_dimensions: Mutex<Option<(u32, u32)>>,
    pub screen_offset: Mutex<Option<(i32, i32)>>,
    pub access_token: Arc<Mutex<Option<String>>>,
    pub recording_pid: Mutex<Option<u32>>,
    pub recording_path: Mutex<Option<String>>,
    /// Path of the last completed recording (persists after stop for the result window)
    pub last_recording_path: Mutex<Option<String>>,
    /// Saved system input volume before muting (to restore on stop/unmute)
    pub saved_input_volume: Mutex<Option<u32>>,
    pub locale: Mutex<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_screenshot: Mutex::new(None),
            screenshot_dimensions: Mutex::new(None),
            screen_offset: Mutex::new(None),
            access_token: Arc::new(Mutex::new(None)),
            recording_pid: Mutex::new(None),
            recording_path: Mutex::new(None),
            last_recording_path: Mutex::new(None),
            saved_input_volume: Mutex::new(None),
            locale: Mutex::new("ru".to_string()),
        }
    }
}

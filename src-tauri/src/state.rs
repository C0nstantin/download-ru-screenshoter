use std::sync::{Arc, Mutex};

pub struct AppState {
    pub current_screenshot: Mutex<Option<Vec<u8>>>,
    pub screenshot_dimensions: Mutex<Option<(u32, u32)>>,
    pub screen_offset: Mutex<Option<(i32, i32)>>,
    pub access_token: Arc<Mutex<Option<String>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_screenshot: Mutex::new(None),
            screenshot_dimensions: Mutex::new(None),
            screen_offset: Mutex::new(None),
            access_token: Arc::new(Mutex::new(None)),
        }
    }
}

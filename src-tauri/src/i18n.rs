use crate::state::AppState;
use tauri::{AppHandle, Manager};

pub struct Translations {
    // Tray menu
    pub screenshot_region: &'static str,
    pub screenshot_fullscreen: &'static str,
    pub screenshot_window: &'static str,
    pub video_screen: &'static str,
    pub video_region: &'static str,
    pub video_window: &'static str,
    pub settings: &'static str,
    pub quit: &'static str,
    pub stop_recording: &'static str,

    // Recording
    pub no_screen_permission: &'static str,
    pub pick_window_prompt: &'static str,
    pub recording_finished: &'static str,

    // Hotkeys
    pub invalid_hotkey_format: &'static str,
    pub failed_to_register: &'static str,

    // Upload
    pub oauth_title: &'static str,
    pub signout_title: &'static str,
}

pub static RU: Translations = Translations {
    screenshot_region: "Скриншот области",
    screenshot_fullscreen: "Скриншот экрана",
    screenshot_window: "Скриншот окна",
    video_screen: "Запись экрана",
    video_region: "Запись области",
    video_window: "Запись окна",
    settings: "Настройки...",
    quit: "Выйти",
    stop_recording: "Остановить запись",

    no_screen_permission: "Нет разрешения на запись экрана. Добавьте приложение в Системные настройки → Конфиденциальность → Запись экрана",
    pick_window_prompt: "Выберите окно для записи:",
    recording_finished: "Запись завершена",

    invalid_hotkey_format: "Неверный формат горячей клавиши",
    failed_to_register: "Не удалось зарегистрировать",

    oauth_title: "Авторизация download.ru",
    signout_title: "Выход...",
};

pub static EN: Translations = Translations {
    screenshot_region: "Region Screenshot",
    screenshot_fullscreen: "Fullscreen Screenshot",
    screenshot_window: "Window Screenshot",
    video_screen: "Record Screen",
    video_region: "Record Region",
    video_window: "Record Window",
    settings: "Settings...",
    quit: "Quit",
    stop_recording: "Stop Recording",

    no_screen_permission: "No screen recording permission. Add the app in System Settings → Privacy → Screen Recording",
    pick_window_prompt: "Select a window to record:",
    recording_finished: "Recording finished",

    invalid_hotkey_format: "Invalid hotkey format",
    failed_to_register: "Failed to register",

    oauth_title: "download.ru Authorization",
    signout_title: "Logging out...",
};

pub fn current(app: &AppHandle) -> &'static Translations {
    let state = app.state::<AppState>();
    let locale = state.locale.lock().unwrap();
    match locale.as_str() {
        "en" => &EN,
        _ => &RU,
    }
}

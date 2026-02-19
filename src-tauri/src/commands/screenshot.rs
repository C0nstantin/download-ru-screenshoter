use std::io::Cursor;
use std::process::Command;
use screenshots::Screen;
use screenshots::image::ImageFormat;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::state::AppState;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(serde::Serialize)]
pub struct ScreenshotData {
    pub base64: String,
    pub width: u32,
    pub height: u32,
}

/// Capture the primary screen and store in state
#[tauri::command]
pub fn capture_fullscreen(
    _app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ScreenshotData, String> {
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;

    let screen = screens.first().ok_or("No screens found")?;
    let image = screen.capture().map_err(|e| format!("Failed to capture screen: {}", e))?;

    let (width, height) = (image.width(), image.height());

    // Convert to PNG bytes
    let mut png_bytes = Vec::new();
    let mut cursor = Cursor::new(&mut png_bytes);
    image.write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    // Store in state
    {
        let mut current = state.current_screenshot.lock().unwrap();
        *current = Some(png_bytes.clone());
    }
    {
        let mut dims = state.screenshot_dimensions.lock().unwrap();
        *dims = Some((width, height));
    }

    // Return base64 for frontend
    let base64_str = BASE64.encode(&png_bytes);

    Ok(ScreenshotData {
        base64: base64_str,
        width,
        height,
    })
}

/// Crop the current screenshot to a region
#[tauri::command]
pub fn crop_image(
    region: Region,
    state: State<'_, AppState>,
) -> Result<ScreenshotData, String> {
    let png_bytes = {
        let current = state.current_screenshot.lock().unwrap();
        current.clone().ok_or("No screenshot in memory")?
    };

    // Load image from bytes
    let img = screenshots::image::load_from_memory(&png_bytes)
        .map_err(|e| format!("Failed to load image: {}", e))?;

    // Crop
    let cropped = img.crop_imm(region.x, region.y, region.width, region.height);
    let (width, height) = (cropped.width(), cropped.height());

    // Encode back to PNG
    let mut cropped_bytes = Vec::new();
    let mut cursor = Cursor::new(&mut cropped_bytes);
    cropped.write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| format!("Failed to encode cropped PNG: {}", e))?;

    // Update state
    {
        let mut current = state.current_screenshot.lock().unwrap();
        *current = Some(cropped_bytes.clone());
    }
    {
        let mut dims = state.screenshot_dimensions.lock().unwrap();
        *dims = Some((width, height));
    }

    let base64_str = BASE64.encode(&cropped_bytes);

    Ok(ScreenshotData {
        base64: base64_str,
        width,
        height,
    })
}

/// Get current screenshot as base64
#[tauri::command]
pub fn get_current_screenshot(
    state: State<'_, AppState>,
) -> Result<ScreenshotData, String> {
    let png_bytes = {
        let current = state.current_screenshot.lock().unwrap();
        current.clone().ok_or("No screenshot in memory")?
    };

    let dims = {
        let dims = state.screenshot_dimensions.lock().unwrap();
        dims.clone().ok_or("No dimensions stored")?
    };

    let base64_str = BASE64.encode(&png_bytes);

    Ok(ScreenshotData {
        base64: base64_str,
        width: dims.0,
        height: dims.1,
    })
}

/// Save screenshot to file
#[tauri::command]
pub fn save_screenshot(
    path: String,
    image_data: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let png_bytes = if let Some(data) = image_data {
        // Remove data URL prefix if present
        let base64_data = if data.contains(",") {
            data.split(",").last().unwrap_or(&data)
        } else {
            &data
        };
        BASE64.decode(base64_data)
            .map_err(|e| format!("Failed to decode base64: {}", e))?
    } else {
        let current = state.current_screenshot.lock().unwrap();
        current.clone().ok_or("No screenshot to save")?
    };

    std::fs::write(&path, &png_bytes)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

/// Open editor window from frontend
#[tauri::command]
pub fn open_editor(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    println!("open_editor called");

    let dims = {
        let dims = state.screenshot_dimensions.lock().unwrap();
        println!("Dimensions from state: {:?}", dims);
        dims.clone().ok_or("No screenshot dimensions")?
    };

    println!("Creating editor with dimensions: {}x{}", dims.0, dims.1);
    create_editor_window(&app, dims.0, dims.1)
}

/// Start region capture - captures primary screen and shows overlay
pub fn start_region_capture(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();

    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    let screen = screens.iter()
        .find(|s| s.display_info.is_primary)
        .unwrap_or(&screens[0]);

    println!("Region capture on screen: id={}, {}x{}",
        screen.display_info.id, screen.display_info.width, screen.display_info.height);

    let image = screen.capture().map_err(|e| format!("Failed to capture screen: {}", e))?;
    let (width, height) = (image.width(), image.height());

    println!("Captured image: {}x{}", width, height);

    let mut png_bytes = Vec::new();
    let mut cursor = Cursor::new(&mut png_bytes);
    image.write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    {
        let mut current = state.current_screenshot.lock().unwrap();
        *current = Some(png_bytes);
    }
    {
        let mut dims = state.screenshot_dimensions.lock().unwrap();
        *dims = Some((width, height));
    }

    // Create overlay on primary screen
    create_overlay_window(&app, 0, 0, width, height)?;

    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.show();
        let _ = overlay.set_focus();
    }

    Ok(())
}

/// Capture fullscreen and go directly to editor (uses native screencapture)
pub fn capture_fullscreen_and_edit(app: AppHandle) -> Result<(), String> {
    // Use native screencapture for reliable full-screen capture
    // Save to Desktop so user can check the actual capture
    let desktop_path = format!("{}/Desktop/debug_fullscreen.png",
        std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));

    println!("Capturing fullscreen to: {}", desktop_path);

    // -x: no sound, -m: main display only
    let output = Command::new("screencapture")
        .args(["-x", "-m", &desktop_path])
        .output()
        .map_err(|e| format!("Failed to run screencapture: {}", e))?;

    if !output.status.success() {
        return Err(format!("screencapture failed: {:?}", output.stderr));
    }

    let png_bytes = std::fs::read(&desktop_path)
        .map_err(|e| format!("Failed to read screenshot: {}", e))?;

    // Don't delete - leave for debugging
    // let _ = std::fs::remove_file(&desktop_path);

    if png_bytes.is_empty() {
        return Err("Screenshot was empty".to_string());
    }

    let img = screenshots::image::load_from_memory(&png_bytes)
        .map_err(|e| format!("Failed to load image: {}", e))?;

    let (width, height) = (img.width(), img.height());
    println!("Fullscreen capture: {}x{} (file size: {} bytes)", width, height, png_bytes.len());

    let state = app.state::<AppState>();
    {
        let mut current = state.current_screenshot.lock().unwrap();
        *current = Some(png_bytes);
    }
    {
        let mut dims = state.screenshot_dimensions.lock().unwrap();
        *dims = Some((width, height));
    }

    // Create and show editor window
    create_editor_window(&app, width, height)?;

    Ok(())
}

/// Capture a window by clicking on it (using native macOS screencapture -w)
pub fn capture_window_and_edit(app: AppHandle) -> Result<(), String> {
    println!("capture_window_and_edit called");

    // Use native screencapture with -w for window selection
    let temp_path = format!("/tmp/screenshot_window_{}.png", std::process::id());

    // Spawn screencapture in background thread
    let temp_path_clone = temp_path.clone();
    let app_clone = app.clone();

    std::thread::spawn(move || {
        println!("Window capture thread started");

        // -x: no sound, -w: window mode (click to select), -o: no shadow
        let output = Command::new("screencapture")
            .args(["-x", "-w", "-o", &temp_path_clone])
            .output();

        println!("screencapture finished: {:?}", output.as_ref().map(|o| o.status));

        match output {
            Ok(result) if result.status.success() => {
                if let Ok(png_bytes) = std::fs::read(&temp_path_clone) {
                    let _ = std::fs::remove_file(&temp_path_clone);

                    if png_bytes.is_empty() {
                        println!("Window capture: empty file, user cancelled");
                        return;
                    }

                    println!("Window capture: {} bytes", png_bytes.len());

                    if let Ok(img) = screenshots::image::load_from_memory(&png_bytes) {
                        let (width, height) = (img.width(), img.height());
                        println!("Window capture image: {}x{}", width, height);

                        let state = app_clone.state::<AppState>();

                        {
                            let mut current = state.current_screenshot.lock().unwrap();
                            *current = Some(png_bytes);
                        }
                        {
                            let mut dims = state.screenshot_dimensions.lock().unwrap();
                            *dims = Some((width, height));
                        }

                        println!("Creating editor window for window capture");
                        let result = create_editor_window(&app_clone, width, height);
                        println!("create_editor_window result: {:?}", result);
                    }
                }
            }
            Ok(result) => {
                println!("screencapture exited with non-zero: {:?}", result.status);
            }
            Err(e) => {
                println!("screencapture error: {}", e);
            }
        }
    });

    Ok(())
}

fn create_overlay_window(app: &AppHandle, _offset_x: i32, _offset_y: i32, _width: u32, _height: u32) -> Result<(), String> {
    // Close existing if any
    if let Some(existing) = app.get_webview_window("overlay") {
        let _ = existing.close();
    }

    // Get primary screen for overlay (simpler approach - just cover primary screen)
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    let primary = screens.iter()
        .find(|s| s.display_info.is_primary)
        .unwrap_or(&screens[0]);

    let info = &primary.display_info;
    let scale = info.scale_factor as f64;

    // Use screen dimensions directly (already in physical pixels for display_info)
    let logical_width = info.width as f64 / scale;
    let logical_height = info.height as f64 / scale;
    let logical_x = info.x as f64;
    let logical_y = info.y as f64;

    println!("Creating overlay on primary screen: {}x{} at ({}, {}), scale={}",
             logical_width, logical_height, logical_x, logical_y, scale);

    WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("index.html#/overlay".into()))
        .title("")
        .inner_size(logical_width, logical_height)
        .position(logical_x, logical_y)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .build()
        .map_err(|e| format!("Failed to create overlay: {}", e))?;

    Ok(())
}

fn create_editor_window(app: &AppHandle, width: u32, height: u32) -> Result<(), String> {
    println!("Creating editor window for image {}x{}", width, height);

    // Calculate window size - add padding for toolbar and actions
    let win_width = ((width as f64 * 0.8) + 100.0).min(1400.0).max(600.0);
    let win_height = ((height as f64 * 0.8) + 200.0).min(900.0).max(500.0);

    // Always destroy existing window and create fresh one
    if let Some(existing) = app.get_webview_window("editor") {
        println!("Destroying existing editor window");
        let _ = existing.destroy();
        // Wait for window to be destroyed
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    println!("Editor window size: {}x{}", win_width, win_height);

    // Add timestamp to URL to prevent caching
    let url = format!("index.html?t={}#/editor", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis());

    let result = WebviewWindowBuilder::new(app, "editor", WebviewUrl::App(url.into()))
        .title("Редактор скриншота")
        .inner_size(win_width, win_height)
        .resizable(true)
        .center()
        .build();

    match result {
        Ok(window) => {
            println!("Editor window created successfully");
            let _ = window.show();
            let _ = window.set_focus();
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to create editor window: {}", e);
            Err(format!("Failed to create editor: {}", e))
        }
    }
}

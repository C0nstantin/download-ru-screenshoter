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

#[derive(serde::Serialize, Clone)]
pub struct DisplayInfo {
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

/// Get list of all displays
#[tauri::command]
pub fn get_displays() -> Result<Vec<DisplayInfo>, String> {
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;

    let displays: Vec<DisplayInfo> = screens.iter().enumerate().map(|(i, s)| {
        DisplayInfo {
            id: s.display_info.id,
            name: format!("Display {}", i + 1),
            x: s.display_info.x,
            y: s.display_info.y,
            width: s.display_info.width,
            height: s.display_info.height,
            is_primary: s.display_info.is_primary,
        }
    }).collect();

    #[cfg(debug_assertions)]
    println!("Found {} displays: {:?}", displays.len(), displays.iter().map(|d| format!("{}x{} at ({},{})", d.width, d.height, d.x, d.y)).collect::<Vec<_>>());

    Ok(displays)
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
    // Block path traversal
    if path.contains("..") {
        return Err("Access denied: path traversal detected".into());
    }
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

    // Small delay to ensure state is fully updated
    std::thread::sleep(std::time::Duration::from_millis(50));

    create_editor_window(&app, dims.0, dims.1)
}

/// Start region capture - platform specific implementation
pub fn start_region_capture(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        start_region_capture_macos(app)
    }

    #[cfg(not(target_os = "macos"))]
    {
        start_region_capture_overlay(app)
    }
}

/// macOS: Use native screencapture -i (interactive) which handles multi-monitor
#[cfg(target_os = "macos")]
fn start_region_capture_macos(app: AppHandle) -> Result<(), String> {
    println!("Starting native region capture (screencapture -i)...");

    let temp_path = format!("/tmp/screenshot_region_{}.png", uuid::Uuid::new_v4());
    let temp_path_clone = temp_path.clone();
    let app_clone = app.clone();

    std::thread::spawn(move || {
        let output = Command::new("screencapture")
            .args(["-i", "-x", &temp_path_clone])
            .output();

        match output {
            Ok(result) if result.status.success() => {
                if let Ok(png_bytes) = std::fs::read(&temp_path_clone) {
                    let _ = std::fs::remove_file(&temp_path_clone);

                    if png_bytes.is_empty() {
                        println!("Region capture: cancelled by user");
                        return;
                    }

                    if let Ok(img) = screenshots::image::load_from_memory(&png_bytes) {
                        let (width, height) = (img.width(), img.height());
                        let state = app_clone.state::<AppState>();

                        {
                            let mut current = state.current_screenshot.lock().unwrap();
                            *current = Some(png_bytes);
                        }
                        {
                            let mut dims = state.screenshot_dimensions.lock().unwrap();
                            *dims = Some((width, height));
                        }

                        let _ = create_editor_window(&app_clone, width, height);
                    }
                }
            }
            _ => {
                println!("Region capture: cancelled or failed");
            }
        }
    });

    Ok(())
}

/// Same as start_region_capture_overlay but opens overlay in video selection mode
pub fn start_region_capture_overlay_video(app: AppHandle) -> Result<(), String> {
    // Reuse all the screen capture + composite logic, just change the overlay URL
    let state = app.state::<AppState>();
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    if screens.is_empty() { return Err("No screens found".into()); }

    let mut captures: Vec<(screenshots::image::RgbaImage, i32, i32, f32)> = Vec::new();
    for screen in &screens {
        let info = &screen.display_info;
        if let Ok(img) = screen.capture() {
            captures.push((img, info.x, info.y, info.scale_factor));
        }
    }
    if captures.is_empty() { return Err("Failed to capture any screen".into()); }

    // Physical pixel bounding box (for image compositing)
    let mut phys_min_x = i32::MAX; let mut phys_min_y = i32::MAX;
    let mut phys_max_x = i32::MIN; let mut phys_max_y = i32::MIN;
    // Logical coordinate bounding box (for window position and screen_offset)
    let mut log_min_x = i32::MAX; let mut log_min_y = i32::MAX;
    let mut log_max_x = i32::MIN; let mut log_max_y = i32::MIN;

    for (img, x, y, scale) in &captures {
        // Physical pixels for image compositing
        let px = (*x as f32 * scale) as i32;
        let py = (*y as f32 * scale) as i32;
        phys_min_x = phys_min_x.min(px); phys_min_y = phys_min_y.min(py);
        phys_max_x = phys_max_x.max(px + img.width() as i32);
        phys_max_y = phys_max_y.max(py + img.height() as i32);
        // Logical coordinates for window positioning
        log_min_x = log_min_x.min(*x); log_min_y = log_min_y.min(*y);
        log_max_x = log_max_x.max(*x + (img.width() as f32 / scale) as i32);
        log_max_y = log_max_y.max(*y + (img.height() as f32 / scale) as i32);
    }
    let total_width = (phys_max_x - phys_min_x) as u32;
    let total_height = (phys_max_y - phys_min_y) as u32;
    let logical_width = (log_max_x - log_min_x) as f64;
    let logical_height = (log_max_y - log_min_y) as f64;

    let mut combined = screenshots::image::RgbaImage::new(total_width, total_height);
    for (img, x, y, scale) in &captures {
        let px = (*x as f32 * scale) as i32;
        let py = (*y as f32 * scale) as i32;
        let ox = (px - phys_min_x) as u32; let oy = (py - phys_min_y) as u32;
        for qy in 0..img.height() {
            for qx in 0..img.width() {
                let pixel = img.get_pixel(qx, qy);
                if ox + qx < total_width && oy + qy < total_height {
                    combined.put_pixel(ox + qx, oy + qy, *pixel);
                }
            }
        }
    }

    let (w, h) = (combined.width(), combined.height());
    let mut png_bytes = Vec::new();
    let mut cursor = Cursor::new(&mut png_bytes);
    combined.write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| format!("PNG encode error: {}", e))?;

    // Store logical offset — used by start_video_recording to translate CSS coords to screen coords
    { let mut o = state.screen_offset.lock().unwrap(); *o = Some((log_min_x, log_min_y)); }
    { let mut s = state.current_screenshot.lock().unwrap(); *s = Some(png_bytes); }
    { let mut d = state.screenshot_dimensions.lock().unwrap(); *d = Some((w, h)); }

    if let Some(existing) = app.get_webview_window("overlay") {
        let _ = existing.destroy();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    tauri::WebviewWindowBuilder::new(
        &app, "overlay",
        tauri::WebviewUrl::App("index.html#/overlay-video".into()),
    )
    .position(log_min_x as f64, log_min_y as f64)
    .inner_size(logical_width, logical_height)
    .always_on_top(true).skip_taskbar(true).decorations(false).focused(true)
    .build()
    .map_err(|e| format!("Failed to create video overlay: {}", e))?;

    Ok(())
}

pub fn start_region_capture_overlay(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;

    if screens.is_empty() {
        return Err("No screens found".to_string());
    }

    // Capture all screens and composite them
    let mut captures: Vec<(screenshots::image::RgbaImage, i32, i32, f32)> = Vec::new();

    for screen in &screens {
        let info = &screen.display_info;
        if let Ok(img) = screen.capture() {
            captures.push((img, info.x, info.y, info.scale_factor));
        }
    }

    if captures.is_empty() {
        return Err("Failed to capture any screen".to_string());
    }

    // Calculate bounding box
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for (img, x, y, scale) in &captures {
        let phys_x = (*x as f32 * scale) as i32;
        let phys_y = (*y as f32 * scale) as i32;
        min_x = min_x.min(phys_x);
        min_y = min_y.min(phys_y);
        max_x = max_x.max(phys_x + img.width() as i32);
        max_y = max_y.max(phys_y + img.height() as i32);
    }

    let total_width = (max_x - min_x) as u32;
    let total_height = (max_y - min_y) as u32;

    // Create combined image
    let mut combined = screenshots::image::RgbaImage::new(total_width, total_height);

    for (img, x, y, scale) in &captures {
        let phys_x = (*x as f32 * scale) as i32;
        let phys_y = (*y as f32 * scale) as i32;
        let offset_x = (phys_x - min_x) as u32;
        let offset_y = (phys_y - min_y) as u32;

        for py in 0..img.height() {
            for px in 0..img.width() {
                let pixel = img.get_pixel(px, py);
                let dest_x = offset_x + px;
                let dest_y = offset_y + py;
                if dest_x < total_width && dest_y < total_height {
                    combined.put_pixel(dest_x, dest_y, *pixel);
                }
            }
        }
    }

    let (width, height) = (combined.width(), combined.height());

    // Encode to PNG
    let mut png_bytes = Vec::new();
    let mut cursor = Cursor::new(&mut png_bytes);
    combined.write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    // Store in state
    {
        let mut offset = state.screen_offset.lock().unwrap();
        *offset = Some((min_x, min_y));
    }
    {
        let mut current = state.current_screenshot.lock().unwrap();
        *current = Some(png_bytes);
    }
    {
        let mut dims = state.screenshot_dimensions.lock().unwrap();
        *dims = Some((width, height));
    }

    // Create overlay window spanning all screens
    create_overlay_window(&app, min_x, min_y, total_width, total_height)?;

    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.show();
        let _ = overlay.set_focus();
    }

    Ok(())
}

/// Capture fullscreen and go directly to editor (tauri command wrapper)
#[tauri::command]
pub fn capture_fullscreen_and_edit(app: AppHandle, display_id: Option<u32>) -> Result<(), String> {
    capture_fullscreen_and_edit_internal(app, display_id)
}

/// Capture fullscreen and go directly to editor
/// If display_id is None, captures all displays. If Some(id), captures specific display.
pub fn capture_fullscreen_and_edit_internal(app: AppHandle, display_id: Option<u32>) -> Result<(), String> {
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;

    if screens.is_empty() {
        return Err("No screens found".to_string());
    }

    // If no specific display and only one screen, just capture it directly (fast path)
    let effective_display_id = if display_id.is_none() && screens.len() == 1 {
        Some(screens[0].display_info.id)
    } else {
        display_id
    };

    let (png_bytes, width, height) = if let Some(id) = effective_display_id {
        // Capture specific display
        let screen = screens.iter()
            .find(|s| s.display_info.id == id)
            .ok_or_else(|| format!("Display {} not found", id))?;

        println!("Capturing display {}: {}x{}", id, screen.display_info.width, screen.display_info.height);

        let image = screen.capture().map_err(|e| format!("Failed to capture screen: {}", e))?;
        let (w, h) = (image.width(), image.height());

        let mut bytes = Vec::new();
        let mut cursor = Cursor::new(&mut bytes);
        image.write_to(&mut cursor, ImageFormat::Png)
            .map_err(|e| format!("Failed to encode PNG: {}", e))?;

        (bytes, w, h)
    } else {
        // Capture all displays combined - use same logic as start_region_capture
        let mut captures: Vec<(screenshots::image::RgbaImage, i32, i32, f32)> = Vec::new();

        for screen in &screens {
            let info = &screen.display_info;
            if let Ok(img) = screen.capture() {
                println!("Screen {}: captured {}x{} (logical {}x{} at ({},{}) scale={})",
                    info.id, img.width(), img.height(), info.width, info.height, info.x, info.y, info.scale_factor);
                captures.push((img, info.x, info.y, info.scale_factor));
            }
        }

        if captures.is_empty() {
            return Err("Failed to capture any screen".to_string());
        }

        // Calculate bounding box using physical pixels
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for (img, x, y, scale) in &captures {
            let phys_x = (*x as f32 * scale) as i32;
            let phys_y = (*y as f32 * scale) as i32;
            min_x = min_x.min(phys_x);
            min_y = min_y.min(phys_y);
            max_x = max_x.max(phys_x + img.width() as i32);
            max_y = max_y.max(phys_y + img.height() as i32);
        }

        let total_width = (max_x - min_x) as u32;
        let total_height = (max_y - min_y) as u32;

        println!("Capturing all {} displays: total {}x{} pixels", captures.len(), total_width, total_height);

        let mut combined = screenshots::image::RgbaImage::new(total_width, total_height);

        for (img, x, y, scale) in &captures {
            let phys_x = (*x as f32 * scale) as i32;
            let phys_y = (*y as f32 * scale) as i32;
            let offset_x = (phys_x - min_x) as u32;
            let offset_y = (phys_y - min_y) as u32;

            use screenshots::image::GenericImage;
            let _ = combined.copy_from(&*img, offset_x, offset_y);
        }

        let (w, h) = (combined.width(), combined.height());
        let mut bytes = Vec::new();
        let mut cursor = Cursor::new(&mut bytes);
        combined.write_to(&mut cursor, ImageFormat::Png)
            .map_err(|e| format!("Failed to encode PNG: {}", e))?;

        (bytes, w, h)
    };

    println!("Fullscreen capture: {}x{} ({} bytes)", width, height, png_bytes.len());

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

/// Capture a window by clicking on it - platform specific
pub fn capture_window_and_edit(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        capture_window_macos(app)
    }

    #[cfg(not(target_os = "macos"))]
    {
        // On Linux/Windows, fall back to region capture for now
        // TODO: Implement proper window selection using platform APIs
        println!("Window capture not implemented on this platform, using region capture");
        start_region_capture(app)
    }
}

/// macOS: Use native screencapture -w for window selection
#[cfg(target_os = "macos")]
fn capture_window_macos(app: AppHandle) -> Result<(), String> {
    println!("capture_window_macos called");

    let temp_path = format!("/tmp/screenshot_window_{}.png", uuid::Uuid::new_v4());
    let temp_path_clone = temp_path.clone();
    let app_clone = app.clone();

    std::thread::spawn(move || {
        let output = Command::new("screencapture")
            .args(["-x", "-w", "-o", &temp_path_clone])
            .output();

        match output {
            Ok(result) if result.status.success() => {
                if let Ok(png_bytes) = std::fs::read(&temp_path_clone) {
                    let _ = std::fs::remove_file(&temp_path_clone);

                    if png_bytes.is_empty() {
                        println!("Window capture: cancelled by user");
                        return;
                    }

                    if let Ok(img) = screenshots::image::load_from_memory(&png_bytes) {
                        let (width, height) = (img.width(), img.height());
                        let state = app_clone.state::<AppState>();

                        {
                            let mut current = state.current_screenshot.lock().unwrap();
                            *current = Some(png_bytes);
                        }
                        {
                            let mut dims = state.screenshot_dimensions.lock().unwrap();
                            *dims = Some((width, height));
                        }

                        let _ = create_editor_window(&app_clone, width, height);
                    }
                }
            }
            Ok(result) => {
                println!("screencapture exited with: {:?}", result.status);
            }
            Err(e) => {
                println!("screencapture error: {}", e);
            }
        }
    });

    Ok(())
}

fn create_overlay_window(app: &AppHandle, _offset_x: i32, _offset_y: i32, _total_width: u32, _total_height: u32) -> Result<(), String> {
    // Close existing if any
    if let Some(existing) = app.get_webview_window("overlay") {
        let _ = existing.close();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Calculate logical bounding box of all screens
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;

    println!("=== Creating overlay for {} screens ===", screens.len());

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for screen in &screens {
        let info = &screen.display_info;
        println!("  Screen {}: {}x{} at ({}, {}), scale={}, primary={}",
            info.id, info.width, info.height, info.x, info.y, info.scale_factor, info.is_primary);

        min_x = min_x.min(info.x);
        min_y = min_y.min(info.y);
        max_x = max_x.max(info.x + info.width as i32);
        max_y = max_y.max(info.y + info.height as i32);
    }

    let logical_width = (max_x - min_x) as f64;
    let logical_height = (max_y - min_y) as f64;
    let logical_x = min_x as f64;
    let logical_y = min_y as f64;

    println!("  Bounding box: min=({}, {}), max=({}, {})", min_x, min_y, max_x, max_y);
    println!("  Overlay window: {}x{} at ({}, {})", logical_width, logical_height, logical_x, logical_y);

    // On Windows, transparent windows don't receive input events properly
    // So we create a non-transparent window and use the screenshot as background
    #[cfg(target_os = "windows")]
    let use_transparent = false;
    #[cfg(not(target_os = "windows"))]
    let use_transparent = true;

    let mut builder = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("index.html#/overlay".into()))
        .title("")
        .inner_size(logical_width, logical_height)
        .position(logical_x, logical_y)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(true);

    if use_transparent {
        builder = builder.transparent(true);
    }

    #[cfg(not(target_os = "windows"))]
    {
        builder = builder.visible_on_all_workspaces(true);
    }

    builder.build()
        .map_err(|e| format!("Failed to create overlay: {}", e))?;

    Ok(())
}

fn create_editor_window(app: &AppHandle, width: u32, height: u32) -> Result<(), String> {
    println!("Creating editor window for image {}x{}", width, height);
    // Show in Dock and Cmd+Tab when editor opens
    #[cfg(target_os = "macos")]
    crate::activate_as_regular(app);

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

    // Add timestamp to URL to prevent caching (placed after hash for WebView2 compatibility)
    let url = format!("index.html#/editor?t={}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis());

    let tr = crate::i18n::current(app);
    let result = WebviewWindowBuilder::new(app, "editor", WebviewUrl::App(url.into()))
        .title(tr.editor_title)
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

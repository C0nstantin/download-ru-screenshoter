use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use crate::state::AppState;

/// Open region selection overlay in video recording mode.
/// Captures full screen first, then shows overlay for user to select region.
#[tauri::command]
pub fn start_video_capture(app: AppHandle) -> Result<(), String> {
    // On macOS the normal region capture uses interactive screencapture -i,
    // but for video we need coordinates first, so always use the overlay flow.
    // Capture all screens into state, then open overlay in video mode.
    capture_screens_for_video(app)
}

fn capture_screens_for_video(app: AppHandle) -> Result<(), String> {
    use screenshots::Screen;
    use screenshots::image::{RgbaImage, GenericImage, ImageEncoder, ColorType};
    use tauri::Manager;

    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    if screens.is_empty() {
        return Err("No screens found".into());
    }

    // Capture all screens into a composite image (same as start_region_capture_overlay)
    let min_x = screens.iter().map(|s| s.display_info.x).min().unwrap_or(0);
    let min_y = screens.iter().map(|s| s.display_info.y).min().unwrap_or(0);
    let max_x = screens.iter().map(|s| s.display_info.x + s.display_info.width as i32).max().unwrap_or(1920);
    let max_y = screens.iter().map(|s| s.display_info.y + s.display_info.height as i32).max().unwrap_or(1080);
    let total_w = (max_x - min_x) as u32;
    let total_h = (max_y - min_y) as u32;

    let mut composite = RgbaImage::new(total_w, total_h);
    for screen in &screens {
        if let Ok(img) = screen.capture() {
            let buffer: RgbaImage = img;
            let ox = (screen.display_info.x - min_x) as u32;
            let oy = (screen.display_info.y - min_y) as u32;
            let _ = composite.copy_from(&buffer, ox, oy);
        }
    }

    let mut png_bytes = Vec::new();
    let encoder = screenshots::image::codecs::png::PngEncoder::new(&mut png_bytes);
    encoder.write_image(composite.as_raw(), total_w, total_h, ColorType::Rgba8.into())
        .map_err(|e| format!("PNG encode error: {}", e))?;

    let state = app.state::<crate::state::AppState>();
    {
        let mut ss = state.current_screenshot.lock().unwrap();
        *ss = Some(png_bytes);
    }
    {
        let mut dims = state.screenshot_dimensions.lock().unwrap();
        *dims = Some((total_w, total_h));
    }

    // Close existing overlay, open new one in video mode
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.destroy();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    tauri::WebviewWindowBuilder::new(
        &app,
        "overlay",
        tauri::WebviewUrl::App("index.html#/overlay-video".into()),
    )
    .fullscreen(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .decorations(false)
    .focused(true)
    .build()
    .map_err(|e| format!("Failed to create video overlay: {}", e))?;

    Ok(())
}

/// Start recording the selected region (called from overlay after region selection)
#[tauri::command]
pub fn start_video_recording(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let output_path = format!("/tmp/recording_{}.mov", timestamp);

        // screencapture -v -R x,y,w,h output.mov
        let child = std::process::Command::new("screencapture")
            .args([
                "-v",
                &format!("-R{},{},{},{}", x, y, width, height),
                &output_path,
            ])
            .spawn()
            .map_err(|e| format!("Failed to start screencapture: {}", e))?;

        {
            let mut proc = state.recording_process.lock().unwrap();
            *proc = Some(child);
        }
        {
            let mut path = state.recording_path.lock().unwrap();
            *path = Some(output_path);
        }

        // Show recording indicator window
        show_recording_window(&app)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        // TODO: Linux/Windows — see VIDEO_RECORDING.md
        let _ = (x, y, width, height);
        let _ = app;
        return Err("Video recording not yet implemented on this platform. See VIDEO_RECORDING.md".into());
    }

    Ok(())
}

/// Stop recording, return path to the .mov file
#[tauri::command]
pub fn stop_video_recording(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path = {
        let p = state.recording_path.lock().unwrap();
        p.clone().ok_or("No recording in progress")?
    };

    // Send SIGINT to screencapture so it finalizes the file
    {
        let mut proc = state.recording_process.lock().unwrap();
        if let Some(child) = proc.as_ref() {
            let pid = child.id().to_string();
            let _ = std::process::Command::new("kill")
                .args(["-2", &pid])
                .status();
        }
        // Wait for process to exit so file is finalized
        if let Some(mut child) = proc.take() {
            let _ = child.wait();
        }
    }

    // Clear recording path
    {
        let mut p = state.recording_path.lock().unwrap();
        *p = None;
    }

    // Close recording indicator
    if let Some(w) = app.get_webview_window("recording") {
        let _ = w.close().ok();
    }

    // Restore normal activation policy
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    Ok(path)
}

/// Check if recording is in progress
#[tauri::command]
pub fn is_recording(state: State<'_, AppState>) -> bool {
    state.recording_process.lock().unwrap().is_some()
}

/// Move recorded video file to destination chosen by user
#[tauri::command]
pub fn move_recording(src_path: String, dst_path: String) -> Result<(), String> {
    std::fs::rename(&src_path, &dst_path)
        .or_else(|_| std::fs::copy(&src_path, &dst_path).map(|_| {
            let _ = std::fs::remove_file(&src_path);
        }))
        .map_err(|e| format!("Failed to move file: {}", e))
}

fn show_recording_window(app: &AppHandle) -> Result<(), String> {
    // Show in Dock while recording
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);

    let win = WebviewWindowBuilder::new(
        app,
        "recording",
        WebviewUrl::App("index.html#/recording".into()),
    )
    .title("Запись")
    .inner_size(220.0, 64.0)
    .always_on_top(true)
    .decorations(false)
    .skip_taskbar(true)
    .resizable(false)
    .build()
    .map_err(|e| format!("Failed to create recording window: {}", e))?;

    // Position in top-right corner
    if let Ok(monitor) = win.current_monitor() {
        if let Some(monitor) = monitor {
            let size = monitor.size();
            let scale = monitor.scale_factor();
            let win_w = 220.0;
            let margin = 20.0;
            let x = (size.width as f64 / scale - win_w - margin) as i32;
            let y = 20;
            let _ = win.set_position(tauri::PhysicalPosition::new(
                (x as f64 * scale) as i32,
                (y as f64 * scale) as i32,
            ));
        }
    }

    Ok(())
}

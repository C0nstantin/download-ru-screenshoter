use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use crate::state::AppState;

/// Start video capture using native macOS UI (screencapture -v).
/// Opens the same picker as Cmd+Shift+5 — user selects area/window/screen.
/// User stops recording via macOS menu bar stop button.
/// When screencapture exits, we automatically show the result window.
#[tauri::command]
pub fn start_video_capture(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let output_path = format!("/tmp/recording_{}.mov", timestamp);

        let mut child = std::process::Command::new("screencapture")
            .args(["-v", &output_path])
            .spawn()
            .map_err(|e| format!("Failed to start screencapture: {}", e))?;

        // Store PID so stop_video_recording can kill the process
        {
            let mut pid = state.recording_pid.lock().unwrap();
            *pid = Some(child.id());
        }
        {
            let mut path = state.recording_path.lock().unwrap();
            *path = Some(output_path.clone());
        }
        {
            let mut last = state.last_recording_path.lock().unwrap();
            *last = Some(output_path.clone());
        }

        // Monitor process in background — when screencapture exits, show result
        let app_clone = app.clone();
        let recording_path = output_path.clone();
        // Show our Stop button while recording
        show_recording_window(&app)?;

        // Monitor process in background — show result window when screencapture exits
        std::thread::spawn(move || {
            let _ = child.wait();
            std::thread::sleep(std::time::Duration::from_millis(500));

            let state = app_clone.state::<AppState>();

            { let mut p = state.recording_pid.lock().unwrap(); *p = None; }
            { let mut p = state.recording_path.lock().unwrap(); *p = None; }

            if std::fs::metadata(&recording_path).is_ok() {
                { let mut l = state.last_recording_path.lock().unwrap(); *l = Some(recording_path.clone()); }
                let _ = open_video_result_window(&app_clone, &recording_path);
            } else {
                eprintln!("Recording cancelled — no file created");
                let mut l = state.last_recording_path.lock().unwrap();
                *l = None;
            }
        });
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("Video recording not yet implemented on this platform. See VIDEO_RECORDING.md".into())
    }
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

    // Send SIGINT to screencapture via stored PID
    {
        let mut pid_guard = state.recording_pid.lock().unwrap();
        if let Some(pid) = pid_guard.take() {
            let _ = std::process::Command::new("kill")
                .args(["-2", &pid.to_string()])
                .status();
        }
    }

    // Clear active recording path, save as last_recording_path for result window
    {
        let mut p = state.recording_path.lock().unwrap();
        *p = None;
    }
    {
        let mut last = state.last_recording_path.lock().unwrap();
        *last = Some(path.clone());
    }

    // Close recording indicator
    if let Some(w) = app.get_webview_window("recording") {
        let _ = w.close().ok();
    }

    // Give screencapture a moment to flush the file to disk
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Only open result window if file actually exists
    if std::fs::metadata(&path).is_ok() {
        open_video_result_window(&app, &path)?;
    } else {
        // File not created (user cancelled picker or recording too short)
        eprintln!("Recording file not found: {} — user may have cancelled", path);
        {
            let mut last = state.last_recording_path.lock().unwrap();
            *last = None;
        }
    }

    Ok(path)
}

/// Check if recording is in progress
#[tauri::command]
pub fn is_recording(state: State<'_, AppState>) -> bool {
    state.recording_process.lock().unwrap().is_some()
}

/// Get path of the last recording (stored temporarily after stop)
#[tauri::command]
pub fn get_last_recording_path(state: State<'_, AppState>) -> Result<String, String> {
    // After stop_video_recording the path was cleared; we store it separately
    let p = state.last_recording_path.lock().unwrap();
    p.clone().ok_or("No recent recording found".into())
}

/// Delete a recording file
#[tauri::command]
pub fn delete_recording(path: String) -> Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| format!("Failed to delete: {}", e))
}

/// Get info about a recorded video file
#[tauri::command]
pub fn get_video_info(path: String) -> Result<serde_json::Value, String> {
    let metadata = std::fs::metadata(&path)
        .map_err(|e| format!("File not found: {}", e))?;
    Ok(serde_json::json!({
        "path": path,
        "size": metadata.len(),
        "name": std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("recording.mov"),
    }))
}

/// Convert video using macOS built-in avconvert.
/// preset: "high" | "medium" | "low" (maps to avconvert presets)
/// start_sec / duration_sec: optional trim (0.0 means no trim)
#[tauri::command]
pub async fn convert_to_mp4(
    src_path: String,
    preset: Option<String>,
    start_sec: Option<f64>,
    duration_sec: Option<f64>,
) -> Result<String, String> {
    let dst_path = src_path
        .replace(".mov", "_converted.mp4")
        .replace(".MOV", "_converted.mp4");

    #[cfg(target_os = "macos")]
    {
        let avconvert_preset = match preset.as_deref().unwrap_or("high") {
            "low"    => "Preset1280x720",
            "medium" => "Preset1920x1080",
            _        => "PresetHighestQuality",
        };

        let mut args = vec![
            "-s".to_string(), src_path.clone(),
            "-o".to_string(), dst_path.clone(),
            "-p".to_string(), avconvert_preset.to_string(),
            "--replace".to_string(),
        ];

        if let Some(start) = start_sec.filter(|&s| s > 0.0) {
            args.push("--start".to_string());
            args.push(start.to_string());
        }
        if let Some(dur) = duration_sec.filter(|&d| d > 0.0) {
            args.push("--duration".to_string());
            args.push(dur.to_string());
        }

        let output = tokio::process::Command::new("avconvert")
            .args(&args)
            .output()
            .await
            .map_err(|e| format!("avconvert failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Conversion failed: {}", stderr));
        }
        return Ok(dst_path);
    }

    #[cfg(not(target_os = "macos"))]
    Err("Conversion not yet supported on this platform".into())
}

/// Upload video file to download.ru
#[tauri::command]
pub async fn upload_video_to_download(
    file_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let token = {
        let t = state.access_token.lock().unwrap();
        t.clone().ok_or("No access token. Please login in settings.")?
    };

    let file_bytes = std::fs::read(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("recording.mov")
        .to_string();

    let mime = if filename.ends_with(".mp4") { "video/mp4" } else { "video/quicktime" };

    let client = reqwest::Client::new();

    // Get .screenshots folder id (reuse existing logic)
    let parent_id = crate::commands::upload::get_or_create_screenshots_folder_pub(&client, &token).await?;

    let file_part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(filename)
        .mime_str(mime)
        .map_err(|e| format!("MIME error: {}", e))?;

    let form = reqwest::multipart::Form::new().part("files[]", file_part);

    let url = format!("https://download.ru/fast_upload?parent_id={}", parent_id);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Upload failed {}: {}", status, body));
    }

    // Parse secure_url from response
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Parse error: {}: {}", e, &body[..body.len().min(200)]))?;

    let secure_url = json["object"]["secure_url"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let url = if secure_url.starts_with('/') {
        format!("https://download.ru{}", secure_url)
    } else {
        secure_url
    };

    Ok(url)
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

fn open_video_result_window(app: &AppHandle, _path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);

    if let Some(w) = app.get_webview_window("video-result") {
        let _ = w.destroy();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    WebviewWindowBuilder::new(
        app,
        "video-result",
        WebviewUrl::App("index.html#/video-result".into()),
    )
    .title("Запись завершена")
    .inner_size(560.0, 480.0)
    .resizable(true)
    .center()
    .build()
    .map_err(|e| format!("Failed to open video result: {}", e))?;

    Ok(())
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

use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use crate::state::AppState;

/// Validate that a file path is a recording in /tmp (our own files).
/// Prevents path traversal attacks from frontend.
fn validate_recording_path(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    let filename = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let in_tmp = p.parent() == Some(std::path::Path::new("/tmp"));
    let is_recording = filename.starts_with("recording_")
        && (filename.ends_with(".mov") || filename.ends_with(".mp4"));
    if !in_tmp || !is_recording || path.contains("..") {
        return Err(format!("Access denied: path not allowed: {}", path));
    }
    Ok(())
}

/// Shared helper: launch screencapture with given args, monitor process, show result on exit.
#[cfg(target_os = "macos")]
fn launch_screencapture(
    app: &AppHandle,
    state: &State<'_, AppState>,
    args: &[&str],
    output_path: &str,
) -> Result<(), String> {
    let mut child = std::process::Command::new("screencapture")
        .args(args)
        .spawn()
        .map_err(|e| format!("Failed to start screencapture: {}", e))?;

    // Store PID so stop_video_recording can kill the process
    {
        let mut pid = state.recording_pid.lock().unwrap();
        *pid = Some(child.id());
    }
    {
        let mut path = state.recording_path.lock().unwrap();
        *path = Some(output_path.to_string());
    }
    {
        let mut last = state.last_recording_path.lock().unwrap();
        *last = Some(output_path.to_string());
    }

    // Show our Stop button while recording
    show_recording_window(app)?;

    // Monitor process in background — show result window when screencapture exits
    let app_clone = app.clone();
    let recording_path = output_path.to_string();
    std::thread::spawn(move || {
        let _ = child.wait();
        std::thread::sleep(std::time::Duration::from_millis(500));

        let state = app_clone.state::<AppState>();

        { let mut p = state.recording_pid.lock().unwrap(); *p = None; }
        { let mut p = state.recording_path.lock().unwrap(); *p = None; }

        // Restore mic volume if it was muted
        #[cfg(target_os = "macos")]
        {
            let saved = { state.saved_input_volume.lock().unwrap().take() };
            if let Some(vol) = saved {
                let _ = std::process::Command::new("osascript")
                    .args(["-e", &format!("set volume input volume {}", vol)])
                    .status();
            }
        }

        // Close the recording indicator window
        if let Some(w) = app_clone.get_webview_window("recording") {
            let _ = w.close();
        }

        if std::fs::metadata(&recording_path).is_ok() {
            { let mut l = state.last_recording_path.lock().unwrap(); *l = Some(recording_path.clone()); }
            let _ = open_video_result_window(&app_clone, &recording_path);
        } else {
            eprintln!("Recording cancelled — no file created");
            let mut l = state.last_recording_path.lock().unwrap();
            *l = None;
        }
    });

    Ok(())
}

fn make_output_path() -> String {
    format!("/tmp/recording_{}.mov", uuid::Uuid::new_v4())
}

/// Start full-screen video capture using native macOS UI (screencapture -v).
/// Opens the same picker as Cmd+Shift+5 — user selects area/window/screen.
/// -g captures audio from the default input (microphone).
#[tauri::command]
pub fn start_video_capture(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let output_path = make_output_path();
        launch_screencapture(&app, &state, &["-v", "-g", &output_path], &output_path)?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, state);
        Err("Video recording not yet implemented on this platform. See VIDEO_RECORDING.md".into())
    }
}

/// Start region video recording via screencapture -v -R x,y,w,h.
/// Coordinates come from the overlay (CSS pixels); screen_offset is added to convert to screen points.
/// A 300ms delay is added before starting so the overlay has time to close.
#[tauri::command]
pub fn start_video_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let (offset_x, offset_y) = {
            let offset = state.screen_offset.lock().unwrap();
            offset.unwrap_or((0, 0))
        };

        let screen_x = x + offset_x;
        let screen_y = y + offset_y;
        let rect_arg = format!("{},{},{},{}", screen_x, screen_y, width, height);
        let output_path = make_output_path();

        // Delay so the overlay window has time to close
        std::thread::sleep(std::time::Duration::from_millis(300));

        launch_screencapture(&app, &state, &["-v", "-g", "-R", &rect_arg, &output_path], &output_path)?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        // TODO: Linux — ffmpeg -video_size WxH -framerate 30 -f x11grab -i :0.0+X,Y output.mp4
        // TODO: Windows — ffmpeg -f gdigrab -offset_x X -offset_y Y -video_size WxH -i desktop output.mp4
        let _ = (app, state, x, y, width, height);
        Err("Region video recording not yet implemented on this platform. See VIDEO_RECORDING.md".into())
    }
}

/// Show a native window picker via osascript/JXA, return CGWindowID.
/// Returns None if user cancelled.
#[cfg(target_os = "macos")]
fn pick_window_id() -> Result<Option<u32>, String> {
    let script = r#"
ObjC.import("CoreGraphics");
var opts = 1 | 16;
var windowList = ObjC.castRefToObject($.CGWindowListCopyWindowInfo(opts, 0));
var choices = [];
var idMap = {};
for (var i = 0; i < windowList.count; i++) {
    var w = windowList.objectAtIndex(i);
    var layer = ObjC.unwrap(w.objectForKey("kCGWindowLayer"));
    if (layer === 0) {
        var owner = ObjC.unwrap(w.objectForKey("kCGWindowOwnerName")) || "?";
        var nameRef = w.objectForKey("kCGWindowName");
        var name = nameRef ? ObjC.unwrap(nameRef) : "";
        var wid = ObjC.unwrap(w.objectForKey("kCGWindowNumber"));
        if (name && name.length > 0) {
            var label = owner + ": " + name;
            choices.push(label);
            idMap[label] = wid;
        }
    }
}
if (choices.length === 0) { ""; }
else {
    var app = Application.currentApplication();
    app.includeStandardAdditions = true;
    var chosen = app.chooseFromList(choices, {
        withPrompt: "Выберите окно для записи:",
        defaultItems: [choices[0]]
    });
    if (chosen && chosen.length > 0) { "" + idMap[chosen[0]]; }
    else { ""; }
}
"#;

    let output = std::process::Command::new("osascript")
        .args(["-l", "JavaScript", "-e", script])
        .output()
        .map_err(|e| format!("Failed to run window picker: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(None); // user cancelled
    }
    let wid: u32 = stdout.parse()
        .map_err(|_| format!("Invalid window ID from picker: {}", stdout))?;
    Ok(Some(wid))
}

/// Start window video recording: shows native window picker, then records via screencapture -v -l <windowID>.
#[tauri::command]
pub fn start_video_capture_window(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let window_id = pick_window_id()?;
        let wid = match window_id {
            Some(id) => id,
            None => return Ok(()), // user cancelled
        };
        let wid_str = wid.to_string();
        let output_path = make_output_path();
        launch_screencapture(&app, &state, &["-v", "-g", "-l", &wid_str, &output_path], &output_path)?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        // TODO: Linux/Windows — window selection not yet implemented
        let _ = (app, state);
        Err("Window video recording not yet implemented on this platform. See VIDEO_RECORDING.md".into())
    }
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

    // Restore mic volume if it was muted during recording
    #[cfg(target_os = "macos")]
    restore_input_volume(&state);

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
    state.recording_pid.lock().unwrap().is_some()
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
    validate_recording_path(&path)?;
    std::fs::remove_file(&path).map_err(|e| format!("Failed to delete: {}", e))
}

/// Get info about a recorded video file
#[tauri::command]
pub fn get_video_info(path: String) -> Result<serde_json::Value, String> {
    validate_recording_path(&path)?;
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
    validate_recording_path(&src_path)?;
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
    validate_recording_path(&file_path)?;
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
    validate_recording_path(&src_path)?;
    std::fs::rename(&src_path, &dst_path)
        .or_else(|_| std::fs::copy(&src_path, &dst_path).map(|_| {
            let _ = std::fs::remove_file(&src_path);
        }))
        .map_err(|e| format!("Failed to move file: {}", e))
}

fn open_video_result_window(app: &AppHandle, _path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    crate::activate_as_regular(app);

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

/// Restore system input volume from saved value (called on stop or unmute).
#[cfg(target_os = "macos")]
fn restore_input_volume(state: &State<'_, AppState>) {
    let saved = {
        let mut v = state.saved_input_volume.lock().unwrap();
        v.take()
    };
    if let Some(vol) = saved {
        let _ = std::process::Command::new("osascript")
            .args(["-e", &format!("set volume input volume {}", vol)])
            .status();
    }
}

/// Toggle microphone mute: saves current input volume and sets to 0, or restores saved volume.
#[tauri::command]
pub fn toggle_mute_mic(state: State<'_, AppState>) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let is_muted = state.saved_input_volume.lock().unwrap().is_some();
        if is_muted {
            // Unmute: restore saved volume
            restore_input_volume(&state);
            return Ok(false); // now unmuted
        } else {
            // Mute: get current volume, save it, set to 0
            let output = std::process::Command::new("osascript")
                .args(["-e", "input volume of (get volume settings)"])
                .output()
                .map_err(|e| format!("Failed to get input volume: {}", e))?;
            let vol_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let vol: u32 = vol_str.parse().unwrap_or(75);
            {
                let mut saved = state.saved_input_volume.lock().unwrap();
                *saved = Some(vol);
            }
            let _ = std::process::Command::new("osascript")
                .args(["-e", "set volume input volume 0"])
                .status();
            return Ok(true); // now muted
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = state;
        Err("Mic mute not yet implemented on this platform".into())
    }
}

/// Check if mic is currently muted (i.e. we have a saved volume to restore).
#[tauri::command]
pub fn is_mic_muted(state: State<'_, AppState>) -> bool {
    state.saved_input_volume.lock().unwrap().is_some()
}

fn show_recording_window(app: &AppHandle) -> Result<(), String> {
    // Do NOT call activate_as_regular here — it steals focus from the native
    // screencapture picker, causing it to cancel and exit immediately.
    // The window is always_on_top so it's visible without app activation.
    // Dock activation happens via Focused(true) handler when user clicks the window.

    let win = WebviewWindowBuilder::new(
        app,
        "recording",
        WebviewUrl::App("index.html#/recording".into()),
    )
    .title("Запись")
    .inner_size(260.0, 64.0)
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
            let win_w = 260.0;
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

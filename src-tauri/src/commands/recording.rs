use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri::menu::MenuItem;
use crate::state::AppState;

/// Validate that a file path is a recording in /tmp (our own files).
/// Prevents path traversal attacks from frontend.
fn validate_recording_path(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    let filename = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let is_recording = filename.starts_with("recording_")
        && (filename.ends_with(".mov") || filename.ends_with(".mp4"));

    // Check parent directory is an allowed temp location
    let in_allowed_dir = if cfg!(target_os = "windows") {
        // On Windows, recordings go to %TEMP%
        // Canonicalize both paths to handle case differences and trailing slashes
        p.parent().map_or(false, |parent| {
            let tmp = std::env::temp_dir();
            // Compare canonicalized paths, fallback to string comparison
            match (parent.canonicalize(), tmp.canonicalize()) {
                (Ok(a), Ok(b)) => a.starts_with(&b) || a == b,
                _ => {
                    // Fallback: case-insensitive string compare
                    let pp = parent.to_str().unwrap_or("").to_lowercase();
                    let tp = tmp.to_str().unwrap_or("").to_lowercase();
                    pp.starts_with(&tp)
                }
            }
        })
    } else {
        p.parent() == Some(std::path::Path::new("/tmp"))
    };

    if !in_allowed_dir || !is_recording || path.contains("..") {
        return Err(format!("Access denied: path not allowed: {}", path));
    }
    Ok(())
}

/// Debug log helper — writes to /tmp/recording_debug.log
#[cfg(target_os = "macos")]
fn debug_log(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true).append(true)
        .open("/tmp/recording_debug.log")
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let _ = writeln!(f, "[{:.3}] {}", now.as_secs_f64(), msg);
    }
}

/// Shared helper: launch screencapture with given args, monitor process, show result on exit.
#[cfg(target_os = "macos")]
fn launch_screencapture(
    app: &AppHandle,
    state: &State<'_, AppState>,
    args: &[&str],
    output_path: &str,
) -> Result<(), String> {
    debug_log(&format!("launch_screencapture called, args: {:?}", args));

    // Check screen recording permission
    {
        extern "C" {
            fn CGRequestScreenCaptureAccess() -> bool;
        }
        let has_access = unsafe { CGRequestScreenCaptureAccess() };
        debug_log(&format!("CGRequestScreenCaptureAccess = {}", has_access));
        if !has_access {
            let _ = std::process::Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
                .spawn();
            return Err(crate::i18n::current(app).no_screen_permission.into());
        }
    }

    // Set activation policy to Regular so screencapture's child process has
    // proper display/WindowServer access (Accessory mode blocks it).
    // Use set_activation_policy directly — do NOT call activateIgnoringOtherApps_
    // because that steals focus from the native picker and causes it to cancel.
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
        debug_log("set activation policy to Regular (without stealing focus)");
    }

    let mut child = std::process::Command::new("screencapture")
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start screencapture: {}", e))?;

    let pid = child.id();
    debug_log(&format!("screencapture spawned, pid={}", pid));

    // Store PID so stop_video_recording can kill the process
    {
        let mut pid_guard = state.recording_pid.lock().unwrap();
        *pid_guard = Some(pid);
    }
    {
        let mut path = state.recording_path.lock().unwrap();
        *path = Some(output_path.to_string());
    }
    {
        let mut last = state.last_recording_path.lock().unwrap();
        *last = Some(output_path.to_string());
    }

    // Update tray menu to show "Stop recording" item
    set_tray_recording_mode(app, true);

    // Monitor process in background — show result window when screencapture exits
    let app_clone = app.clone();
    let recording_path = output_path.to_string();
    std::thread::spawn(move || {
        let start = std::time::Instant::now();

        let status = child.wait();
        let elapsed = start.elapsed();

        let status_str = match &status {
            Ok(s) => format!("exit={:?}", s),
            Err(e) => format!("wait error: {}", e),
        };
        debug_log(&format!(
            "screencapture exited: {}, elapsed={:.1}ms, file_exists={}",
            status_str,
            elapsed.as_secs_f64() * 1000.0,
            std::fs::metadata(&recording_path).is_ok()
        ));

        std::thread::sleep(std::time::Duration::from_millis(500));

        let state = app_clone.state::<AppState>();

        { let mut p = state.recording_pid.lock().unwrap(); *p = None; }
        { let mut p = state.recording_path.lock().unwrap(); *p = None; }

        // Restore mic volume if it was muted
        {
            let saved = { state.saved_input_volume.lock().unwrap().take() };
            if let Some(vol) = saved {
                let _ = std::process::Command::new("osascript")
                    .args(["-e", &format!("set volume input volume {}", vol)])
                    .status();
            }
        }

        // Restore normal tray menu
        set_tray_recording_mode(&app_clone, false);

        if std::fs::metadata(&recording_path).is_ok() {
            { let mut l = state.last_recording_path.lock().unwrap(); *l = Some(recording_path.clone()); }
            let _ = open_video_result_window(&app_clone, &recording_path);
        } else {
            debug_log("Recording cancelled — no file created");
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
        // Note: -g (audio capture) removed — it causes screencapture to exit immediately
        // when the parent process doesn't have explicit microphone TCC permission.
        // Audio can be added back once mic permission is properly configured.
        launch_screencapture(&app, &state, &["-v", &output_path], &output_path)?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let output_path = make_output_path_windows();
        start_windows_recording(&app, &state, None, &output_path)?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let _ = (app, state);
        Err("Video recording not yet implemented on Linux. See VIDEO_RECORDING.md".into())
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

        launch_screencapture(&app, &state, &["-v", "-R", &rect_arg, &output_path], &output_path)?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let output_path = make_output_path_windows();
        // Windows recording captures full monitor, region is noted but not cropped live
        // (similar to how macOS screencapture -v -R works)
        let _ = (x, y, width, height);
        start_windows_recording(&app, &state, None, &output_path)?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let _ = (app, state, x, y, width, height);
        Err("Region video recording not yet implemented on Linux. See VIDEO_RECORDING.md".into())
    }
}

/// Show a native window picker via osascript/JXA, return CGWindowID.
/// Returns None if user cancelled.
#[cfg(target_os = "macos")]
fn pick_window_id(prompt: &str) -> Result<Option<u32>, String> {
    let script_template = r#"
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
        withPrompt: "PROMPT_PLACEHOLDER",
        defaultItems: [choices[0]]
    });
    if (chosen && chosen.length > 0) { "" + idMap[chosen[0]]; }
    else { ""; }
}
"#;

    let script = script_template.replace("PROMPT_PLACEHOLDER", prompt);

    let output = std::process::Command::new("osascript")
        .args(["-l", "JavaScript", "-e", &script])
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
        let tr = crate::i18n::current(&app);
        let window_id = pick_window_id(tr.pick_window_prompt)?;
        let wid = match window_id {
            Some(id) => id,
            None => return Ok(()), // user cancelled
        };
        let wid_str = wid.to_string();
        let output_path = make_output_path();
        launch_screencapture(&app, &state, &["-v", "-l", &wid_str, &output_path], &output_path)?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let output_path = make_output_path_windows();
        start_windows_recording(&app, &state, None, &output_path)?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let _ = (app, state);
        Err("Window video recording not yet implemented on Linux. See VIDEO_RECORDING.md".into())
    }
}


/// Rebuild the tray menu in normal or "recording" mode.
/// In recording mode, shows "Stop recording" instead of the start-recording items.
pub fn set_tray_recording_mode(app: &AppHandle, recording: bool) {
    use tauri::menu::Menu;

    let hotkeys = crate::commands::hotkeys::get_hotkeys(app.clone());

    let tr = crate::i18n::current(app);

    let items: Vec<MenuItem<tauri::Wry>> = if recording {
        // Recording mode: only show stop + settings + quit
        vec![
            MenuItem::with_id(app, "stop_recording", format!("⏹ {}", tr.stop_recording), true, None::<&str>).unwrap(),
            MenuItem::with_id(app, "settings", tr.settings, true, None::<&str>).unwrap(),
            MenuItem::with_id(app, "quit", tr.quit, true, None::<&str>).unwrap(),
        ]
    } else {
        // Normal mode: all screenshot and video items
        vec![
            MenuItem::with_id(app, "screenshot", format!("{} ({})", tr.screenshot_region, hotkeys.region), true, None::<&str>).unwrap(),
            MenuItem::with_id(app, "screenshot_full", format!("{} ({})", tr.screenshot_fullscreen, hotkeys.fullscreen), true, None::<&str>).unwrap(),
            MenuItem::with_id(app, "screenshot_window", format!("{} ({})", tr.screenshot_window, hotkeys.window), true, None::<&str>).unwrap(),
            MenuItem::with_id(app, "video_screen", tr.video_screen, true, None::<&str>).unwrap(),
            MenuItem::with_id(app, "video_region", tr.video_region, true, None::<&str>).unwrap(),
            MenuItem::with_id(app, "video_window", tr.video_window, true, None::<&str>).unwrap(),
            MenuItem::with_id(app, "settings", tr.settings, true, None::<&str>).unwrap(),
            MenuItem::with_id(app, "quit", tr.quit, true, None::<&str>).unwrap(),
        ]
    };

    let refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = items.iter().map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>).collect();
    if let Ok(menu) = Menu::with_items(app, &refs) {
        if let Some(tray) = app.tray_by_id("main") {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

/// Stop recording — called from tray menu or from frontend command.
/// Sends SIGINT to screencapture, the background monitor thread handles the rest.
pub fn stop_recording_internal(app: &AppHandle) -> Result<String, String> {
    tracing::info!("stop_recording_internal called");
    let state = app.state::<AppState>();

    let path = {
        let p = state.recording_path.lock().unwrap();
        p.clone().ok_or_else(|| {
            tracing::warn!("stop_recording: no recording path in state");
            "No recording in progress".to_string()
        })?
    };
    tracing::info!(path = %path, "stopping recording");

    // Stop recording
    #[cfg(target_os = "windows")]
    {
        // Signal the Windows recording thread to stop
        state.recording_stop_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        tracing::info!("recording_stop_flag set to true");
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Send SIGINT to screencapture via stored PID
        let mut pid_guard = state.recording_pid.lock().unwrap();
        if let Some(pid) = pid_guard.take() {
            let _ = std::process::Command::new("kill")
                .args(["-2", &pid.to_string()])
                .status();
        }
    }

    // Save as last_recording_path
    {
        let mut last = state.last_recording_path.lock().unwrap();
        *last = Some(path.clone());
    }

    // Restore mic volume if it was muted during recording
    #[cfg(target_os = "macos")]
    restore_input_volume(&state);

    // Tray menu will be restored by the background monitor thread when screencapture exits

    Ok(path)
}

/// Stop recording, return path to the .mov file
#[tauri::command]
pub fn stop_video_recording(
    app: AppHandle,
    _state: State<'_, AppState>,
) -> Result<String, String> {
    stop_recording_internal(&app)
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

    #[cfg(target_os = "windows")]
    {
        // On Windows, recording is already MP4/H.264 — no conversion needed.
        // Trim is not yet supported (would require re-encoding).
        let _ = (preset, dst_path);
        if start_sec.filter(|&s| s > 0.0).is_some() || duration_sec.filter(|&d| d > 0.0).is_some() {
            return Err("Trimming is not yet supported on Windows. Save the file and trim externally.".into());
        }
        return Ok(src_path);
    }

    #[cfg(target_os = "linux")]
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

    let url = format!("{}/fast_upload?parent_id={}", crate::config::api_base_url(), parent_id);
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
        format!("{}{}", crate::config::api_base_url(), secure_url)
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
    .title(crate::i18n::current(app).recording_finished)
    .inner_size(560.0, 480.0)
    .min_inner_size(400.0, 300.0)
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

// ── Windows screen recording via windows-capture ──────────────────

#[cfg(target_os = "windows")]
fn make_output_path_windows() -> String {
    let tmp = std::env::temp_dir();
    let filename = format!("recording_{}.mp4", uuid::Uuid::new_v4());
    tmp.join(filename).to_string_lossy().to_string()
}

#[cfg(target_os = "windows")]
fn start_windows_recording(
    app: &AppHandle,
    state: &State<'_, AppState>,
    _monitor_index: Option<usize>,
    output_path: &str,
) -> Result<(), String> {
    use windows_capture::{
        capture::{Context, GraphicsCaptureApiHandler},
        encoder::{AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder},
        frame::Frame,
        graphics_capture_api::InternalCaptureControl,
        monitor::Monitor,
        settings::{ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
                   MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings},
    };

    tracing::info!(output = %output_path, "starting Windows screen recording");

    // Reset stop flag
    state.recording_stop_flag.store(false, std::sync::atomic::Ordering::SeqCst);
    let stop_flag = state.recording_stop_flag.clone();

    // Get primary monitor
    let monitor = Monitor::primary().map_err(|e| format!("Failed to get primary monitor: {}", e))?;

    // Store recording path
    {
        let mut p = state.recording_path.lock().unwrap();
        *p = Some(output_path.to_string());
    }
    {
        let mut l = state.last_recording_path.lock().unwrap();
        *l = Some(output_path.to_string());
    }

    // Update tray to show stop button
    set_tray_recording_mode(app, true);

    let output_path_owned = output_path.to_string();
    let app_clone = app.clone();

    struct CaptureHandler {
        encoder: Option<VideoEncoder>,
        stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl GraphicsCaptureApiHandler for CaptureHandler {
        type Flags = (std::sync::Arc<std::sync::atomic::AtomicBool>, String);
        type Error = Box<dyn std::error::Error + Send + Sync>;

        fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
            let (stop_flag, output_path) = ctx.flags;

            let monitor = Monitor::primary()?;
            let width = monitor.width()?;
            let height = monitor.height()?;

            let encoder = VideoEncoder::new(
                VideoSettingsBuilder::new(width, height),
                AudioSettingsBuilder::default().disabled(true),
                ContainerSettingsBuilder::default(),
                &output_path,
            )?;
            Ok(Self { encoder: Some(encoder), stop_flag })
        }

        fn on_frame_arrived(
            &mut self,
            frame: &mut Frame,
            capture_control: InternalCaptureControl,
        ) -> Result<(), Self::Error> {
            if self.stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
                tracing::info!("stop flag detected, finalizing encoder");
                if let Some(encoder) = self.encoder.take() {
                    encoder.finish()?;
                }
                capture_control.stop();
                tracing::info!("capture_control.stop() called");
                return Ok(());
            }
            if let Some(ref mut encoder) = self.encoder {
                encoder.send_frame(frame)?;
            }
            Ok(())
        }

        fn on_closed(&mut self) -> Result<(), Self::Error> {
            if let Some(encoder) = self.encoder.take() {
                encoder.finish()?;
            }
            Ok(())
        }
    }

    let settings = Settings::new(
        monitor,
        CursorCaptureSettings::Default,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        (stop_flag, output_path_owned.clone()),
    );

    std::thread::spawn(move || {
        tracing::info!("recording thread started, calling CaptureHandler::start");
        let result = CaptureHandler::start(settings);
        tracing::info!(ok = result.is_ok(), "CaptureHandler::start returned");

        if let Err(e) = &result {
            tracing::error!(error = %e, "Windows recording error");
        }

        let state = app_clone.state::<AppState>();
        { let mut p = state.recording_path.lock().unwrap(); *p = None; }

        set_tray_recording_mode(&app_clone, false);

        if std::fs::metadata(&output_path_owned).is_ok() {
            tracing::info!(path = %output_path_owned, "recording file exists, opening result window");
            { let mut l = state.last_recording_path.lock().unwrap(); *l = Some(output_path_owned.clone()); }
            let _ = open_video_result_window(&app_clone, &output_path_owned);
        } else {
            tracing::info!("Recording cancelled — no file created");
            let mut l = state.last_recording_path.lock().unwrap();
            *l = None;
        }
    });

    Ok(())
}


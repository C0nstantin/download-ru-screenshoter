//! ffmpeg-based recorder for a pipewire stream produced by the XDG ScreenCast portal.
//!
//! Spawns ffmpeg as a child process with `-f pipewire -i <node_id>`, returns the
//! child handle. Caller is responsible for waiting on the process or sending SIGINT
//! via `stop_ffmpeg_recording` to gracefully finalize the MP4 file.

use std::path::Path;
use std::process::{Child, Command, Stdio};

/// Spawn ffmpeg recording from a pipewire stream node.
///
/// ffmpeg must be installed on the host. On most Ubuntu/Fedora this is
/// `apt install ffmpeg` / `dnf install ffmpeg`.
///
/// Recording uses libx264 with the `ultrafast` preset and yuv420p pixel format
/// for maximum compatibility (works in browsers, Telegram, Quicktime).
pub fn start_ffmpeg_recording(node_id: u32, output_path: &Path) -> Result<Child, String> {
    let output_str = output_path
        .to_str()
        .ok_or_else(|| "non-utf8 output path".to_string())?;

    let child = Command::new("ffmpeg")
        .args([
            "-f",
            "pipewire",
            "-i",
            &node_id.to_string(),
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-y",
            output_str,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg spawn failed (is ffmpeg installed?): {e}"))?;

    tracing::info!(
        pid = child.id(),
        node_id,
        output = output_str,
        "ffmpeg recording started"
    );

    Ok(child)
}

/// Send SIGINT to ffmpeg so it can finalize the MP4 (write moov atom),
/// then wait for it to exit. Using SIGKILL would leave a corrupt file.
pub fn stop_ffmpeg_recording(mut child: Child) -> Result<(), String> {
    let pid = child.id();
    // Avoid pulling libc/nix crates — use the `kill` shell command.
    let _ = Command::new("kill")
        .args(["-INT", &pid.to_string()])
        .status();

    // Give ffmpeg time to finalize, then wait. `wait` blocks until exit.
    let status = child
        .wait()
        .map_err(|e| format!("ffmpeg wait failed: {e}"))?;

    tracing::info!(pid, exit_code = ?status.code(), "ffmpeg recording stopped");
    Ok(())
}

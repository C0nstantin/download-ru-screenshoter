//! Wayland-compatible screenshot capture via XDG Desktop Portal.
//!
//! Uses `ashpd::desktop::screenshot::Screenshot` to invoke the
//! `org.freedesktop.portal.Screenshot` interactive picker, reads the
//! returned URI as PNG bytes, and returns them.
//!
//! This is a Wayland-friendly alternative to the X11-only `screenshots` crate.
//! On X11 sessions the portal also works (it proxies to xprop/xrandr internally),
//! so callers can choose this path unconditionally on Linux.

use ashpd::desktop::screenshot::Screenshot;
use std::path::PathBuf;

/// Convert a `file://` URI string to a filesystem path.
fn uri_to_path(uri: &ashpd::Uri) -> Result<PathBuf, String> {
    let s = uri.as_str();
    let path_str = s
        .strip_prefix("file://")
        .ok_or_else(|| format!("portal returned non-file URI: {s}"))?;
    let decoded = percent_decode_str(path_str);
    Ok(PathBuf::from(decoded))
}

/// Simple percent-decoding for `file://` URI paths.
fn percent_decode_str(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next().unwrap_or(b'0') as char;
            let h2 = chars.next().unwrap_or(b'0') as char;
            if let Ok(byte) = u8::from_str_radix(&format!("{h1}{h2}"), 16) {
                result.push(byte as char);
            }
        } else {
            result.push(b as char);
        }
    }
    result
}

/// Capture the screen interactively via the XDG portal.
///
/// Shows the system picker (whole-screen / window / region), waits for the
/// user to confirm or cancel, then reads the resulting PNG file from disk
/// and returns its bytes.
///
/// Returns an error if the portal isn't available, the user cancels, or
/// reading the file fails.
pub async fn capture_via_portal() -> Result<Vec<u8>, String> {
    let response = Screenshot::request()
        .interactive(true)
        .modal(true)
        .send()
        .await
        .map_err(|e| format!("portal screenshot request failed: {e}"))?
        .response()
        .map_err(|e| format!("portal screenshot response failed: {e}"))?;

    let uri = response.uri();
    let path = uri_to_path(uri)?;

    tracing::info!(path = %path.display(), "portal screenshot saved, reading bytes");

    let bytes = std::fs::read(&path)
        .map_err(|e| format!("failed to read portal screenshot {}: {e}", path.display()))?;

    let _ = std::fs::remove_file(&path);

    Ok(bytes)
}

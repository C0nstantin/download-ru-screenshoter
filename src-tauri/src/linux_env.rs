//! Linux-specific environment detection utilities.

/// Checks via DBus whether a StatusNotifierWatcher is available on the session bus.
/// Returns `true` if `org.kde.StatusNotifierWatcher` is registered.
pub fn is_sni_watcher_available() -> bool {
    let result = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--dest=org.freedesktop.DBus",
            "--type=method_call",
            "--print-reply",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus.NameHasOwner",
            "string:org.kde.StatusNotifierWatcher",
        ])
        .output();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let available = stdout.contains("boolean true");
            tracing::info!(
                sni_watcher_available = available,
                "SNI watcher detection via dbus-send"
            );
            available
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "dbus-send not found or failed — assuming no SNI watcher"
            );
            false
        }
    }
}

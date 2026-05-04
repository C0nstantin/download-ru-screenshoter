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

/// Checks via DBus whether the GlobalShortcuts portal is available.
/// Returns `true` if `org.freedesktop.portal.Desktop` exports the
/// `org.freedesktop.portal.GlobalShortcuts` interface.
pub fn is_global_shortcuts_portal_available() -> bool {
    let result = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--dest=org.freedesktop.portal.Desktop",
            "--type=method_call",
            "--print-reply",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.DBus.Introspectable.Introspect",
        ])
        .output();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let available = stdout.contains("org.freedesktop.portal.GlobalShortcuts");
            tracing::info!(available, "GlobalShortcuts portal detection");
            available
        }
        Err(e) => {
            tracing::warn!(error = %e, "dbus-send failed for GlobalShortcuts portal check");
            false
        }
    }
}

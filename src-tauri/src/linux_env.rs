//! Linux-specific environment detection utilities.

/// Detects whether the user runs a dark color scheme via gsettings.
/// Returns `true` for `prefer-dark`, `false` otherwise.
pub fn is_dark_theme() -> bool {
    std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("prefer-dark"))
        .unwrap_or(false)
}

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

/// Checks via DBus whether a portal interface is available.
/// Introspects `org.freedesktop.portal.Desktop` and looks for `interface` in the XML output.
fn is_portal_interface_available(interface: &str) -> bool {
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
            let available = stdout.contains(interface);
            tracing::info!(interface, available, "portal interface check");
            available
        }
        Err(e) => {
            tracing::warn!(interface, error = %e, "dbus-send failed for portal check");
            false
        }
    }
}

/// Checks whether the Screenshot portal is available.
pub fn is_screenshot_portal_available() -> bool {
    is_portal_interface_available("org.freedesktop.portal.Screenshot")
}

/// Checks whether the ScreenCast portal is available.
pub fn is_screencast_portal_available() -> bool {
    is_portal_interface_available("org.freedesktop.portal.ScreenCast")
}

/// Checks whether the GlobalShortcuts portal is available.
pub fn is_global_shortcuts_portal_available() -> bool {
    is_portal_interface_available("org.freedesktop.portal.GlobalShortcuts")
}

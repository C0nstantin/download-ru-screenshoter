//! Linux-specific environment detection utilities.

/// Returns `XDG_CURRENT_DESKTOP` lowercased, or empty string.
fn xdg_desktop() -> String {
    std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase()
}

/// True for GNOME-family sessions (GNOME, Unity, Cinnamon, MATE, Pantheon).
pub fn is_gnome_like() -> bool {
    let de = xdg_desktop();
    de.contains("gnome")
        || de.contains("unity")
        || de.contains("cinnamon")
        || de.contains("mate")
        || de.contains("pantheon")
}

/// True for KDE Plasma sessions.
pub fn is_kde_like() -> bool {
    let de = xdg_desktop();
    de.contains("kde") || de.contains("plasma")
}

/// True if the current session is Wayland (XDG_SESSION_TYPE=wayland).
pub fn is_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .map(|s| s.to_lowercase().contains("wayland"))
        .unwrap_or(false)
}

/// True if the current session is X11 (or unknown — we assume X11 as the default).
pub fn is_x11_session() -> bool {
    !is_wayland_session()
}

/// Returns the running Plasma shell version (e.g. "plasmashell 5.27.0"), or None.
pub fn plasma_version() -> Option<String> {
    std::process::Command::new("plasmashell")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Detects whether the user runs a dark color scheme. Branches by DE:
/// - GNOME-family: `gsettings get org.gnome.desktop.interface color-scheme` → `prefer-dark`.
/// - KDE: parses `~/.config/kdeglobals` for `ColorScheme=...Dark`.
/// Returns false on unknown environments.
pub fn is_dark_theme() -> bool {
    if is_kde_like() {
        return is_kde_dark_theme();
    }
    is_gnome_dark_theme()
}

fn is_gnome_dark_theme() -> bool {
    std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("prefer-dark"))
        .unwrap_or(false)
}

fn is_kde_dark_theme() -> bool {
    let Some(home) = std::env::var_os("HOME") else { return false };
    let path = std::path::Path::new(&home).join(".config/kdeglobals");
    let Ok(content) = std::fs::read_to_string(path) else { return false };
    for line in content.lines() {
        if let Some(value) = line.trim().strip_prefix("ColorScheme=") {
            return value.to_lowercase().contains("dark");
        }
    }
    false
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

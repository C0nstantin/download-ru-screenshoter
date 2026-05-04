use ashpd::desktop::global_shortcuts::{
    BindShortcutsOptions, GlobalShortcuts, NewShortcut,
};

pub struct GlobalShortcutsSession {
    session: ashpd::desktop::Session<GlobalShortcuts>,
}

impl GlobalShortcutsSession {
    pub async fn start(handle: tauri::AppHandle) -> Result<Self, String> {
        let proxy = GlobalShortcuts::new()
            .await
            .map_err(|e| format!("GlobalShortcuts::new failed: {e}"))?;

        let session = proxy
            .create_session(Default::default())
            .await
            .map_err(|e| format!("create_session failed: {e}"))?;

        let shortcuts = vec![
            NewShortcut::new("region", "Screenshot region"),
            NewShortcut::new("fullscreen", "Screenshot fullscreen"),
            NewShortcut::new("window", "Screenshot window"),
        ];

        proxy
            .bind_shortcuts(&session, &shortcuts, None, BindShortcutsOptions::default())
            .await
            .map_err(|e| format!("bind_shortcuts failed: {e}"))?
            .response()
            .map_err(|e| format!("bind_shortcuts response failed: {e}"))?;

        tracing::info!("GlobalShortcuts session started, shortcuts registered");

        tauri::async_runtime::spawn(async move {
            if let Err(e) = listen_for_activations(proxy, handle).await {
                tracing::error!(error = %e, "global shortcut listener exited");
            }
        });

        Ok(Self { session })
    }

    pub async fn close(&self) -> Result<(), String> {
        self.session
            .close()
            .await
            .map_err(|e| format!("session close failed: {e}"))
    }
}

async fn listen_for_activations(
    proxy: GlobalShortcuts,
    handle: tauri::AppHandle,
) -> Result<(), ashpd::Error> {
    use futures_util::StreamExt;
    let mut activated = proxy.receive_activated().await?;

    while let Some(event) = activated.next().await {
        let shortcut_id = event.shortcut_id().to_string();
        tracing::info!(shortcut_id = %shortcut_id, "global shortcut activated");

        match shortcut_id.as_str() {
            "region" => {
                if let Err(e) = crate::commands::screenshot::start_region_capture(handle.clone()) {
                    tracing::error!(error = %e, "region capture failed");
                }
            }
            "fullscreen" => {
                let app = handle.clone();
                std::thread::spawn(move || {
                    if let Err(e) =
                        crate::commands::screenshot::capture_fullscreen_and_edit_internal(app, None)
                    {
                        tracing::error!(error = %e, "fullscreen capture failed");
                    }
                });
            }
            "window" => {
                let app = handle.clone();
                std::thread::spawn(move || {
                    if let Err(e) = crate::commands::screenshot::capture_window_and_edit(app) {
                        tracing::error!(error = %e, "window capture failed");
                    }
                });
            }
            _ => {
                tracing::warn!(shortcut_id, "unknown global shortcut activated");
            }
        }
    }

    Ok(())
}

use ashpd::desktop::screencast::{
    CursorMode, OpenPipeWireRemoteOptions, Screencast, SelectSourcesOptions, SourceType,
    StartCastOptions,
};
use ashpd::desktop::PersistMode;
use std::os::fd::OwnedFd;

pub struct ScreencastSession {
    proxy: Screencast,
    session: ashpd::desktop::Session<Screencast>,
    pub pipewire_fd: OwnedFd,
    pub stream_node_id: u32,
}

impl ScreencastSession {
    pub async fn start() -> Result<Self, String> {
        let proxy = Screencast::new()
            .await
            .map_err(|e| format!("Screencast::new failed: {e}"))?;

        let session = proxy
            .create_session(Default::default())
            .await
            .map_err(|e| format!("create_session failed: {e}"))?;

        proxy
            .select_sources(
                &session,
                SelectSourcesOptions::default()
                    .set_cursor_mode(CursorMode::Embedded)
                    .set_sources(SourceType::Monitor | SourceType::Window)
                    .set_multiple(false)
                    .set_persist_mode(PersistMode::DoNot),
            )
            .await
            .map_err(|e| format!("select_sources failed: {e}"))?;

        let response = proxy
            .start(&session, None, StartCastOptions::default())
            .await
            .map_err(|e| format!("start failed: {e}"))?
            .response()
            .map_err(|e| format!("start response failed: {e}"))?;

        let stream = response
            .streams()
            .first()
            .ok_or_else(|| "no streams returned".to_string())?;
        let stream_node_id = stream.pipe_wire_node_id();

        let pipewire_fd = proxy
            .open_pipe_wire_remote(&session, OpenPipeWireRemoteOptions::default())
            .await
            .map_err(|e| format!("open_pipe_wire_remote failed: {e}"))?;

        tracing::info!(stream_node_id, "ScreenCast session started");

        Ok(Self {
            proxy,
            session,
            pipewire_fd,
            stream_node_id,
        })
    }

    pub async fn close(&self) -> Result<(), String> {
        self.session
            .close()
            .await
            .map_err(|e| format!("session close failed: {e}"))
    }
}

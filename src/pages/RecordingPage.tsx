import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "../i18n/useTranslation";

function RecordingPage() {
  const [stopping, setStopping] = useState(false);
  const [muted, setMuted] = useState(false);
  const { t } = useTranslation();

  useEffect(() => {
    invoke<boolean>("is_mic_muted").then(setMuted).catch(() => {});
  }, []);

  const handleStop = async () => {
    setStopping(true);
    try {
      await invoke("stop_video_recording");
      // VideoResultPage opens automatically after stop
    } catch (err) {
      console.error("Stop recording error:", err);
      setStopping(false);
    }
  };

  const handleToggleMute = async () => {
    try {
      const nowMuted = await invoke<boolean>("toggle_mute_mic");
      setMuted(nowMuted);
    } catch (err) {
      console.error("Toggle mute error:", err);
    }
  };

  return (
    <div style={{
      display: "flex",
      alignItems: "center",
      gap: 8,
      padding: "0 12px",
      height: "100%",
      background: "#1a1a1a",
      borderRadius: 10,
      border: "1px solid #333",
      userSelect: "none",
    }}>
      <div style={{
        width: 12,
        height: 12,
        borderRadius: "50%",
        background: "#ff3b30",
        animation: "pulse 1.2s ease-in-out infinite",
        flexShrink: 0,
      }} />
      <span style={{ color: "#fff", fontSize: 13, fontWeight: 500, flex: 1 }}>
        {stopping ? t("recording.stopping") : t("recording.recording")}
      </span>
      <button
        onClick={handleToggleMute}
        disabled={stopping}
        title={muted ? t("recording.enableMic") : t("recording.disableMic")}
        style={{
          background: muted ? "#7f1d1d" : "#2a2a3e",
          color: "#fff",
          border: "1px solid #444",
          borderRadius: 6,
          padding: "4px 8px",
          fontSize: 14,
          cursor: stopping ? "default" : "pointer",
          opacity: stopping ? 0.6 : 1,
          lineHeight: 1,
        }}
      >
        {muted ? "🔇" : "🎙"}
      </button>
      <button
        onClick={handleStop}
        disabled={stopping}
        style={{
          background: "#ff3b30",
          color: "#fff",
          border: "none",
          borderRadius: 6,
          padding: "4px 12px",
          fontSize: 13,
          fontWeight: 600,
          cursor: stopping ? "default" : "pointer",
          opacity: stopping ? 0.6 : 1,
        }}
      >
        {t("recording.stop")}
      </button>
      <style>{`
        @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        html, body, #root { height: 100%; }
      `}</style>
    </div>
  );
}

export default RecordingPage;

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";

function RecordingPage() {
  const [stopping, setStopping] = useState(false);

  const handleStop = async () => {
    setStopping(true);
    try {
      const filePath = await invoke<string>("stop_video_recording");
      // Ask where to save
      const savePath = await save({
        defaultPath: `recording_${Date.now()}.mov`,
        filters: [{ name: "Video", extensions: ["mov"] }],
      });
      if (savePath && filePath) {
        await invoke("move_recording", { srcPath: filePath, dstPath: savePath });
      }
    } catch (err) {
      console.error("Stop recording error:", err);
      // Still close the window
      const win = getCurrentWindow();
      await win.close();
    }
  };

  return (
    <div style={{
      display: "flex",
      alignItems: "center",
      gap: 12,
      padding: "0 16px",
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
        {stopping ? "Сохранение..." : "Запись..."}
      </span>
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
        Стоп
      </button>
      <style>{`
        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.3; }
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        html, body, #root { height: 100%; }
      `}</style>
    </div>
  );
}

export default RecordingPage;

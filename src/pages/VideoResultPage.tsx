import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { sendNotification, isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface VideoInfo {
  path: string;
  size: number;
  name: string;
}

function fmt(bytes: number) {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function fmtTime(s: number) {
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${sec.toString().padStart(2, "0")}`;
}

function VideoResultPage() {
  const [info, setInfo] = useState<VideoInfo | null>(null);
  const [videoSrc, setVideoSrc] = useState("");
  const [duration, setDuration] = useState(0);
  const [trimStart, setTrimStart] = useState(0);
  const [trimEnd, setTrimEnd] = useState(0);
  const [preset, setPreset] = useState("high");
  const [isConverting, setIsConverting] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const [uploadUrl, setUploadUrl] = useState("");
  const [error, setError] = useState("");
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    invoke<string>("get_last_recording_path").then((path) => {
      invoke<VideoInfo>("get_video_info", { path }).then((info) => {
        setInfo(info);
        // Use custom localfile:// protocol that supports byte-range (needed for seek)
        setVideoSrc(`localfile://localhost${info.path}`);
      }).catch((e) => setError(String(e)));
    }).catch((e) => setError(String(e)));
  }, []);

  const handleLoaded = () => {
    if (videoRef.current) {
      const d = videoRef.current.duration;
      setDuration(d);
      setTrimEnd(d);
    }
  };

  const handleConvert = async () => {
    if (!info) return;
    setIsConverting(true);
    setError("");
    try {
      const startSec = trimStart > 0 ? trimStart : undefined;
      const durSec = (trimEnd < duration && trimEnd > 0) ? (trimEnd - (trimStart || 0)) : undefined;
      const newPath = await invoke<string>("convert_to_mp4", {
        srcPath: info.path,
        preset,
        startSec,
        durationSec: durSec,
      });
      const newInfo = await invoke<VideoInfo>("get_video_info", { path: newPath });
      setInfo(newInfo);
      setVideoSrc(`localfile://localhost${newPath}`);
      setUploadUrl(""); // allow re-upload after conversion
      setTrimStart(0);
      setTrimEnd(0);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsConverting(false);
    }
  };

  const doUpload = async () => {
    if (!info) return;
    const url = await invoke<string>("upload_video_to_download", { filePath: info.path });
    setUploadUrl(url);
    await writeText(url);
    let ok = await isPermissionGranted();
    if (!ok) { const p = await requestPermission(); ok = p === "granted"; }
    if (ok) sendNotification({ title: "Видео загружено", body: `Ссылка скопирована` });
  };

  const handleUpload = async () => {
    if (!info) return;
    setIsUploading(true);
    setError("");
    try {
      await doUpload();
    } catch (e) {
      const msg = String(e);
      if (msg.includes("401") || msg.includes("unauthorized") || msg.includes("No access token")) {
        // Token expired — open OAuth, retry after auth
        setError("");
        try {
          await invoke("open_oauth_browser");
          const unlisten = await listen<boolean>("oauth-complete", async (event) => {
            unlisten();
            if (event.payload) {
              setIsUploading(true);
              try { await doUpload(); } catch (e2) { setError(String(e2)); }
              finally { setIsUploading(false); }
            } else {
              setIsUploading(false);
              setError("Авторизация не удалась. Попробуйте ещё раз.");
            }
          });
          return;
        } catch (authErr) {
          setError(`Ошибка авторизации: ${authErr}`);
        }
      } else {
        setError(msg);
      }
    } finally {
      setIsUploading(false);
    }
  };

  const handleSave = async () => {
    if (!info) return;
    const ext = info.name.includes(".mp4") ? "mp4" : "mov";
    const dst = await save({
      defaultPath: info.name,
      filters: [{ name: "Video", extensions: [ext, "mp4", "mov"] }],
    });
    if (dst) {
      await invoke("move_recording", { srcPath: info.path, dstPath: dst });
      const win = getCurrentWindow();
      await win.close();
    }
  };

  const handleDelete = async () => {
    if (info) { try { await invoke("delete_recording", { path: info.path }); } catch (_) {} }
    const win = getCurrentWindow();
    await win.close();
  };

  const hasTrim = duration > 0 && (trimStart > 0 || trimEnd < duration);

  return (
    <div style={{
      display: "flex", flexDirection: "column", height: "100vh",
      background: "#1a1a2e", color: "#eee",
      fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
      padding: 12, gap: 10,
    }}>
      {/* Video */}
      <div style={{ flex: 1, background: "#000", borderRadius: 8, overflow: "hidden", minHeight: 0, position: "relative" }}>
        {videoSrc ? (
          <video
            ref={videoRef}
            src={videoSrc}
            controls
            onLoadedMetadata={handleLoaded}
            style={{ width: "100%", height: "100%", objectFit: "contain" }}
          />
        ) : (
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "#555" }}>
            Загрузка...
          </div>
        )}
      </div>

      {/* File info */}
      {info && (
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "#666" }}>
          <span>{info.name}</span>
          <span>{fmt(info.size)}{duration > 0 ? ` · ${fmtTime(duration)}` : ""}</span>
        </div>
      )}

      {/* Trim controls */}
      {duration > 0 && (
        <div style={{ background: "#12122a", borderRadius: 6, padding: "8px 10px", display: "flex", flexDirection: "column", gap: 6 }}>
          <div style={{ fontSize: 11, color: "#888", marginBottom: 2 }}>Обрезка</div>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span style={{ fontSize: 11, color: "#888", width: 36 }}>Начало</span>
            <input
              type="range" min={0} max={trimEnd - 0.1} step={0.1}
              value={trimStart}
              onChange={(e) => {
                const v = +e.target.value;
                setTrimStart(v);
                if (videoRef.current) videoRef.current.currentTime = v;
              }}
              style={{ flex: 1 }}
            />
            <span style={{ fontSize: 11, color: "#aaa", width: 36, textAlign: "right" }}>{fmtTime(trimStart)}</span>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span style={{ fontSize: 11, color: "#888", width: 36 }}>Конец</span>
            <input
              type="range" min={trimStart + 0.1} max={duration} step={0.1}
              value={trimEnd}
              onChange={(e) => {
                const v = +e.target.value;
                setTrimEnd(v);
                if (videoRef.current) videoRef.current.currentTime = v;
              }}
              style={{ flex: 1 }}
            />
            <span style={{ fontSize: 11, color: "#aaa", width: 36, textAlign: "right" }}>{fmtTime(trimEnd)}</span>
          </div>
          {hasTrim && (
            <div style={{ fontSize: 11, color: "#60a5fa" }}>
              Будет вырезано: {fmtTime(trimStart)} — {fmtTime(trimEnd)} ({fmtTime(trimEnd - trimStart)})
            </div>
          )}
        </div>
      )}

      {/* Upload result */}
      {uploadUrl && (
        <div style={{
          background: "#1a4d2e", borderRadius: 6, padding: "6px 10px",
          display: "flex", gap: 8, alignItems: "center",
        }}>
          <span style={{ color: "#4ade80", fontSize: 11, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {uploadUrl}
          </span>
          <button onClick={() => writeText(uploadUrl)} style={btnSm}>Копировать</button>
        </div>
      )}

      {/* Error */}
      {error && (
        <div style={{ background: "#4d1a1a", borderRadius: 6, padding: "6px 10px", color: "#ff6666", fontSize: 11 }}>
          {error}
        </div>
      )}

      {/* Actions */}
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
        {/* Quality selector */}
        <select
          value={preset}
          onChange={(e) => setPreset(e.target.value)}
          style={{
            background: "#12122a", color: "#eee", border: "1px solid #444",
            borderRadius: 6, padding: "5px 8px", fontSize: 12, cursor: "pointer",
          }}
        >
          <option value="high">MP4 Высокое</option>
          <option value="medium">MP4 Среднее (1080p)</option>
          <option value="low">MP4 Низкое (720p)</option>
        </select>

        <button onClick={handleConvert} disabled={isConverting} style={btn}>
          {isConverting ? "Конвертация..." : hasTrim ? "Обрезать и конвертировать" : "Конвертировать в MP4"}
        </button>

        <div style={{ flex: 1 }} />

        <button
          onClick={handleUpload}
          disabled={isUploading || !!uploadUrl}
          style={{ ...btn, background: "#2563eb", fontWeight: 600 }}
        >
          {isUploading ? "Загрузка..." : uploadUrl ? "Загружено ✓" : "Загрузить"}
        </button>
        <button onClick={handleSave} style={btn}>Сохранить...</button>
        <button onClick={handleDelete} style={{ ...btn, background: "#7f1d1d" }}>✕</button>
      </div>
    </div>
  );
}

const btn: React.CSSProperties = {
  background: "#2a2a3e", color: "#eee", border: "1px solid #444",
  borderRadius: 6, padding: "5px 12px", fontSize: 12, cursor: "pointer",
};
const btnSm: React.CSSProperties = {
  ...btn, padding: "3px 8px", fontSize: 11,
};

export default VideoResultPage;

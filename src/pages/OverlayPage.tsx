import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTranslation } from "../i18n/useTranslation";

interface ScreenshotData {
  base64: string;
  width: number;
  height: number;
}

interface SelectionRect {
  startX: number;
  startY: number;
  endX: number;
  endY: number;
}

interface WindowInfo {
  hwnd: number;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

function OverlayPage({ mode = "screenshot" }: { mode?: "screenshot" | "video" | "window" }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [screenshot, setScreenshot] = useState<ScreenshotData | null>(null);
  const [isSelecting, setIsSelecting] = useState(false);
  const [selection, setSelection] = useState<SelectionRect | null>(null);
  const [imageLoaded, setImageLoaded] = useState(false);
  const [windowList, setWindowList] = useState<WindowInfo[]>([]);
  const [hoveredWindow, setHoveredWindow] = useState<WindowInfo | null>(null);
  const { t } = useTranslation();

  // Load screenshot on mount
  useEffect(() => {
    console.log("OverlayPage mounted, loading screenshot...");
    invoke<ScreenshotData>("get_current_screenshot")
      .then((data) => {
        console.log("Got screenshot:", data.width, "x", data.height);
        setScreenshot(data);
      })
      .catch((err) => {
        console.error("Failed to get screenshot:", err);
        closeOverlay();
      });

    // In window mode, load window list
    if (mode === "window") {
      invoke<WindowInfo[]>("get_window_list")
        .then((list) => {
          console.log("Got window list:", list.length, "windows");
          setWindowList(list);
        })
        .catch((err) => console.error("Failed to get window list:", err));
    }
  }, []);

  // Draw the screenshot and selection/window highlight
  useEffect(() => {
    if (!screenshot || !canvasRef.current) return;

    const canvas = canvasRef.current;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const img = new Image();
    img.onload = () => {
      // Set canvas size to match CSS size exactly (no DPR scaling)
      const rect = canvas.getBoundingClientRect();
      canvas.width = rect.width;
      canvas.height = rect.height;

      const scaleX = canvas.width / screenshot.width;
      const scaleY = canvas.height / screenshot.height;

      // Draw full image
      ctx.drawImage(img, 0, 0, canvas.width, canvas.height);

      // Draw dark overlay on top
      ctx.fillStyle = "rgba(0, 0, 0, 0.4)";
      ctx.fillRect(0, 0, canvas.width, canvas.height);

      if (mode === "window" && hoveredWindow) {
        // Highlight hovered window
        const wx = hoveredWindow.x * scaleX;
        const wy = hoveredWindow.y * scaleY;
        const ww = hoveredWindow.width * scaleX;
        const wh = hoveredWindow.height * scaleY;

        ctx.save();
        ctx.beginPath();
        ctx.rect(wx, wy, ww, wh);
        ctx.clip();
        ctx.clearRect(wx, wy, ww, wh);
        ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
        ctx.restore();

        ctx.strokeStyle = "#00aaff";
        ctx.lineWidth = 3;
        ctx.strokeRect(wx, wy, ww, wh);

        // Draw window title
        const title = hoveredWindow.title.length > 50
          ? hoveredWindow.title.substring(0, 50) + "..."
          : hoveredWindow.title;
        ctx.fillStyle = "rgba(0, 170, 255, 0.9)";
        const textWidth = ctx.measureText(title).width + 16;
        ctx.fillRect(wx, wy - 28, textWidth, 24);
        ctx.fillStyle = "#fff";
        ctx.font = "13px -apple-system, sans-serif";
        ctx.fillText(title, wx + 8, wy - 10);
      } else if (selection) {
        // Region/video selection highlight
        const x = Math.min(selection.startX, selection.endX);
        const y = Math.min(selection.startY, selection.endY);
        const w = Math.abs(selection.endX - selection.startX);
        const h = Math.abs(selection.endY - selection.startY);

        // Clear selection area and draw bright image
        ctx.save();
        ctx.beginPath();
        ctx.rect(x, y, w, h);
        ctx.clip();
        ctx.clearRect(x, y, w, h);
        ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
        ctx.restore();

        // Draw selection border
        ctx.strokeStyle = "#00aaff";
        ctx.lineWidth = 2;
        ctx.strokeRect(x, y, w, h);

        // Draw dimensions
        if (w > 50 && h > 20) {
          ctx.fillStyle = "rgba(0, 170, 255, 0.9)";
          ctx.fillRect(x, y - 25, 90, 22);
          ctx.fillStyle = "#fff";
          ctx.font = "13px -apple-system, sans-serif";
          ctx.fillText(`${Math.round(w)} × ${Math.round(h)}`, x + 8, y - 8);
        }
      }

      setImageLoaded(true);
    };
    img.src = `data:image/png;base64,${screenshot.base64}`;
  }, [screenshot, selection, hoveredWindow]);

  const closeOverlay = async () => {
    const win = getCurrentWindow();
    await win.close();
  };

  // Get mouse coordinates relative to canvas
  const getCanvasCoords = (e: React.MouseEvent) => {
    const canvas = canvasRef.current;
    if (!canvas) return { x: 0, y: 0 };

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    return { x, y };
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    if (mode === "window") {
      // In window mode, click captures the hovered window
      if (e.button === 2) {
        e.preventDefault();
        closeOverlay();
        return;
      }
      if (hoveredWindow) {
        captureWindow(hoveredWindow);
      }
      return;
    }

    // Right click to reset selection
    if (e.button === 2) {
      e.preventDefault();
      setSelection(null);
      setIsSelecting(false);
      return;
    }

    const { x, y } = getCanvasCoords(e);
    setIsSelecting(true);
    setSelection({
      startX: x,
      startY: y,
      endX: x,
      endY: y,
    });
  };

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    // Reset selection on right click
    setSelection(null);
    setIsSelecting(false);
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (mode === "window") {
      // Find which window the cursor is over
      if (!screenshot || !canvasRef.current) return;
      const { x, y } = getCanvasCoords(e);
      const canvas = canvasRef.current;
      const rect = canvas.getBoundingClientRect();
      const scaleX = screenshot.width / rect.width;
      const scaleY = screenshot.height / rect.height;
      const screenX = x * scaleX;
      const screenY = y * scaleY;

      // Find smallest window containing the point (most specific)
      let best: WindowInfo | null = null;
      let bestArea = Infinity;
      for (const w of windowList) {
        if (screenX >= w.x && screenX <= w.x + w.width &&
            screenY >= w.y && screenY <= w.y + w.height) {
          const area = w.width * w.height;
          if (area < bestArea) {
            best = w;
            bestArea = area;
          }
        }
      }
      setHoveredWindow(best);
      return;
    }

    if (!isSelecting || !selection) return;
    const { x, y } = getCanvasCoords(e);
    setSelection({
      ...selection,
      endX: x,
      endY: y,
    });
  };

  const handleMouseUp = () => {
    if (!isSelecting || !selection) return;
    setIsSelecting(false);

    const width = Math.abs(selection.endX - selection.startX);
    const height = Math.abs(selection.endY - selection.startY);

    // Minimum selection size
    if (width < 10 || height < 10) {
      setSelection(null);
      return;
    }

    // Selection is complete, user can now:
    // - Press Enter to confirm
    // - Right-click or R to reset
    // - ESC to cancel
  };

  const captureWindow = async (win: WindowInfo) => {
    try {
      closeOverlay();
      await invoke("capture_window_by_hwnd", { hwnd: win.hwnd });
    } catch (err) {
      console.error("Failed to capture window:", err);
      closeOverlay();
    }
  };

  const confirmSelection = useCallback(async () => {
    console.log("confirmSelection called", { selection, screenshot: !!screenshot, canvas: !!canvasRef.current });
    if (!selection || !screenshot || !canvasRef.current) {
      console.log("Early return - missing data");
      return;
    }

    const x = Math.min(selection.startX, selection.endX);
    const y = Math.min(selection.startY, selection.endY);
    const width = Math.abs(selection.endX - selection.startX);
    const height = Math.abs(selection.endY - selection.startY);

    const canvas = canvasRef.current;
    const rect = canvas.getBoundingClientRect();
    // Selection is in CSS pixels, screenshot is in physical pixels
    const scaleX = screenshot.width / rect.width;
    const scaleY = screenshot.height / rect.height;

    const imgX = Math.max(0, Math.round(x * scaleX));
    const imgY = Math.max(0, Math.round(y * scaleY));
    const imgWidth = Math.min(screenshot.width - imgX, Math.round(width * scaleX));
    const imgHeight = Math.min(screenshot.height - imgY, Math.round(height * scaleY));

    try {
      if (mode === "video") {
        // For video: pass CSS pixel coords (logical screen coords) to screencapture
        const vidX = Math.round(Math.min(selection.startX, selection.endX));
        const vidY = Math.round(Math.min(selection.startY, selection.endY));
        const vidW = Math.round(Math.abs(selection.endX - selection.startX));
        const vidH = Math.round(Math.abs(selection.endY - selection.startY));
        closeOverlay();
        await invoke("start_video_recording", { x: vidX, y: vidY, width: vidW, height: vidH });
      } else {
        // Screenshot mode
        const cropResult = await invoke("crop_image", {
          region: { x: imgX, y: imgY, width: imgWidth, height: imgHeight },
        });
        console.log("Crop done, result:", cropResult);
        await invoke("open_editor");
        closeOverlay();
      }
    } catch (err) {
      console.error("Failed:", err);
      closeOverlay();
    }
  }, [selection, screenshot]);

  // Double click to confirm selection
  const handleDoubleClick = () => {
    if (selection && !isSelecting) {
      confirmSelection();
    }
  };

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.key === "Escape") {
      if (mode === "window") {
        closeOverlay();
        return;
      }
      if (selection && !isSelecting) {
        setSelection(null);
      } else {
        closeOverlay();
      }
    } else if (e.key === "Enter" || e.key === " ") {
      // Enter or Space to confirm selection
      if (selection && !isSelecting) {
        confirmSelection();
      }
    } else if (e.key === "r" || e.key === "R" || e.key === "к" || e.key === "К") {
      // R to reset selection (also Russian К)
      setSelection(null);
      setIsSelecting(false);
    }
  }, [selection, isSelecting, confirmSelection]);

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  // Focus the overlay div on mount for keyboard events
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Focus the overlay to receive keyboard events
    overlayRef.current?.focus();

    // Also ensure window has focus (important for Windows)
    const ensureFocus = async () => {
      try {
        const win = getCurrentWindow();
        await win.setFocus();
      } catch (e) {
        console.error("Failed to set window focus:", e);
      }
    };
    ensureFocus();
  }, []);

  const getHint = () => {
    if (mode === "window") {
      return hoveredWindow
        ? `${hoveredWindow.title} — click to capture`
        : "Hover over a window and click to capture. ESC to cancel.";
    }
    if (selection && !isSelecting) {
      return mode === "video"
        ? t("overlay.videoConfirmHint")
        : t("overlay.screenshotConfirmHint");
    }
    return mode === "video"
      ? t("overlay.videoSelectHint")
      : t("overlay.screenshotSelectHint");
  };

  return (
    <div
      className="overlay-page"
      ref={overlayRef}
      tabIndex={0}
      onKeyDown={(e) => handleKeyDown(e.nativeEvent)}
    >
      <canvas
        ref={canvasRef}
        className="overlay-canvas"
        style={(selection && !isSelecting) ? { pointerEvents: 'none' } : undefined}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        onDoubleClick={handleDoubleClick}
        onContextMenu={handleContextMenu}
      />
      {imageLoaded && (
        <div className="overlay-hint">
          {getHint()}
        </div>
      )}
      {/* Buttons for platforms where keyboard might not work */}
      {selection && !isSelecting && (
        <div className="overlay-buttons" onMouseDown={(e) => e.stopPropagation()}>
          <button onClick={confirmSelection} className="overlay-btn confirm">
            ✓ {t("overlay.confirm")}
          </button>
          <button onClick={() => setSelection(null)} className="overlay-btn reset">
            ↺ {t("overlay.reselect")}
          </button>
          <button onClick={closeOverlay} className="overlay-btn cancel">
            ✕ {t("overlay.cancel")}
          </button>
        </div>
      )}
    </div>
  );
}

export default OverlayPage;

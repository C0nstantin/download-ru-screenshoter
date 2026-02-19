import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

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

function OverlayPage() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [screenshot, setScreenshot] = useState<ScreenshotData | null>(null);
  const [isSelecting, setIsSelecting] = useState(false);
  const [selection, setSelection] = useState<SelectionRect | null>(null);
  const [imageLoaded, setImageLoaded] = useState(false);

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
  }, []);

  // Draw the screenshot and selection
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

      // Draw full image
      ctx.drawImage(img, 0, 0, canvas.width, canvas.height);

      // Draw dark overlay on top
      ctx.fillStyle = "rgba(0, 0, 0, 0.4)";
      ctx.fillRect(0, 0, canvas.width, canvas.height);

      // If selection exists, draw highlighted region
      if (selection) {
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
  }, [screenshot, selection]);

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
      console.log("Cropping region:", { x: imgX, y: imgY, width: imgWidth, height: imgHeight });

      // Crop the image
      await invoke("crop_image", {
        region: { x: imgX, y: imgY, width: imgWidth, height: imgHeight },
      });

      console.log("Crop done, opening editor...");

      // Open editor window BEFORE closing overlay
      await invoke("open_editor");
      console.log("open_editor returned");

      console.log("Editor opened, closing overlay...");

      // Close overlay after editor is opened
      await closeOverlay();
    } catch (err) {
      console.error("Failed to crop or open editor:", err);
      await closeOverlay();
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
      if (selection && !isSelecting) {
        // If there's a finished selection, reset it first
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

  return (
    <div className="overlay-page">
      <canvas
        ref={canvasRef}
        className="overlay-canvas"
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        onDoubleClick={handleDoubleClick}
        onContextMenu={handleContextMenu}
      />
      {imageLoaded && (
        <div className="overlay-hint">
          {selection && !isSelecting
            ? "Enter или двойной клик — подтвердить • ПКМ/R — перевыбрать • ESC — отмена"
            : "Выделите область • ESC — отмена"}
        </div>
      )}
    </div>
  );
}

export default OverlayPage;

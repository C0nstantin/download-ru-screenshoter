import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { writeText, writeImage } from "@tauri-apps/plugin-clipboard-manager";
import { save } from "@tauri-apps/plugin-dialog";
import { sendNotification, isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import { Stage, Layer, Image as KonvaImage, Arrow, Rect, Text, Transformer, Circle, Group } from "react-konva";
import Konva from "konva";
import { useEditorStore, Shape } from "../stores/editorStore";

// NumberMarker component - renders a numbered circle
interface NumberMarkerProps {
  id: string;
  x: number;
  y: number;
  number: number;
  color: string;
  draggable: boolean;
  onClick: () => void;
  onTap: () => void;
}

function NumberMarker({ id, x, y, number, color, draggable, onClick, onTap }: NumberMarkerProps) {
  const radius = 16;
  return (
    <Group
      id={id}
      x={x}
      y={y}
      draggable={draggable}
      onClick={onClick}
      onTap={onTap}
    >
      <Circle
        radius={radius}
        fill={color}
        stroke="#fff"
        strokeWidth={2}
      />
      <Text
        text={String(number)}
        fontSize={16}
        fontStyle="bold"
        fill="#fff"
        align="center"
        verticalAlign="middle"
        width={radius * 2}
        height={radius * 2}
        offsetX={radius}
        offsetY={radius}
      />
    </Group>
  );
}
import Toolbar from "../components/Toolbar";

// BlurRect component - renders a pixelated/blurred region
interface BlurRectProps {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  image: HTMLImageElement | null;
  draggable: boolean;
  onClick: () => void;
  onTap: () => void;
}

function BlurRect({ id, x, y, width, height, image, draggable, onClick, onTap }: BlurRectProps) {
  const [blurredImage, setBlurredImage] = useState<HTMLCanvasElement | null>(null);

  // Normalize negative width/height
  const nx = width < 0 ? x + width : x;
  const ny = height < 0 ? y + height : y;
  const nw = Math.abs(width);
  const nh = Math.abs(height);

  useEffect(() => {
    if (!image || nw === 0 || nh === 0) return;

    // Create pixelated version of the region
    const pixelSize = 10;
    const canvas = document.createElement("canvas");
    canvas.width = nw;
    canvas.height = nh;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // Draw the region from the original image
    ctx.drawImage(image, nx, ny, nw, nh, 0, 0, nw, nh);

    // Pixelate: scale down then scale up
    const smallW = Math.max(1, Math.floor(nw / pixelSize));
    const smallH = Math.max(1, Math.floor(nh / pixelSize));

    // Create small canvas
    const smallCanvas = document.createElement("canvas");
    smallCanvas.width = smallW;
    smallCanvas.height = smallH;
    const smallCtx = smallCanvas.getContext("2d");
    if (!smallCtx) return;

    // Draw scaled down
    smallCtx.drawImage(canvas, 0, 0, smallW, smallH);

    // Clear and draw scaled up (pixelated)
    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, nw, nh);
    ctx.drawImage(smallCanvas, 0, 0, smallW, smallH, 0, 0, nw, nh);

    setBlurredImage(canvas);
  }, [image, nx, ny, nw, nh]);

  if (!blurredImage || nw === 0 || nh === 0) return null;

  return (
    <KonvaImage
      id={id}
      image={blurredImage}
      x={nx}
      y={ny}
      width={nw}
      height={nh}
      draggable={draggable}
      onClick={onClick}
      onTap={onTap}
    />
  );
}

interface ScreenshotData {
  base64: string;
  width: number;
  height: number;
}

interface UploadResponse {
  id: string;
  name: string;
  secure_url: string;
  shared: boolean;
}

function EditorPage() {
  const [screenshot, setScreenshot] = useState<ScreenshotData | null>(null);
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [uploadResult, setUploadResult] = useState<UploadResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [scale, setScale] = useState<number | null>(null); // null = not calculated yet
  const containerRef = useRef<HTMLDivElement>(null);

  const stageRef = useRef<Konva.Stage>(null);
  const transformerRef = useRef<Konva.Transformer>(null);

  const {
    tool,
    color,
    strokeWidth,
    shapes,
    selectedId,
    addShape,
    deleteShape,
    setSelectedId,
    clearShapes,
    undo,
    redo,
    getNextNumber,
  } = useEditorStore();

  const [isDrawing, setIsDrawing] = useState(false);
  const [newShape, setNewShape] = useState<Shape | null>(null);
  const [isDraggingOver, setIsDraggingOver] = useState(false);

  // Load screenshot on mount
  useEffect(() => {
    invoke<ScreenshotData>("get_current_screenshot")
      .then((data) => {
        console.log("Got screenshot from backend:", data.width, "x", data.height);
        setScreenshot(data);
        const img = new window.Image();
        img.src = `data:image/png;base64,${data.base64}`;
        img.onload = () => {
          console.log("Image loaded, natural size:", img.naturalWidth, "x", img.naturalHeight);
          // Verify image matches reported dimensions
          if (img.naturalWidth !== data.width || img.naturalHeight !== data.height) {
            console.warn("Image size mismatch! Reported:", data.width, "x", data.height,
                         "Actual:", img.naturalWidth, "x", img.naturalHeight);
          }
          setImage(img);
        };
        img.onerror = (e) => {
          console.error("Image load error:", e);
          setError("Ошибка загрузки изображения");
        };
      })
      .catch((err) => {
        console.error("Failed to get screenshot:", err);
        setError("Не удалось загрузить скриншот: " + String(err));
      });

    // Clear shapes from previous session
    clearShapes();
  }, [clearShapes]);

  // Calculate initial scale to fit image in container
  useEffect(() => {
    if (!screenshot || !containerRef.current) return;

    const calculateScale = () => {
      const container = containerRef.current;
      if (!container) return;

      const padding = 40;
      const availableWidth = container.clientWidth - padding;
      const availableHeight = container.clientHeight - padding;

      if (availableWidth <= 0 || availableHeight <= 0) {
        // Container not ready, try again
        requestAnimationFrame(calculateScale);
        return;
      }

      const scaleX = availableWidth / screenshot.width;
      const scaleY = availableHeight / screenshot.height;
      const newScale = Math.min(scaleX, scaleY, 1);

      console.log("Scale:", newScale, "container:", availableWidth, "x", availableHeight, "image:", screenshot.width, "x", screenshot.height);
      setScale(newScale);
    };

    // Reset scale when screenshot changes, then calculate
    setScale(null);
    requestAnimationFrame(calculateScale);
  }, [screenshot]);

  // Handle mouse wheel zoom
  const handleWheel = useCallback((e: WheelEvent) => {
    if (!e.ctrlKey && !e.metaKey) return; // Require Ctrl/Cmd for zoom
    e.preventDefault();

    const delta = e.deltaY > 0 ? 0.9 : 1.1; // Zoom out or in
    setScale((prevScale) => {
      if (prevScale === null) return prevScale;
      const newScale = prevScale * delta;
      // Limit zoom range: 0.1x to 3x
      return Math.max(0.1, Math.min(3, newScale));
    });
  }, []);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    container.addEventListener("wheel", handleWheel, { passive: false });
    return () => container.removeEventListener("wheel", handleWheel);
  }, [handleWheel]);

  // Update transformer when selection changes
  useEffect(() => {
    if (!transformerRef.current || !stageRef.current) return;

    const stage = stageRef.current;
    const transformer = transformerRef.current;

    if (selectedId) {
      const node = stage.findOne(`#${selectedId}`);
      if (node) {
        transformer.nodes([node]);
        transformer.getLayer()?.batchDraw();
      }
    } else {
      transformer.nodes([]);
      transformer.getLayer()?.batchDraw();
    }
  }, [selectedId]);

  // Get pointer position in stage coordinates (accounting for CSS scale transform)
  const getScaledPointerPosition = () => {
    const stage = stageRef.current;
    if (!stage || scale === null) return null;

    const mousePos = stage.getPointerPosition();
    if (!mousePos) return null;

    // Since we use CSS transform scale, we need to divide by scale
    return {
      x: mousePos.x / scale,
      y: mousePos.y / scale,
    };
  };

  const handleMouseDown = (e: Konva.KonvaEventObject<MouseEvent>) => {
    // If clicking on empty area, deselect
    if (e.target === e.target.getStage()) {
      setSelectedId(null);
    }

    if (tool === "select") return;

    const pos = getScaledPointerPosition();
    if (!pos) return;

    setIsDrawing(true);

    const id = `shape_${Date.now()}`;
    const baseShape = {
      id,
      color,
      strokeWidth,
      x: pos.x,
      y: pos.y,
    };

    if (tool === "arrow") {
      setNewShape({
        ...baseShape,
        type: "arrow",
        points: [0, 0, 0, 0],
      });
    } else if (tool === "rect") {
      setNewShape({
        ...baseShape,
        type: "rect",
        width: 0,
        height: 0,
      });
    } else if (tool === "blur") {
      setNewShape({
        ...baseShape,
        type: "blur",
        width: 0,
        height: 0,
      });
    } else if (tool === "text") {
      const text = prompt("Введите текст:");
      if (text) {
        addShape({
          ...baseShape,
          type: "text",
          text,
        });
      }
      setIsDrawing(false);
    } else if (tool === "number") {
      addShape({
        ...baseShape,
        type: "number",
        number: getNextNumber(),
      });
      setIsDrawing(false);
    }
  };

  const handleMouseMove = () => {
    if (!isDrawing || !newShape) return;

    const pos = getScaledPointerPosition();
    if (!pos) return;

    if (newShape.type === "arrow") {
      setNewShape({
        ...newShape,
        points: [0, 0, pos.x - newShape.x, pos.y - newShape.y],
      });
    } else if (newShape.type === "rect" || newShape.type === "blur") {
      setNewShape({
        ...newShape,
        width: pos.x - newShape.x,
        height: pos.y - newShape.y,
      });
    }
  };

  const handleMouseUp = () => {
    if (!isDrawing || !newShape) return;

    setIsDrawing(false);

    // Check minimum size
    if (newShape.type === "arrow") {
      const points = newShape.points || [0, 0, 0, 0];
      const length = Math.sqrt(points[2] ** 2 + points[3] ** 2);
      if (length > 10) {
        addShape(newShape);
      }
    } else if (newShape.type === "rect" || newShape.type === "blur") {
      const w = Math.abs(newShape.width || 0);
      const h = Math.abs(newShape.height || 0);
      if (w > 5 && h > 5) {
        addShape(newShape);
      }
    }

    setNewShape(null);
  };

  const handleShapeClick = (id: string) => {
    if (tool === "select") {
      setSelectedId(id);
    }
  };

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        handleClose();
      } else if (e.key === "Delete" || e.key === "Backspace") {
        if (selectedId) {
          deleteShape(selectedId);
        }
      } else if ((e.ctrlKey || e.metaKey) && e.key === "z") {
        e.preventDefault();
        if (e.shiftKey) {
          redo();
        } else {
          undo();
        }
      } else if ((e.ctrlKey || e.metaKey) && e.key === "y") {
        e.preventDefault();
        redo();
      }
    },
    [selectedId, deleteShape, undo, redo]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  const handleClose = async () => {
    const win = getCurrentWindow();
    await win.hide();
    clearShapes();
  };

  const getStageDataUrl = (): string => {
    if (!stageRef.current || scale === null) return "";
    // Export at original size regardless of display scale
    return stageRef.current.toDataURL({ pixelRatio: 1 / scale });
  };

  const handleSave = async () => {
    const path = await save({
      defaultPath: `screenshot_${Date.now()}.png`,
      filters: [{ name: "PNG", extensions: ["png"] }],
    });

    if (path) {
      try {
        const dataUrl = getStageDataUrl();
        const base64 = dataUrl.split(",")[1];
        await invoke("save_screenshot", { path, imageData: base64 });
        setError(null);
      } catch (err) {
        setError(`Ошибка сохранения: ${err}`);
      }
    }
  };

  const doUpload = async () => {
    const dataUrl = getStageDataUrl();
    const base64 = dataUrl.split(",")[1];
    const filename = `screenshot_${Date.now()}.png`;

    const result = await invoke<UploadResponse>("upload_to_download", {
      filename,
      imageData: base64,
    });

    setUploadResult(result);
    await writeText(result.secure_url);

    let permissionGranted = await isPermissionGranted();
    if (!permissionGranted) {
      const permission = await requestPermission();
      permissionGranted = permission === "granted";
    }
    if (permissionGranted) {
      sendNotification({
        title: "Скриншот загружен",
        body: `Ссылка скопирована: ${result.secure_url}`,
      });
    }
  };

  const handleUpload = async () => {
    setIsUploading(true);
    setError(null);
    setUploadResult(null);

    try {
      // Check token — if missing, open auth and retry after success
      const token = await invoke<string | null>("get_access_token");
      if (!token) {
        await invoke("open_oauth_browser");
        // Wait for oauth-complete event then retry
        const unlisten = await listen<boolean>("oauth-complete", async (event) => {
          unlisten();
          if (event.payload) {
            setIsUploading(true);
            setError(null);
            try {
              await doUpload();
            } catch (err) {
              setError(`Ошибка загрузки: ${err}`);
            } finally {
              setIsUploading(false);
            }
          } else {
            setIsUploading(false);
            setError("Авторизация не удалась. Попробуйте ещё раз.");
          }
        });
        return; // will finish in listener
      }

      await doUpload();
    } catch (err) {
      const errorMsg = String(err);
      if (errorMsg.includes("401") || errorMsg.includes("unauthorized") || errorMsg.includes("No access token")) {
        // Token expired — re-auth and retry
        setError(null);
        try {
          await invoke("open_oauth_browser");
          const unlisten = await listen<boolean>("oauth-complete", async (event) => {
            unlisten();
            if (event.payload) {
              setIsUploading(true);
              try {
                await doUpload();
              } catch (e) {
                setError(`Ошибка загрузки: ${e}`);
              } finally {
                setIsUploading(false);
              }
            } else {
              setIsUploading(false);
              setError("Авторизация не удалась. Попробуйте ещё раз.");
            }
          });
          return;
        } catch (authErr) {
          setError(`Ошибка авторизации: ${authErr}`);
        }
      } else if (errorMsg.includes("network") || errorMsg.includes("connection") || errorMsg.includes("timeout")) {
        setError("Ошибка сети. Проверьте подключение к интернету.");
      } else {
        setError(`Ошибка загрузки: ${err}`);
      }
    } finally {
      setIsUploading(false);
    }
  };

  const handleCopyUrl = async () => {
    if (uploadResult) {
      await writeText(uploadResult.secure_url);
    }
  };

  const handleCopyImage = async () => {
    try {
      const dataUrl = getStageDataUrl();
      const base64 = dataUrl.split(",")[1];
      // Convert base64 to Uint8Array for clipboard
      const binaryString = atob(base64);
      const bytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
      }
      await writeImage(bytes);
      setError(null);
      // Show brief success message
      setUploadResult({ id: "", name: "", secure_url: "Изображение скопировано!", shared: false });
      setTimeout(() => setUploadResult(null), 2000);
    } catch (err) {
      setError(`Ошибка копирования: ${err}`);
    }
  };

  // Drag & drop handlers
  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDraggingOver(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDraggingOver(false);
  };

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDraggingOver(false);

    const files = e.dataTransfer.files;
    if (files.length === 0) return;

    const file = files[0];
    if (!file.type.startsWith("image/")) {
      setError("Можно загружать только изображения");
      return;
    }

    try {
      const reader = new FileReader();
      reader.onload = (event) => {
        const dataUrl = event.target?.result as string;
        const base64 = dataUrl.split(",")[1];

        const img = new window.Image();
        img.onload = () => {
          setScreenshot({
            base64,
            width: img.width,
            height: img.height,
          });
          setImage(img);
          clearShapes();
          setError(null);
        };
        img.onerror = () => {
          setError("Не удалось загрузить изображение");
        };
        img.src = dataUrl;
      };
      reader.readAsDataURL(file);
    } catch (err) {
      setError(`Ошибка загрузки файла: ${err}`);
    }
  };

  // Show loading state but keep the full layout for proper scale calculation
  if (!screenshot || !image) {
    return (
      <div className="editor-page">
        <Toolbar />
        <div className="editor-canvas-container" ref={containerRef}>
          <p style={{ color: '#888' }}>Загрузка...</p>
        </div>
        <div className="editor-actions">
          <button disabled>Загрузка...</button>
        </div>
      </div>
    );
  }

  return (
    <div
      className={`editor-page ${isDraggingOver ? "dragging-over" : ""}`}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <Toolbar />

      <div className="editor-canvas-container" ref={containerRef}>
        {scale !== null ? (
          <Stage
            ref={stageRef}
            width={Math.round(screenshot.width * scale)}
            height={Math.round(screenshot.height * scale)}
            scaleX={scale}
            scaleY={scale}
            onMouseDown={handleMouseDown}
            onMouseMove={handleMouseMove}
            onMouseUp={handleMouseUp}
          >
            <Layer>
              <KonvaImage image={image} x={0} y={0} />
            </Layer>

          <Layer>
            {/* Render shapes */}
            {shapes.map((shape) => {
              if (shape.type === "arrow") {
                return (
                  <Arrow
                    key={shape.id}
                    id={shape.id}
                    x={shape.x}
                    y={shape.y}
                    points={shape.points || []}
                    stroke={shape.color}
                    strokeWidth={shape.strokeWidth}
                    fill={shape.color}
                    pointerLength={10}
                    pointerWidth={10}
                    draggable={tool === "select"}
                    onClick={() => handleShapeClick(shape.id)}
                    onTap={() => handleShapeClick(shape.id)}
                  />
                );
              } else if (shape.type === "rect") {
                return (
                  <Rect
                    key={shape.id}
                    id={shape.id}
                    x={shape.x}
                    y={shape.y}
                    width={shape.width}
                    height={shape.height}
                    stroke={shape.color}
                    strokeWidth={shape.strokeWidth}
                    draggable={tool === "select"}
                    onClick={() => handleShapeClick(shape.id)}
                    onTap={() => handleShapeClick(shape.id)}
                  />
                );
              } else if (shape.type === "text") {
                return (
                  <Text
                    key={shape.id}
                    id={shape.id}
                    x={shape.x}
                    y={shape.y}
                    text={shape.text}
                    fontSize={20}
                    fill={shape.color}
                    draggable={tool === "select"}
                    onClick={() => handleShapeClick(shape.id)}
                    onTap={() => handleShapeClick(shape.id)}
                  />
                );
              } else if (shape.type === "blur") {
                return (
                  <BlurRect
                    key={shape.id}
                    id={shape.id}
                    x={shape.x}
                    y={shape.y}
                    width={shape.width || 0}
                    height={shape.height || 0}
                    image={image}
                    draggable={tool === "select"}
                    onClick={() => handleShapeClick(shape.id)}
                    onTap={() => handleShapeClick(shape.id)}
                  />
                );
              } else if (shape.type === "number") {
                return (
                  <NumberMarker
                    key={shape.id}
                    id={shape.id}
                    x={shape.x}
                    y={shape.y}
                    number={shape.number || 1}
                    color={shape.color}
                    draggable={tool === "select"}
                    onClick={() => handleShapeClick(shape.id)}
                    onTap={() => handleShapeClick(shape.id)}
                  />
                );
              }
              return null;
            })}

            {/* Render shape being drawn */}
            {newShape && newShape.type === "arrow" && (
              <Arrow
                x={newShape.x}
                y={newShape.y}
                points={newShape.points || []}
                stroke={newShape.color}
                strokeWidth={newShape.strokeWidth}
                fill={newShape.color}
                pointerLength={10}
                pointerWidth={10}
              />
            )}
            {newShape && newShape.type === "rect" && (
              <Rect
                x={newShape.x}
                y={newShape.y}
                width={newShape.width}
                height={newShape.height}
                stroke={newShape.color}
                strokeWidth={newShape.strokeWidth}
              />
            )}
            {newShape && newShape.type === "blur" && (
              <Rect
                x={newShape.width && newShape.width < 0 ? newShape.x + newShape.width : newShape.x}
                y={newShape.height && newShape.height < 0 ? newShape.y + newShape.height : newShape.y}
                width={Math.abs(newShape.width || 0)}
                height={Math.abs(newShape.height || 0)}
                fill="rgba(128, 128, 128, 0.5)"
                stroke="#888"
                strokeWidth={1}
                dash={[5, 5]}
              />
            )}

            <Transformer ref={transformerRef} />
          </Layer>
        </Stage>
        ) : (
          <p style={{ color: '#888' }}>Подготовка...</p>
        )}
      </div>

      <div className="editor-actions">
        <div className="zoom-controls">
          <span className="zoom-label">{Math.round((scale ?? 1) * 100)}%</span>
          <button
            onClick={() => setScale(1)}
            className="zoom-reset"
            title="Сбросить масштаб (100%)"
          >
            ⟲
          </button>
        </div>
        <button onClick={handleCopyImage} title="Копировать изображение в буфер">
          Копировать
        </button>
        <button onClick={handleSave}>Сохранить</button>
        <button
          onClick={handleUpload}
          disabled={isUploading}
          className="primary"
        >
          {isUploading ? "Загрузка..." : "Загрузить"}
        </button>
        <button onClick={handleClose} className="secondary">
          Закрыть
        </button>
      </div>

      {uploadResult && (
        <div className="upload-result success">
          <p>Загружено! Ссылка скопирована в буфер.</p>
          <div className="url-row">
            <input type="text" value={uploadResult.secure_url} readOnly />
            <button onClick={handleCopyUrl}>Копировать</button>
          </div>
        </div>
      )}

      {error && <div className="upload-result error">{error}</div>}
    </div>
  );
}

export default EditorPage;

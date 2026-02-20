import { ReactNode } from "react";
import { useEditorStore, Tool } from "../stores/editorStore";

const CursorIcon = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
    <path d="M3 1L3 12L6.5 8.5L10 14L12 13L8.5 7L13 7L3 1Z" />
  </svg>
);

const tools: { id: Tool; label: string; icon: ReactNode }[] = [
  { id: "select", label: "Выбор", icon: <CursorIcon /> },
  { id: "arrow", label: "Стрелка", icon: "→" },
  { id: "rect", label: "Рамка", icon: "□" },
  { id: "text", label: "Текст", icon: "T" },
  { id: "number", label: "Номер", icon: "①" },
  { id: "blur", label: "Размытие", icon: "▦" },
];

const colors = [
  "#ff0000",
  "#ff6600",
  "#ffcc00",
  "#00cc00",
  "#0066ff",
  "#9933ff",
  "#000000",
  "#ffffff",
];

function Toolbar() {
  const { tool, color, strokeWidth, fontSize, setTool, setColor, setStrokeWidth, setFontSize } =
    useEditorStore();

  return (
    <div className="toolbar">
      <div className="toolbar-section">
        <span className="toolbar-label">Инструмент:</span>
        <div className="tool-buttons">
          {tools.map((t) => (
            <button
              key={t.id}
              className={`tool-btn ${tool === t.id ? "active" : ""}`}
              onClick={() => setTool(t.id)}
              title={t.label}
            >
              {t.icon}
            </button>
          ))}
        </div>
      </div>

      <div className="toolbar-section">
        <span className="toolbar-label">Цвет:</span>
        <div className="color-buttons">
          {colors.map((c) => (
            <button
              key={c}
              className={`color-btn ${color === c ? "active" : ""}`}
              style={{ backgroundColor: c }}
              onClick={() => setColor(c)}
            />
          ))}
        </div>
      </div>

      <div className="toolbar-section">
        <span className="toolbar-label">Толщина:</span>
        <input
          type="range"
          min="1"
          max="10"
          value={strokeWidth}
          onChange={(e) => setStrokeWidth(Number(e.target.value))}
        />
        <span className="stroke-value">{strokeWidth}px</span>
      </div>

      {tool === "text" && (
        <div className="toolbar-section">
          <span className="toolbar-label">Шрифт:</span>
          <input
            type="range"
            min="10"
            max="72"
            value={fontSize}
            onChange={(e) => setFontSize(Number(e.target.value))}
          />
          <span className="stroke-value">{fontSize}px</span>
        </div>
      )}
    </div>
  );
}

export default Toolbar;

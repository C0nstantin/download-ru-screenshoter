import { useEditorStore, Tool } from "../stores/editorStore";

const tools: { id: Tool; label: string; icon: string }[] = [
  { id: "select", label: "Выбор", icon: "↖" },
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
  const { tool, color, strokeWidth, setTool, setColor, setStrokeWidth } =
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
    </div>
  );
}

export default Toolbar;

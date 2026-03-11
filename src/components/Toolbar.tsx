import { ReactNode } from "react";
import { useEditorStore, Tool } from "../stores/editorStore";
import { useTranslation } from "../i18n/useTranslation";

const CursorIcon = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
    <path d="M3 1L3 12L6.5 8.5L10 14L12 13L8.5 7L13 7L3 1Z" />
  </svg>
);

const toolDefs: { id: Tool; labelKey: string; icon: ReactNode }[] = [
  { id: "select", labelKey: "toolbar.select", icon: <CursorIcon /> },
  { id: "arrow", labelKey: "toolbar.arrow", icon: "→" },
  { id: "rect", labelKey: "toolbar.rect", icon: "□" },
  { id: "text", labelKey: "toolbar.text", icon: "T" },
  { id: "number", labelKey: "toolbar.number", icon: "①" },
  { id: "blur", labelKey: "toolbar.blur", icon: "▦" },
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
  const { t } = useTranslation();

  return (
    <div className="toolbar">
      <div className="toolbar-section">
        <span className="toolbar-label">{t("toolbar.tool")}</span>
        <div className="tool-buttons">
          {toolDefs.map((td) => (
            <button
              key={td.id}
              className={`tool-btn ${tool === td.id ? "active" : ""}`}
              onClick={() => setTool(td.id)}
              title={t(td.labelKey)}
            >
              {td.icon}
            </button>
          ))}
        </div>
      </div>

      <div className="toolbar-section">
        <span className="toolbar-label">{t("toolbar.color")}</span>
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
        <span className="toolbar-label">{t("toolbar.thickness")}</span>
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
          <span className="toolbar-label">{t("toolbar.font")}</span>
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

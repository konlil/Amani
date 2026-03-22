import { useAppStore } from "../../stores/appStore";

export default function Toolbar() {
  const { infoMode, setInfoMode, runMode, project } = useAppStore();

  return (
    <div className="toolbar">
      <div className="toolbar-section">
        <span className="toolbar-title">LLM3dEngine</span>
        {project && (
          <span className="toolbar-project">{project.name}</span>
        )}
      </div>
      <div className="toolbar-section">
        <button
          className={`toolbar-btn ${infoMode === "normal" ? "active" : ""}`}
          onClick={() => setInfoMode("normal")}
        >
          普通
        </button>
        <button
          className={`toolbar-btn ${infoMode === "advanced" ? "active" : ""}`}
          onClick={() => setInfoMode("advanced")}
        >
          高级
        </button>
        <span style={{ color: "var(--text-muted)", fontSize: 12 }}>|</span>
        <span
          style={{
            fontSize: 12,
            color: runMode === "dev" ? "var(--green)" : "var(--yellow)",
          }}
        >
          {runMode === "dev" ? "开发模式" : "Play 模式"}
        </span>
      </div>
    </div>
  );
}

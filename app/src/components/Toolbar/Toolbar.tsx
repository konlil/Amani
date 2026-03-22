import { useState } from "react";
import { useAppStore } from "../../stores/appStore";

function SettingsModal({ onClose }: { onClose: () => void }) {
  const { llmConfig, setLlmConfig } = useAppStore();
  const [apiKey, setApiKey] = useState(llmConfig.api_key);
  const [endpoint, setEndpoint] = useState(llmConfig.endpoint);
  const [model, setModel] = useState(llmConfig.model);
  const [enginePath, setEnginePath] = useState(llmConfig.engine_path);

  const handleSave = () => {
    setLlmConfig({ api_key: apiKey, endpoint, model, engine_path: enginePath });
    onClose();
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>设置</h2>
        <div className="modal-field">
          <label>API Key</label>
          <input
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="sk-..."
          />
        </div>
        <div className="modal-field">
          <label>API Endpoint</label>
          <input
            value={endpoint}
            onChange={(e) => setEndpoint(e.target.value)}
            placeholder="https://api.openai.com/v1"
          />
        </div>
        <div className="modal-field">
          <label>模型</label>
          <input
            value={model}
            onChange={(e) => setModel(e.target.value)}
            placeholder="gpt-4"
          />
        </div>
        <div className="modal-field">
          <label>引擎路径</label>
          <input
            value={enginePath}
            onChange={(e) => setEnginePath(e.target.value)}
            placeholder="/path/to/godot.macos.template_release.arm64"
          />
        </div>
        <div className="modal-actions">
          <button className="toolbar-btn" onClick={onClose}>取消</button>
          <button className="toolbar-btn active" onClick={handleSave}>保存</button>
        </div>
      </div>
    </div>
  );
}

export default function Toolbar() {
  const { infoMode, setInfoMode, runMode, project } = useAppStore();
  const [showSettings, setShowSettings] = useState(false);

  return (
    <>
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
          <button className="toolbar-btn" onClick={() => setShowSettings(true)}>
            设置
          </button>
        </div>
      </div>
      {showSettings && <SettingsModal onClose={() => setShowSettings(false)} />}
    </>
  );
}

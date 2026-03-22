import { useAppStore } from "../../stores/appStore";

export default function PreviewPanel() {
  const { screenshotUrl, engineRunning } = useAppStore();

  return (
    <div className="preview-panel">
      <div className="preview-header">
        <span>预览</span>
        <span
          style={{
            color: engineRunning ? "var(--green)" : "var(--text-muted)",
          }}
        >
          {engineRunning ? "● 引擎运行中" : "○ 引擎未启动"}
        </span>
      </div>
      <div className="preview-content">
        {screenshotUrl ? (
          <>
            <img
              className="preview-screenshot"
              src={screenshotUrl}
              alt="Engine preview"
            />
            <div className="preview-status-bar">
              <span>FPS: --</span>
              <span>场景: default</span>
              <span>实体: --</span>
            </div>
          </>
        ) : (
          <div className="preview-placeholder">
            <div className="preview-placeholder-icon">🎮</div>
            <div className="preview-placeholder-text">
              {engineRunning
                ? "等待引擎截屏..."
                : "创建或打开项目以启动引擎"}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

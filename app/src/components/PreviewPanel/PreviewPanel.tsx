import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useAppStore } from "../../stores/appStore";

export default function PreviewPanel() {
  const { screenshotUrl, engineRunning, setScreenshotUrl, setEngineRunning } =
    useAppStore();

  useEffect(() => {
    // Listen for engine responses containing screenshots
    const unlistenResponse = listen<Record<string, unknown>>(
      "engine-response",
      (event) => {
        const data = event.payload;
        console.log("[preview] engine response:", data);

        // Handle screenshot from compile_and_run or screenshot command
        const screenshotPath =
          (data.screenshot as string) ??
          (data.type === "screenshot" ? (data.path as string) : null);

        if (screenshotPath) {
          // Convert local file path to Tauri asset URL
          const url = convertFileSrc(screenshotPath);
          setScreenshotUrl(url + "?t=" + Date.now());
        }
      }
    );

    const unlistenStopped = listen("engine-stopped", () => {
      console.log("[preview] engine stopped");
      setEngineRunning(false);
    });

    return () => {
      unlistenResponse.then((f) => f());
      unlistenStopped.then((f) => f());
    };
  }, [setScreenshotUrl, setEngineRunning]);

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

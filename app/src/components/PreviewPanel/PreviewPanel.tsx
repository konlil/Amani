import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppStore } from "../../stores/appStore";

export default function PreviewPanel() {
  const containerRef = useRef<HTMLDivElement>(null);
  const { engineRunning, setEngineRunning } = useAppStore();

  // Listen for engine-stopped event
  useEffect(() => {
    const unlistenStopped = listen("engine-stopped", () => {
      setEngineRunning(false);
    });
    return () => {
      unlistenStopped.then((f) => f());
    };
  }, [setEngineRunning]);

  // When engine starts, embed the window.
  // Call immediately (Rust side polls up to 5s for the handle),
  // and also listen for the event in case the first attempt times out.
  useEffect(() => {
    if (!engineRunning) return;
    let cancelled = false;

    const doEmbed = () => {
      if (cancelled) return;
      invoke("embed_engine_window").catch((err: unknown) => {
        console.error("[preview] embed failed:", err);
      });
    };

    doEmbed();
    const unlistenHandle = listen("engine-window-handle", doEmbed);

    return () => {
      cancelled = true;
      unlistenHandle.then((f) => f());
    };
  }, [engineRunning]);

  // Sync container bounds to the native child window
  useEffect(() => {
    if (!containerRef.current || !engineRunning) return;

    const sync = () => {
      const el = containerRef.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      invoke("update_engine_bounds", {
        x: rect.left,
        y: rect.top,
        width: rect.width,
        height: rect.height,
        scaleFactor: window.devicePixelRatio,
      }).catch(() => {});
    };

    const observer = new ResizeObserver(sync);
    observer.observe(containerRef.current);
    window.addEventListener("resize", sync);
    sync();

    return () => {
      observer.disconnect();
      window.removeEventListener("resize", sync);
    };
  }, [engineRunning]);

  // Sync overlay position when Tauri window moves, and hide/show on focus changes
  useEffect(() => {
    if (!engineRunning) return;
    const appWindow = getCurrentWindow();

    const syncBounds = () => {
      const el = containerRef.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      invoke("update_engine_bounds", {
        x: rect.left,
        y: rect.top,
        width: rect.width,
        height: rect.height,
        scaleFactor: window.devicePixelRatio,
      }).catch(() => {});
    };

    const unlistenMove = appWindow.onMoved(() => {
      invoke("reposition_engine_overlay").catch(() => {});
    });

    const unlistenFocus = appWindow.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        invoke("show_engine_window").catch(() => {});
        syncBounds();
      } else {
        invoke("hide_engine_window").catch(() => {});
      }
    });

    return () => {
      unlistenMove.then((f) => f());
      unlistenFocus.then((f) => f());
    };
  }, [engineRunning]);

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
      <div ref={containerRef} className="preview-content engine-viewport">
        {!engineRunning && (
          <div className="preview-placeholder">
            <div className="preview-placeholder-icon">🎮</div>
            <div className="preview-placeholder-text">
              创建或打开项目以启动引擎
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

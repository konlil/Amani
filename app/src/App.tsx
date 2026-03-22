import { useState, useCallback, useRef, useEffect } from "react";
import Toolbar from "./components/Toolbar/Toolbar";
import ChatPanel from "./components/ChatPanel/ChatPanel";
import PreviewPanel from "./components/PreviewPanel/PreviewPanel";
import { useAppStore } from "./stores/appStore";

export default function App() {
  const { splitRatio, setSplitRatio } = useAppStore();
  const [isDragging, setIsDragging] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const handleMouseDown = useCallback(() => {
    setIsDragging(true);
  }, []);

  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!isDragging || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const ratio = (e.clientX - rect.left) / rect.width;
      setSplitRatio(Math.max(0.2, Math.min(0.8, ratio)));
    },
    [isDragging, setSplitRatio]
  );

  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
  }, []);

  useEffect(() => {
    if (isDragging) {
      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", handleMouseUp);
      return () => {
        window.removeEventListener("mousemove", handleMouseMove);
        window.removeEventListener("mouseup", handleMouseUp);
      };
    }
  }, [isDragging, handleMouseMove, handleMouseUp]);

  return (
    <div className="app-container">
      <Toolbar />
      <div
        className="main-layout"
        ref={containerRef}
        style={{ cursor: isDragging ? "col-resize" : undefined }}
      >
        <div style={{ width: `${splitRatio * 100}%`, minWidth: 0, height: "100%", overflow: "hidden" }}>
          <ChatPanel />
        </div>
        <div className="split-handle" onMouseDown={handleMouseDown} />
        <div style={{ flex: 1, minWidth: 0, height: "100%", overflow: "hidden" }}>
          <PreviewPanel />
        </div>
      </div>
    </div>
  );
}

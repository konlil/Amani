import type { ChatMessage } from "../../types";
import { useAppStore } from "../../stores/appStore";

function CodeBlock({ code, filename, language }: { code: string; filename: string; language: string }) {
  const [collapsed, setCollapsed] = useState(false);
  return (
    <div className="code-block">
      <div className="code-block-header" onClick={() => setCollapsed(!collapsed)} style={{ cursor: "pointer" }}>
        <span>{filename} ({language})</span>
        <span>{collapsed ? "▶" : "▼"}</span>
      </div>
      {!collapsed && <pre className="code-block-body"><code>{code}</code></pre>}
    </div>
  );
}

function CompileResultBlock({ result }: { result: { success: boolean; output: string; errors: string[] } }) {
  return (
    <div className={`compile-result ${result.success ? "success" : "error"}`}>
      {result.success ? "✓ 编译成功" : `✗ 编译失败 (${result.errors.length} 个错误)`}
      {!result.success && result.errors.length > 0 && (
        <pre style={{ marginTop: 4, fontSize: 11 }}>{result.errors.join("\n")}</pre>
      )}
    </div>
  );
}

function MessageBubble({ message }: { message: ChatMessage }) {
  const { infoMode } = useAppStore();
  const showTechnical = infoMode === "advanced";

  // Simple markdown-like rendering: split by code blocks
  const renderContent = (content: string) => {
    if (!showTechnical) {
      // Strip code blocks in normal mode
      return content.replace(/```[\s\S]*?```/g, "[代码已生成]");
    }
    return content;
  };

  return (
    <div className={`message-bubble ${message.role}`}>
      <div>{renderContent(message.content)}</div>
      {showTechnical && message.codeBlocks?.map((block, i) => (
        <CodeBlock key={i} {...block} />
      ))}
      {showTechnical && message.compileResult && (
        <CompileResultBlock result={message.compileResult} />
      )}
    </div>
  );
}

import { useRef, useEffect, useState } from "react";

export default function ChatPanel() {
  const { messages } = useAppStore();
  const listRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom
  useEffect(() => {
    if (listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
  }, [messages]);

  return (
    <div className="chat-panel">
      <div className="message-list" ref={listRef}>
        {messages.length === 0 && (
          <div style={{ textAlign: "center", color: "var(--text-muted)", marginTop: 40 }}>
            <div style={{ fontSize: 32, marginBottom: 8 }}>💬</div>
            <div>描述你想要的游戏，LLM 会帮你实现</div>
          </div>
        )}
        {messages.map((msg) => (
          <MessageBubble key={msg.id} message={msg} />
        ))}
      </div>
      <ChatInput />
    </div>
  );
}

function ChatInput() {
  const [input, setInput] = useState("");
  const { addMessage, isStreaming } = useAppStore();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const handleSend = () => {
    const text = input.trim();
    if (!text || isStreaming) return;

    addMessage({
      id: crypto.randomUUID(),
      role: "user",
      content: text,
      timestamp: Date.now(),
    });
    setInput("");

    // Reset textarea height
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleInput = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInput(e.target.value);
    // Auto-resize
    const el = e.target;
    el.style.height = "auto";
    el.style.height = Math.min(el.scrollHeight, 120) + "px";
  };

  return (
    <div className="chat-input-container">
      <div className="chat-input-wrapper">
        <textarea
          ref={textareaRef}
          className="chat-input"
          value={input}
          onChange={handleInput}
          onKeyDown={handleKeyDown}
          placeholder="描述你想要的游戏功能..."
          rows={1}
        />
        <button
          className="send-btn"
          onClick={handleSend}
          disabled={!input.trim() || isStreaming}
        >
          发送
        </button>
      </div>
    </div>
  );
}

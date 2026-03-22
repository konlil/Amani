import { useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "../stores/appStore";
import { chatCompletion, extractCodeBlocks } from "../services/llm";
import { writeProjectFile, compileProject } from "../services/engine";

const ENGINE_DTS_CONTEXT = `You are a game development assistant for LLM3dEngine.
The engine uses TypeScript compiled to JavaScript and executed via QuickJS.
Available API is defined in the Engine namespace. Write TypeScript code using Engine.* APIs.
When generating code, wrap it in a code block with the target filename, e.g.:
\`\`\`typescript // scripts/enemy.ts
// code here
\`\`\`
`;

export function useChat() {
  const {
    messages,
    addMessage,
    appendToLastAssistant,
    llmConfig,
    project,
    setIsStreaming,
  } = useAppStore();

  const sendMessage = useCallback(
    async (userText: string) => {
      if (!llmConfig.api_key) {
        addMessage({
          id: crypto.randomUUID(),
          role: "assistant",
          content: "请先在设置中配置 API Key。",
          timestamp: Date.now(),
        });
        return;
      }

      setIsStreaming(true);

      // Add placeholder for assistant response
      const assistantId = crypto.randomUUID();
      addMessage({
        id: assistantId,
        role: "assistant",
        content: "",
        timestamp: Date.now(),
      });

      // Listen for stream events
      const unlisten = await listen<{ chunk: string; done: boolean }>(
        "llm-stream",
        (event) => {
          if (event.payload.done) return;
          appendToLastAssistant(event.payload.chunk);
        }
      );

      try {
        // Build message history for API
        const apiMessages = messages
          .filter((m) => m.role !== "system")
          .map((m) => ({ role: m.role, content: m.content }));
        apiMessages.push({ role: "user", content: userText });

        const fullResponse = await chatCompletion(
          llmConfig,
          apiMessages,
          ENGINE_DTS_CONTEXT
        );

        // Extract and process code blocks
        const codeBlocks = extractCodeBlocks(fullResponse);

        if (codeBlocks.length > 0 && project) {
          // Write code files to project
          for (const block of codeBlocks) {
            await writeProjectFile(project.path, block.filename, block.code);
          }

          // Compile
          const result = await compileProject(project.path);

          // Add compile result as a system message
          addMessage({
            id: crypto.randomUUID(),
            role: "assistant",
            content: result.success
              ? "代码已编译成功，场景已更新。"
              : `编译失败：\n${result.errors.join("\n")}`,
            timestamp: Date.now(),
            codeBlocks,
            compileResult: result,
          });
        }
      } catch (err) {
        appendToLastAssistant(
          `\n\n[错误: ${err instanceof Error ? err.message : String(err)}]`
        );
      } finally {
        unlisten();
        setIsStreaming(false);
      }
    },
    [messages, llmConfig, project, addMessage, appendToLastAssistant, setIsStreaming]
  );

  return { sendMessage };
}

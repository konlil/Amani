import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "../stores/appStore";
import { chatCompletion, extractCodeBlocks } from "../services/llm";
import { writeProjectFile, compileProject, createProject, startEngine, sendEngineCommand, pollEngineResult } from "../services/engine";
import { homeDir } from "@tauri-apps/api/path";

const ENGINE_DTS_CONTEXT = `You are a game development assistant for LLM3dEngine.
The engine uses TypeScript compiled to JavaScript and executed via QuickJS.
Engine is a global object — use Engine.* directly (e.g. Engine.Node3D, Engine.SceneTree).
Do NOT use import statements. All engine APIs are available globally via the Engine namespace.
When generating code, wrap it in a code block with the target filename, e.g.:
\`\`\`typescript // scripts/enemy.ts
// code here
\`\`\`
`;

export function useChat() {
  const sendMessage = async (userText: string) => {
    const store = useAppStore.getState();
    console.log("[useChat] sendMessage called:", userText);
    console.log("[useChat] llmConfig:", { ...store.llmConfig, api_key: store.llmConfig.api_key ? "***" : "" });

    if (!store.llmConfig.api_key) {
      store.addMessage({
        id: crypto.randomUUID(),
        role: "assistant",
        content: "请先在设置中配置 API Key。",
        timestamp: Date.now(),
      });
      return;
    }

    store.setIsStreaming(true);

    // Add placeholder for assistant response
    store.addMessage({
      id: crypto.randomUUID(),
      role: "assistant",
      content: "",
      timestamp: Date.now(),
    });

    // Listen for stream events
    const unlisten = await listen<{ chunk: string; done: boolean }>(
      "llm-stream",
      (event) => {
        if (event.payload.done) return;
        useAppStore.getState().appendToLastAssistant(event.payload.chunk);
      }
    );

    try {
      // Build message history for API (read fresh state)
      const currentMessages = useAppStore.getState().messages;
      const apiMessages = currentMessages
        .filter((m) => m.role !== "system" && m.content !== "")
        .map((m) => ({ role: m.role, content: m.content }));
      // The last one is the empty assistant placeholder, remove it
      if (apiMessages.length > 0 && apiMessages[apiMessages.length - 1].role === "assistant") {
        apiMessages.pop();
      }

      const fullResponse = await chatCompletion(
        store.llmConfig,
        apiMessages,
        ENGINE_DTS_CONTEXT
      );
      console.log("[useChat] LLM response received, length:", fullResponse.length);

      // Extract and process code blocks
      const codeBlocks = extractCodeBlocks(fullResponse);
      let project = useAppStore.getState().project;

      // Auto-create project if none exists and we have code to write
      if (codeBlocks.length > 0 && !project) {
        try {
          const home = await homeDir();
          const parentDir = home.endsWith("/") ? home + "LLM3dEngine-Projects" : home + "/LLM3dEngine-Projects";
          const projectName = `llm3d-project-${Date.now()}`;
          project = await createProject(parentDir, projectName);
          useAppStore.getState().setProject(project);
          console.log("[useChat] Auto-created project:", project.path);
        } catch (e) {
          console.error("[useChat] Failed to create project:", e);
        }
      }

      if (codeBlocks.length > 0 && project) {
        for (const block of codeBlocks) {
          await writeProjectFile(project.path, block.filename, block.code);
        }

        const result = await compileProject(project.path);

        useAppStore.getState().addMessage({
          id: crypto.randomUUID(),
          role: "assistant",
          content: result.success
            ? "代码已编译成功，场景已更新。"
            : `编译失败：\n${result.errors.join("\n")}`,
          timestamp: Date.now(),
          codeBlocks,
          compileResult: result,
        });

        // If compile succeeded and engine path is configured, start engine and run
        if (result.success) {
          const { llmConfig, engineRunning, setEngineRunning, setScreenshotUrl } = useAppStore.getState();
          if (llmConfig.engine_path) {
            try {
              if (!engineRunning) {
                await startEngine(llmConfig.engine_path, project.path);
                setEngineRunning(true);
                // Wait for engine to be ready
                await new Promise((r) => setTimeout(r, 3000));
              }
              // Send compile_and_run command via file
              await sendEngineCommand({ action: "compile_and_run" });
              console.log("[useChat] sent compile_and_run to engine");

              // Poll for result (up to 15 seconds)
              for (let i = 0; i < 30; i++) {
                await new Promise((r) => setTimeout(r, 500));
                const engineResult = await pollEngineResult();
                if (engineResult) {
                  console.log("[useChat] engine result:", engineResult);
                  const screenshotPath = engineResult.screenshot as string | undefined;
                  if (screenshotPath) {
                    // Use Tauri asset protocol to load local file
                    setScreenshotUrl("asset://localhost/" + encodeURIComponent(screenshotPath) + "?t=" + Date.now());
                  }
                  break;
                }
              }
            } catch (e) {
              console.error("[useChat] engine error:", e);
            }
          }
        }
      }
    } catch (err) {
      console.error("[useChat] error:", err);
      useAppStore.getState().appendToLastAssistant(
        `\n\n[错误: ${err instanceof Error ? err.message : String(err)}]`
      );
    } finally {
      unlisten();
      useAppStore.getState().setIsStreaming(false);
    }
  };

  return { sendMessage };
}

import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "../stores/appStore";
import { chatCompletion, extractCodeBlocks } from "../services/llm";
import { writeProjectFile, compileProject, createProject, startEngine, sendEngineCommand, waitForEngineResponse, getEngineDts } from "../services/engine";
import { homeDir } from "@tauri-apps/api/path";

const SYSTEM_PROMPT_PREFIX = `You are a game development assistant for LLM3dEngine.
The engine uses TypeScript compiled to JavaScript and executed via QuickJS.
Engine is a global object — use Engine.* directly as values (e.g. new Engine.Node3D()).
Do NOT use import statements. All engine APIs are available globally via the Engine namespace.
Only use APIs that exist in the type definitions below. Do NOT invent APIs.

IMPORTANT — Always follow this pattern:
1. Start with Engine.setupScene() to create camera, light, and environment.
2. Use Engine.createPrimitive(type, opts) to create meshes (box, sphere, cylinder, capsule, prism, triangle, plane, quad).
3. Use Engine.addToScene(node) to add nodes to the scene.
4. Use Engine.vec3(x,y,z) and Engine.color(r,g,b,a) to construct value types.
5. Use Engine.onProcess(fn) for per-frame animation logic.

Example:
\`\`\`typescript // scripts/main.ts
Engine.setupScene({
  background: Engine.color(0.15, 0.15, 0.2),
  camera: { position: Engine.vec3(0, 0, 4) }
});
const box = Engine.createPrimitive("box", {
  color: Engine.color(1, 0.3, 0.1),
  position: Engine.vec3(0, 0.5, 0)
});
Engine.addToScene(box);
Engine.onProcess((delta) => {
  box.rotate_y(delta * 2);
});
\`\`\`

When generating code, wrap it in a code block with the target filename:
\`\`\`typescript // scripts/main.ts
// code here
\`\`\`

Here are the available Engine APIs:
`;

let cachedSystemPrompt: string | null = null;

async function getSystemPrompt(): Promise<string> {
  if (cachedSystemPrompt) return cachedSystemPrompt;
  try {
    const dts = await getEngineDts();
    cachedSystemPrompt = SYSTEM_PROMPT_PREFIX + "\n```typescript\n" + dts + "\n```";
  } catch {
    cachedSystemPrompt = SYSTEM_PROMPT_PREFIX + "\n(engine type definitions unavailable)";
  }
  return cachedSystemPrompt;
}

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

      const systemPrompt = await getSystemPrompt();
      const fullResponse = await chatCompletion(
        store.llmConfig,
        apiMessages,
        systemPrompt
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

        let result = await compileProject(project.path);

        // Auto-retry: if compile fails, send errors back to LLM to fix (up to 3 attempts)
        const MAX_RETRIES = 3;
        let attempt = 0;
        while (!result.success && attempt < MAX_RETRIES) {
          attempt++;
          console.log(`[useChat] compile failed, auto-fix attempt ${attempt}/${MAX_RETRIES}`);

          useAppStore.getState().appendToLastAssistant(
            `\n\n编译失败，正在自动修复 (${attempt}/${MAX_RETRIES})...`
          );

          // Send errors back to LLM
          const fixMessages = [
            ...apiMessages,
            { role: "assistant", content: fullResponse },
            {
              role: "user",
              content: `编译失败，请修复以下错误。只输出修复后的完整代码，不要解释：\n${result.errors.join("\n")}\n\n完整编译输出：\n${result.output}`,
            },
          ];

          const fixResponse = await chatCompletion(
            store.llmConfig,
            fixMessages,
            systemPrompt
          );

          const fixBlocks = extractCodeBlocks(fixResponse);
          if (fixBlocks.length > 0) {
            for (const block of fixBlocks) {
              await writeProjectFile(project.path, block.filename, block.code);
            }
            result = await compileProject(project.path);
          } else {
            break; // LLM didn't return code blocks, stop retrying
          }
        }

        useAppStore.getState().addMessage({
          id: crypto.randomUUID(),
          role: "assistant",
          content: result.success
            ? "代码已编译成功，场景已更新。"
            : `编译失败（已尝试 ${attempt} 次自动修复）：\n${result.errors.join("\n")}`,
          timestamp: Date.now(),
          codeBlocks,
          compileResult: result,
        });

        // If compile succeeded and engine path is configured, start engine and run
        if (result.success) {
          const { llmConfig, engineRunning, setEngineRunning } = useAppStore.getState();
          if (llmConfig.engine_path) {
            try {
              if (!engineRunning) {
                await startEngine(llmConfig.engine_path, project.path);
                setEngineRunning(true);
              }
              const requestId = crypto.randomUUID();
              await sendEngineCommand({ action: "compile_and_run", request_id: requestId });
              console.log("[useChat] sent compile_and_run to engine");
              const engineResult = await waitForEngineResponse(
                (payload) =>
                  payload.type === "compile_and_run" &&
                  payload.request_id === requestId,
                15000
              );
              console.log("[useChat] engine result:", engineResult);
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

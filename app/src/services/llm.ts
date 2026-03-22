import { invoke } from "@tauri-apps/api/core";
import type { LlmConfig, ChatMessage } from "../types";

export async function chatCompletion(
  config: LlmConfig,
  messages: Array<{ role: string; content: string }>,
  systemPrompt?: string
): Promise<string> {
  return invoke<string>("chat_completion", {
    config,
    messages,
    systemPrompt: systemPrompt ?? null,
  });
}

// Extract code blocks from LLM response
export function extractCodeBlocks(
  content: string
): Array<{ filename: string; language: string; code: string }> {
  const blocks: Array<{ filename: string; language: string; code: string }> =
    [];
  const regex = /```(\w+)?(?:\s*\/\/\s*(\S+))?\n([\s\S]*?)```/g;
  let match;
  while ((match = regex.exec(content)) !== null) {
    blocks.push({
      language: match[1] || "typescript",
      filename: match[2] || `script_${blocks.length}.ts`,
      code: match[3].trim(),
    });
  }
  return blocks;
}

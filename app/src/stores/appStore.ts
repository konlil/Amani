import { create } from "zustand";
import { persist } from "zustand/middleware";
import type {
  ChatMessage,
  InfoMode,
  RunMode,
  ProjectInfo,
  LlmConfig,
} from "../types";

interface AppState {
  // Messages
  messages: ChatMessage[];
  addMessage: (msg: ChatMessage) => void;
  appendToLastAssistant: (chunk: string) => void;
  clearMessages: () => void;

  // Modes
  infoMode: InfoMode;
  runMode: RunMode;
  setInfoMode: (mode: InfoMode) => void;
  setRunMode: (mode: RunMode) => void;

  // Project
  project: ProjectInfo | null;
  setProject: (project: ProjectInfo | null) => void;

  // LLM config
  llmConfig: LlmConfig;
  setLlmConfig: (config: LlmConfig) => void;

  // Engine
  engineRunning: boolean;
  setEngineRunning: (running: boolean) => void;

  // Preview
  screenshotUrl: string | null;
  setScreenshotUrl: (url: string | null) => void;

  // UI
  splitRatio: number;
  setSplitRatio: (ratio: number) => void;
  isStreaming: boolean;
  setIsStreaming: (streaming: boolean) => void;
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
  messages: [],
  addMessage: (msg) =>
    set((state) => ({ messages: [...state.messages, msg] })),
  appendToLastAssistant: (chunk) =>
    set((state) => {
      const msgs = [...state.messages];
      const last = msgs[msgs.length - 1];
      if (last && last.role === "assistant") {
        msgs[msgs.length - 1] = { ...last, content: last.content + chunk };
      }
      return { messages: msgs };
    }),
  clearMessages: () => set({ messages: [] }),

  infoMode: "advanced",
  runMode: "dev",
  setInfoMode: (mode) => set({ infoMode: mode }),
  setRunMode: (mode) => set({ runMode: mode }),

  project: null,
  setProject: (project) => set({ project }),

  llmConfig: {
    api_key: "",
    endpoint: "https://api.openai.com/v1",
    model: "gpt-4",
    engine_path: "",
  },
  setLlmConfig: (config) => set({ llmConfig: config }),

  engineRunning: false,
  setEngineRunning: (running) => set({ engineRunning: running }),

  screenshotUrl: null,
  setScreenshotUrl: (url) => set({ screenshotUrl: url }),

  splitRatio: 0.4,
  setSplitRatio: (ratio) => set({ splitRatio: ratio }),

  isStreaming: false,
  setIsStreaming: (streaming) => set({ isStreaming: streaming }),
}),
  {
    name: "llm3d-settings",
    partialize: (state) => ({
      llmConfig: state.llmConfig,
      infoMode: state.infoMode,
      splitRatio: state.splitRatio,
    }),
  }
));

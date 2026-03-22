export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: number;
  codeBlocks?: CodeBlock[];
  compileResult?: CompileResult;
}

export interface CodeBlock {
  filename: string;
  language: string;
  code: string;
}

export interface CompileResult {
  success: boolean;
  output: string;
  errors: string[];
}

export interface LlmConfig {
  api_key: string;
  endpoint: string;
  model: string;
}

export interface ProjectInfo {
  name: string;
  path: string;
}

export interface EngineStatus {
  running: boolean;
  project_path: string | null;
}

export type InfoMode = "normal" | "advanced";
export type RunMode = "dev" | "play";

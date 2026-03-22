import { invoke } from "@tauri-apps/api/core";
import type { EngineStatus, CompileResult, ProjectInfo } from "../types";

export async function startEngine(
  enginePath: string,
  gameProjectPath: string
): Promise<string> {
  return invoke<string>("start_engine", { enginePath, gameProjectPath });
}

export async function stopEngine(): Promise<string> {
  return invoke<string>("stop_engine");
}

export async function restartEngine(
  enginePath: string,
  gameProjectPath: string
): Promise<string> {
  return invoke<string>("restart_engine", { enginePath, gameProjectPath });
}

export async function sendEngineCommand(command: Record<string, unknown>): Promise<void> {
  return invoke("send_engine_command", { command });
}

export async function pollEngineResult(): Promise<Record<string, unknown> | null> {
  return invoke<Record<string, unknown> | null>("poll_engine_result");
}

export async function getEngineStatus(): Promise<EngineStatus> {
  return invoke<EngineStatus>("engine_status");
}

export async function createProject(
  parentDir: string,
  name: string
): Promise<ProjectInfo> {
  return invoke<ProjectInfo>("create_project", { parentDir, name });
}

export async function openProject(path: string): Promise<ProjectInfo> {
  return invoke<ProjectInfo>("open_project", { path });
}

export async function writeProjectFile(
  projectPath: string,
  relativePath: string,
  content: string
): Promise<void> {
  return invoke("write_project_file", { projectPath, relativePath, content });
}

export async function readProjectFile(
  projectPath: string,
  relativePath: string
): Promise<string> {
  return invoke<string>("read_project_file", { projectPath, relativePath });
}

export async function compileProject(
  projectPath: string
): Promise<CompileResult> {
  return invoke<CompileResult>("compile_project", { projectPath });
}

export async function getEngineDts(): Promise<string> {
  return invoke<string>("get_engine_dts");
}

export async function embedEngineWindow(): Promise<void> {
  return invoke("embed_engine_window");
}

export async function updateEngineBounds(
  x: number,
  y: number,
  width: number,
  height: number,
  scaleFactor: number
): Promise<void> {
  return invoke("update_engine_bounds", { x, y, width, height, scaleFactor });
}

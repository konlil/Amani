import { invoke } from "@tauri-apps/api/core";
import type { EngineStatus, CompileResult, ProjectInfo } from "../types";

export async function startEngine(
  enginePath: string,
  projectPath: string
): Promise<string> {
  return invoke<string>("start_engine", { enginePath, projectPath });
}

export async function stopEngine(): Promise<string> {
  return invoke<string>("stop_engine");
}

export async function restartEngine(
  enginePath: string,
  projectPath: string
): Promise<string> {
  return invoke<string>("restart_engine", { enginePath, projectPath });
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

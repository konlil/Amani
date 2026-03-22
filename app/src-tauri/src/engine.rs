use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;
use tauri::State;
use tokio::process::{Child, Command};

pub struct EngineProcess {
    pub child: Mutex<Option<Child>>,
    pub project_path: Mutex<Option<PathBuf>>,
}

impl Default for EngineProcess {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
            project_path: Mutex::new(None),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EngineStatus {
    pub running: bool,
    pub project_path: Option<String>,
}

#[tauri::command]
pub async fn start_engine(
    engine_path: String,
    project_path: String,
    state: State<'_, EngineProcess>,
) -> Result<String, String> {
    let mut child_lock = state.child.lock().map_err(|e| e.to_string())?;
    if child_lock.is_some() {
        return Err("Engine already running".into());
    }

    // Store project path
    let mut pp = state.project_path.lock().map_err(|e| e.to_string())?;
    *pp = Some(PathBuf::from(&project_path));

    // Launch Godot engine subprocess
    // The engine binary path should point to the custom-built Godot with QuickJS + coordinator
    let child = Command::new(&engine_path)
        .arg("--path")
        .arg(&project_path)
        .arg("--llm-coordinator")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start engine: {}", e))?;

    *child_lock = Some(child);
    Ok("Engine started".into())
}

#[tauri::command]
pub async fn stop_engine(state: State<'_, EngineProcess>) -> Result<String, String> {
    let child = {
        let mut child_lock = state.child.lock().map_err(|e| e.to_string())?;
        child_lock.take()
    };
    if let Some(mut child) = child {
        child.kill().await.map_err(|e| format!("Failed to kill engine: {}", e))?;
        Ok("Engine stopped".into())
    } else {
        Err("Engine not running".into())
    }
}

#[tauri::command]
pub async fn restart_engine(
    engine_path: String,
    project_path: String,
    state: State<'_, EngineProcess>,
) -> Result<String, String> {
    // Stop if running
    let old_child = {
        let mut child_lock = state.child.lock().map_err(|e| e.to_string())?;
        child_lock.take()
    };
    if let Some(mut child) = old_child {
        let _ = child.kill().await;
    }
    // Start again
    start_engine(engine_path, project_path, state).await
}

#[tauri::command]
pub async fn engine_status(state: State<'_, EngineProcess>) -> Result<EngineStatus, String> {
    let child_lock = state.child.lock().map_err(|e| e.to_string())?;
    let pp = state.project_path.lock().map_err(|e| e.to_string())?;
    Ok(EngineStatus {
        running: child_lock.is_some(),
        project_path: pp.as_ref().map(|p| p.to_string_lossy().to_string()),
    })
}

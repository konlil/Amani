use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use tauri::{Emitter, State, Window};
use tokio::io::AsyncBufReadExt;
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

fn bridge_project_path() -> PathBuf {
    let app_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    app_dir.join("engine-bridge")
}

#[tauri::command]
pub async fn start_engine(
    window: Window,
    engine_path: String,
    game_project_path: String,
    state: State<'_, EngineProcess>,
) -> Result<String, String> {
    {
        let child_lock = state.child.lock().map_err(|e| e.to_string())?;
        if child_lock.is_some() {
            return Err("Engine already running".into());
        }
    }

    {
        let mut pp = state.project_path.lock().map_err(|e| e.to_string())?;
        *pp = Some(PathBuf::from(&game_project_path));
    }

    let bridge_path = bridge_project_path();

    // Launch engine in watch mode
    let mut child = Command::new(&engine_path)
        .arg("--path")
        .arg(bridge_path.to_string_lossy().as_ref())
        .arg("--")
        .arg("--project-dir")
        .arg(&game_project_path)
        .arg("--watch")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start engine: {}", e))?;

    // Read stdout for JSON responses
    let stdout = child.stdout.take();
    if let Some(stdout) = stdout {
        let win = window.clone();
        tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(json) = serde_json::from_str::<Value>(&line) {
                    let _ = win.emit("engine-response", json);
                } else {
                    let _ = win.emit("engine-log", line);
                }
            }
            let _ = win.emit("engine-stopped", ());
        });
    }

    // Read stderr to prevent pipe buffer from filling up and blocking the engine
    let stderr = child.stderr.take();
    if let Some(stderr) = stderr {
        let win = window.clone();
        tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = win.emit("engine-log", line);
            }
        });
    }

    {
        let mut child_lock = state.child.lock().map_err(|e| e.to_string())?;
        *child_lock = Some(child);
    }

    Ok("Engine started".into())
}

#[tauri::command]
pub async fn stop_engine(state: State<'_, EngineProcess>) -> Result<String, String> {
    // Write quit command file
    let project_path = {
        let pp = state.project_path.lock().map_err(|e| e.to_string())?;
        pp.clone()
    };
    if let Some(ref pp) = project_path {
        let cmd_file = pp.join("command.json");
        let _ = tokio::fs::write(&cmd_file, r#"{"action":"quit"}"#).await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    let child = {
        let mut child_lock = state.child.lock().map_err(|e| e.to_string())?;
        child_lock.take()
    };

    if let Some(mut child) = child {
        let _ = child.kill().await;
        Ok("Engine stopped".into())
    } else {
        Err("Engine not running".into())
    }
}

#[tauri::command]
pub async fn restart_engine(
    window: Window,
    engine_path: String,
    game_project_path: String,
    state: State<'_, EngineProcess>,
) -> Result<String, String> {
    let old_child = {
        let mut child_lock = state.child.lock().map_err(|e| e.to_string())?;
        child_lock.take()
    };
    if let Some(mut child) = old_child {
        let _ = child.kill().await;
    }
    start_engine(window, engine_path, game_project_path, state).await
}

/// Send a command to the engine via file-based protocol
#[tauri::command]
pub async fn send_engine_command(
    command: Value,
    state: State<'_, EngineProcess>,
) -> Result<(), String> {
    let project_path = {
        let pp = state.project_path.lock().map_err(|e| e.to_string())?;
        pp.clone().ok_or("No project path set")?
    };

    // Clear previous result
    let result_file = project_path.join("result.json");
    let _ = tokio::fs::remove_file(&result_file).await;

    // Write command file
    let cmd_file = project_path.join("command.json");
    let json_str = serde_json::to_string(&command).map_err(|e| e.to_string())?;
    tokio::fs::write(&cmd_file, &json_str)
        .await
        .map_err(|e| format!("Failed to write command: {}", e))?;

    Ok(())
}

/// Poll for engine result file
#[tauri::command]
pub async fn poll_engine_result(
    state: State<'_, EngineProcess>,
) -> Result<Option<Value>, String> {
    let project_path = {
        let pp = state.project_path.lock().map_err(|e| e.to_string())?;
        pp.clone().ok_or("No project path set")?
    };

    let result_file = project_path.join("result.json");
    match tokio::fs::read_to_string(&result_file).await {
        Ok(content) => {
            let _ = tokio::fs::remove_file(&result_file).await;
            let json: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
            Ok(Some(json))
        }
        Err(_) => Ok(None),
    }
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

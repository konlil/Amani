use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use tauri::{Emitter, State, Window};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

pub struct EngineProcess {
    pub child: Mutex<Option<Child>>,
    pub project_path: Mutex<Option<PathBuf>>,
    pub stdin_tx: Mutex<Option<mpsc::Sender<String>>>,
}

impl Default for EngineProcess {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
            project_path: Mutex::new(None),
            stdin_tx: Mutex::new(None),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EngineStatus {
    pub running: bool,
    pub project_path: Option<String>,
}

/// Resolve the engine bridge project path (bundled with the app)
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

    // Store project path
    {
        let mut pp = state.project_path.lock().map_err(|e| e.to_string())?;
        *pp = Some(PathBuf::from(&game_project_path));
    }

    let bridge_path = bridge_project_path();

    // Launch Godot engine with the bridge project
    let mut child = Command::new(&engine_path)
        .arg("--path")
        .arg(bridge_path.to_string_lossy().as_ref())
        .arg("--")
        .arg("--project-dir")
        .arg(&game_project_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start engine: {}", e))?;

    // Take ownership of stdin/stdout
    let stdin = child.stdin.take().ok_or("Failed to get engine stdin")?;
    let stdout = child.stdout.take().ok_or("Failed to get engine stdout")?;

    // Channel for sending commands to stdin writer task
    let (tx, mut rx) = mpsc::channel::<String>(32);

    // Stdin writer task
    tokio::spawn(async move {
        let mut stdin = stdin;
        while let Some(cmd) = rx.recv().await {
            if let Err(e) = stdin.write_all(cmd.as_bytes()).await {
                eprintln!("[engine] stdin write error: {}", e);
                break;
            }
            if let Err(e) = stdin.write_all(b"\n").await {
                eprintln!("[engine] stdin newline error: {}", e);
                break;
            }
            let _ = stdin.flush().await;
        }
    });

    // Stdout reader task — forward engine responses as Tauri events
    let win = window.clone();
    tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            // Try to parse as JSON
            if let Ok(json) = serde_json::from_str::<Value>(&line) {
                let _ = win.emit("engine-response", json);
            } else {
                // Non-JSON output (debug prints etc)
                let _ = win.emit("engine-log", line);
            }
        }
        // Engine process ended
        let _ = win.emit("engine-stopped", ());
    });

    // Store child and stdin sender
    {
        let mut child_lock = state.child.lock().map_err(|e| e.to_string())?;
        *child_lock = Some(child);
    }
    {
        let mut tx_lock = state.stdin_tx.lock().map_err(|e| e.to_string())?;
        *tx_lock = Some(tx);
    }

    Ok("Engine started".into())
}

#[tauri::command]
pub async fn stop_engine(state: State<'_, EngineProcess>) -> Result<String, String> {
    // Send quit command first
    let tx_clone = {
        let tx_lock = state.stdin_tx.lock().map_err(|e| e.to_string())?;
        tx_lock.clone()
    };
    if let Some(tx) = tx_clone {
        let _ = tx.send(r#"{"action":"quit"}"#.to_string()).await;
    }

    // Give it a moment, then force kill
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let child = {
        let mut child_lock = state.child.lock().map_err(|e| e.to_string())?;
        child_lock.take()
    };
    {
        let mut tx_lock = state.stdin_tx.lock().map_err(|e| e.to_string())?;
        *tx_lock = None;
    }

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
    // Stop if running
    let old_child = {
        let mut child_lock = state.child.lock().map_err(|e| e.to_string())?;
        child_lock.take()
    };
    {
        let mut tx_lock = state.stdin_tx.lock().map_err(|e| e.to_string())?;
        *tx_lock = None;
    }
    if let Some(mut child) = old_child {
        let _ = child.kill().await;
    }
    start_engine(window, engine_path, game_project_path, state).await
}

#[tauri::command]
pub async fn send_engine_command(
    command: Value,
    state: State<'_, EngineProcess>,
) -> Result<(), String> {
    let tx_lock = state.stdin_tx.lock().map_err(|e| e.to_string())?;
    let tx = tx_lock.as_ref().ok_or("Engine not running")?;
    let json_str = serde_json::to_string(&command).map_err(|e| e.to_string())?;
    tx.try_send(json_str).map_err(|e| format!("Failed to send command: {}", e))
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

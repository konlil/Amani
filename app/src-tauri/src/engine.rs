use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, State, Window};
use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, Command};

#[cfg(target_os = "macos")]
use crate::window_embed;

pub struct EngineProcess {
    pub child: Mutex<Option<Child>>,
    pub project_path: Mutex<Option<PathBuf>>,
    pub window_handle: Arc<Mutex<Option<u64>>>,
}

impl Default for EngineProcess {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
            project_path: Mutex::new(None),
            window_handle: Arc::new(Mutex::new(None)),
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

    // Clear previous window handle
    {
        let mut wh = state.window_handle.lock().map_err(|e| e.to_string())?;
        *wh = None;
    }

    // Launch engine in watch mode, positioned offscreen to avoid flicker
    let mut child = Command::new(&engine_path)
        .arg("--path")
        .arg(bridge_path.to_string_lossy().as_ref())
        .arg("--position")
        .arg("-10000,-10000")
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
        let wh = Arc::clone(&state.window_handle);
        tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(json) = serde_json::from_str::<Value>(&line) {
                    // Capture window handle reported by bridge.gd
                    // GDScript prints the NSWindow pointer as a signed int,
                    // so parse as i64 first then cast to u64.
                    if json.get("type").and_then(|t| t.as_str()) == Some("window_handle") {
                        let handle = json.get("handle")
                            .and_then(|h| h.as_u64().or_else(|| h.as_i64().map(|v| v as u64)));
                        if let Some(handle) = handle {
                            if let Ok(mut lock) = wh.lock() {
                                *lock = Some(handle);
                            }
                            let _ = win.emit("engine-window-handle", handle);
                        }
                    }
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
pub async fn stop_engine(
    _window: Window,
    state: State<'_, EngineProcess>,
) -> Result<String, String> {
    // Detach file-based overlay
    #[cfg(target_os = "macos")]
    {
        let _ = window_embed::detach_engine();
    }

    // Clear window handle
    {
        let mut wh = state.window_handle.lock().map_err(|e| e.to_string())?;
        *wh = None;
    }

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

/// Set up file-based window positioning bridge.
/// Waits for Godot to report its window handle (confirming it's alive),
/// then initializes the overlay state with the project path.
#[tauri::command]
pub async fn embed_engine_window(
    window: Window,
    state: State<'_, EngineProcess>,
) -> Result<(), String> {
    let wh_arc = Arc::clone(&state.window_handle);
    let project_path = {
        let pp = state.project_path.lock().map_err(|e| e.to_string())?;
        pp.clone()
    };

    // Wait for Godot to be ready (it writes window_handle file on startup)
    for _ in 0..50 {
        if let Ok(lock) = wh_arc.lock() {
            if lock.is_some() {
                break;
            }
        }
        if let Some(ref pp) = project_path {
            let handle_file = pp.join("window_handle");
            if let Ok(content) = tokio::fs::read_to_string(&handle_file).await {
                if let Ok(h) = content.trim().parse::<i64>() {
                    let h = h as u64;
                    if let Ok(mut lock) = wh_arc.lock() {
                        *lock = Some(h);
                    }
                    let _ = tokio::fs::remove_file(&handle_file).await;
                    break;
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Confirm Godot is alive
    {
        let lock = wh_arc.lock().map_err(|e| e.to_string())?;
        if lock.is_none() {
            return Err("Timed out waiting for Godot window handle".into());
        }
    }

    let pp = project_path.ok_or("No project path set")?;

    #[cfg(target_os = "macos")]
    {
        let ns_ptr = window.ns_window().map_err(|e| e.to_string())? as usize;
        let (tx, rx) = tokio::sync::oneshot::channel();
        window
            .run_on_main_thread(move || {
                let result = window_embed::embed_engine(ns_ptr, pp);
                let _ = tx.send(result);
            })
            .map_err(|e| e.to_string())?;
        rx.await
            .map_err(|_| "main thread channel closed".to_string())??;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, pp);
        return Err("Window embedding is only supported on macOS".into());
    }

    Ok(())
}

/// Update the embedded engine window position/size to match the preview panel.
#[tauri::command]
pub async fn update_engine_bounds(
    window: Window,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale_factor: f64,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let ns_ptr = window.ns_window().map_err(|e| e.to_string())? as usize;
        let (tx, rx) = tokio::sync::oneshot::channel();
        window
            .run_on_main_thread(move || {
                let result =
                    window_embed::update_engine_frame(ns_ptr, x, y, width, height, scale_factor);
                let _ = tx.send(result);
            })
            .map_err(|e| e.to_string())?;
        rx.await
            .map_err(|_| "main thread channel closed".to_string())??;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, x, y, width, height, scale_factor);
        return Err("Window embedding is only supported on macOS".into());
    }

    Ok(())
}

/// Reposition the engine overlay using the last saved bounds.
/// Called when the Tauri window is dragged/moved.
#[tauri::command]
pub async fn reposition_engine_overlay(window: Window) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        window
            .run_on_main_thread(move || {
                let result = window_embed::reposition_to_last();
                let _ = tx.send(result);
            })
            .map_err(|e| e.to_string())?;
        rx.await
            .map_err(|_| "main thread channel closed".to_string())??;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
    }

    Ok(())
}

/// Show the engine overlay window (e.g. when the app regains focus).
#[tauri::command]
pub async fn show_engine_window(window: Window) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        window
            .run_on_main_thread(move || {
                let result = window_embed::show_engine();
                let _ = tx.send(result);
            })
            .map_err(|e| e.to_string())?;
        rx.await
            .map_err(|_| "main thread channel closed".to_string())??;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
    }

    Ok(())
}

/// Hide the engine overlay window (e.g. when the app loses focus).
#[tauri::command]
pub async fn hide_engine_window(_window: Window) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = window_embed::hide_engine();
    }

    Ok(())
}

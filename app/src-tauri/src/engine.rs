use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs::OpenOptions, io::Write};
use tauri::{Emitter, State, Window};
use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::UnboundedSender;

#[cfg(target_os = "macos")]
use crate::window_embed;
use crate::engine_ipc;

#[derive(Clone)]
pub struct EngineProcess {
    pub child: Arc<Mutex<Option<Child>>>,
    pub project_path: Arc<Mutex<Option<PathBuf>>>,
    pub window_handle: Arc<Mutex<Option<u64>>>,
    pub session_id: Arc<Mutex<Option<String>>>,
    pub command_tx: Arc<Mutex<Option<UnboundedSender<String>>>>,
}

impl Default for EngineProcess {
    fn default() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
            project_path: Arc::new(Mutex::new(None)),
            window_handle: Arc::new(Mutex::new(None)),
            session_id: Arc::new(Mutex::new(None)),
            command_tx: Arc::new(Mutex::new(None)),
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

fn log_engine(message: &str) {
    let path = std::env::temp_dir().join("llm3d_overlay_engine.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}

fn new_session_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("engine-session-{ts}")
}

fn clear_runtime_state(state: &EngineProcess) {
    if let Ok(mut lock) = state.window_handle.lock() {
        *lock = None;
    }
    if let Ok(mut lock) = state.command_tx.lock() {
        *lock = None;
    }
}

fn build_command_message(command: Value, state: &EngineProcess) -> Result<Value, String> {
    let session_id = state
        .session_id
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("No engine session is active")?;

    let mut request = match command {
        Value::Object(map) => map,
        _ => return Err("Engine command must be a JSON object".into()),
    };

    request
        .entry("request_id")
        .or_insert_with(|| Value::String(new_session_id()));
    request.insert("session_id".into(), Value::String(session_id));

    Ok(Value::Object(request))
}

fn send_overlay_command(action: &str, payload: Value, state: &EngineProcess) -> Result<(), String> {
    let mut obj = serde_json::Map::new();
    obj.insert("action".into(), Value::String(action.to_string()));
    if let Value::Object(map) = payload {
        for (key, value) in map {
            obj.insert(key, value);
        }
    }

    let msg = build_command_message(Value::Object(obj), state)?;
    engine_ipc::send_message(&state.command_tx, msg)
}

async fn wait_for_ipc_connection(state: &EngineProcess) -> Result<(), String> {
    for _ in 0..50 {
        let connected = state
            .command_tx
            .lock()
            .map_err(|e| e.to_string())?
            .is_some();
        if connected {
            return Ok(());
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    Err("Timed out waiting for engine IPC connection".into())
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

    clear_runtime_state(&state);

    let session_id = new_session_id();
    {
        let mut session_lock = state.session_id.lock().map_err(|e| e.to_string())?;
        *session_lock = Some(session_id.clone());
    }

    let ipc_port = engine_ipc::start_ipc_server(window.clone(), state.inner().clone()).await?;
    let bridge_path = bridge_project_path();
    log_engine(&format!(
        "start_engine project={} session={} ipc_port={}",
        game_project_path, session_id, ipc_port
    ));

    let mut child = Command::new(&engine_path)
        .arg("--path")
        .arg(bridge_path.to_string_lossy().as_ref())
        .arg("--position")
        .arg("-10000,-10000")
        .arg("--")
        .arg("--project-dir")
        .arg(&game_project_path)
        .arg("--watch")
        .arg("--ipc-host")
        .arg("127.0.0.1")
        .arg("--ipc-port")
        .arg(ipc_port.to_string())
        .arg("--session-id")
        .arg(&session_id)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start engine: {e}"))?;

    let stdout = child.stdout.take();
    if let Some(stdout) = stdout {
        let win = window.clone();
        let state_clone = state.inner().clone();
        tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = win.emit("engine-log", line);
            }

            if let Ok(mut child_lock) = state_clone.child.lock() {
                *child_lock = None;
            }
            clear_runtime_state(&state_clone);
            let _ = win.emit("engine-stopped", ());
        });
    }

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
    #[cfg(target_os = "macos")]
    {
        let _ = window_embed::detach_engine();
    }

    let child = {
        let mut child_lock = state.child.lock().map_err(|e| e.to_string())?;
        child_lock.take()
    };

    if let Some(mut child) = child {
        let _ = send_overlay_command("quit", json!({}), &state);
        let _ = tokio::time::timeout(tokio::time::Duration::from_millis(500), child.wait()).await;
        let _ = child.kill().await;
        clear_runtime_state(&state);
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

    clear_runtime_state(&state);
    start_engine(window, engine_path, game_project_path, state).await
}

#[tauri::command]
pub async fn send_engine_command(
    command: Value,
    state: State<'_, EngineProcess>,
) -> Result<(), String> {
    wait_for_ipc_connection(&state).await?;

    let request = build_command_message(command, &state)?;
    log_engine(&format!("send_engine_command payload={request}"));
    engine_ipc::send_message(&state.command_tx, request)
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

#[tauri::command]
pub async fn embed_engine_window(
    window: Window,
    state: State<'_, EngineProcess>,
) -> Result<(), String> {
    for _ in 0..50 {
        if let Ok(lock) = state.window_handle.lock() {
            if lock.is_some() {
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    {
        let lock = state.window_handle.lock().map_err(|e| e.to_string())?;
        if lock.is_none() {
            return Err("Timed out waiting for Godot window handle".into());
        }
    }

    #[cfg(target_os = "macos")]
    {
        let ns_ptr = window.ns_window().map_err(|e| e.to_string())? as usize;
        let (tx, rx) = tokio::sync::oneshot::channel();
        window
            .run_on_main_thread(move || {
                let result = window_embed::embed_engine(ns_ptr);
                let _ = tx.send(result);
            })
            .map_err(|e| e.to_string())?;
        rx.await
            .map_err(|_| "main thread channel closed".to_string())??;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        return Err("Window embedding is only supported on macOS".into());
    }

    Ok(())
}

#[tauri::command]
pub async fn update_engine_bounds(
    window: Window,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale_factor: f64,
    state: State<'_, EngineProcess>,
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

        if let Some(frame) = rx
            .await
            .map_err(|_| "main thread channel closed".to_string())??
        {
            log_engine(&format!(
                "update_engine_bounds frame x={} y={} w={} h={}",
                frame.x, frame.y, frame.width, frame.height
            ));
            send_overlay_command(
                "set_frame",
                json!({
                    "x": frame.x,
                    "y": frame.y,
                    "width": frame.width,
                    "height": frame.height,
                }),
                &state,
            )?;
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, x, y, width, height, scale_factor, state);
        return Err("Window embedding is only supported on macOS".into());
    }

    Ok(())
}

#[tauri::command]
pub async fn reposition_engine_overlay(
    window: Window,
    state: State<'_, EngineProcess>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        window
            .run_on_main_thread(move || {
                let result = window_embed::reposition_to_last();
                let _ = tx.send(result);
            })
            .map_err(|e| e.to_string())?;

        if let Some(frame) = rx
            .await
            .map_err(|_| "main thread channel closed".to_string())??
        {
            log_engine(&format!(
                "reposition_engine_overlay frame x={} y={} w={} h={}",
                frame.x, frame.y, frame.width, frame.height
            ));
            send_overlay_command(
                "set_frame",
                json!({
                    "x": frame.x,
                    "y": frame.y,
                    "width": frame.width,
                    "height": frame.height,
                }),
                &state,
            )?;
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, state);
    }

    Ok(())
}

#[tauri::command]
pub async fn show_engine_window(window: Window, state: State<'_, EngineProcess>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        window
            .run_on_main_thread(move || {
                let result = window_embed::reposition_to_last();
                let _ = tx.send(result);
            })
            .map_err(|e| e.to_string())?;

        if let Some(frame) = rx
            .await
            .map_err(|_| "main thread channel closed".to_string())??
        {
            log_engine(&format!(
                "show_engine_window frame x={} y={} w={} h={}",
                frame.x, frame.y, frame.width, frame.height
            ));
            send_overlay_command(
                "set_frame",
                json!({
                    "x": frame.x,
                    "y": frame.y,
                    "width": frame.width,
                    "height": frame.height,
                }),
                &state,
            )?;
        }

        send_overlay_command("show_window", json!({}), &state)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, state);
    }

    Ok(())
}

#[tauri::command]
pub async fn hide_engine_window(_window: Window, state: State<'_, EngineProcess>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        send_overlay_command("hide_window", json!({}), &state)?;
    }

    Ok(())
}

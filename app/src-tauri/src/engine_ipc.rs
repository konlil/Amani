use serde_json::Value;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Window};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use std::{fs::OpenOptions, io::Write};

use crate::engine::EngineProcess;

fn log_ipc(message: &str) {
    let path = std::env::temp_dir().join("llm3d_overlay_ipc.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}

fn session_matches(expected: &Arc<Mutex<Option<String>>>, json: &Value) -> bool {
    let message_session = json.get("session_id").and_then(Value::as_str);
    let expected_session = expected.lock().ok().and_then(|lock| lock.clone());

    match (expected_session.as_deref(), message_session) {
        (Some(expected), Some(actual)) => expected == actual,
        (Some(_), None) => false,
        _ => true,
    }
}

fn clear_ipc_sender(state: &EngineProcess) {
    if let Ok(mut lock) = state.command_tx.lock() {
        *lock = None;
    }
}

fn handle_incoming_message(window: &Window, state: &EngineProcess, json: Value) {
    if !session_matches(&state.session_id, &json) {
        return;
    }

    match json.get("type").and_then(Value::as_str) {
        Some("window_ready") => {
            let handle = json
                .get("handle")
                .and_then(|h| h.as_u64().or_else(|| h.as_i64().map(|v| v as u64)));
            if let Some(handle) = handle {
                if let Ok(mut lock) = state.window_handle.lock() {
                    *lock = Some(handle);
                }
                log_ipc(&format!("window_ready handle={handle}"));
                let _ = window.emit("engine-window-handle", handle);
            }
        }
        Some("log") => {
            if let Some(message) = json.get("message").and_then(Value::as_str) {
                let _ = window.emit("engine-log", message.to_string());
            }
        }
        _ => {}
    }

    let _ = window.emit("engine-response", json);
}

pub async fn start_ipc_server(
    window: Window,
    state: EngineProcess,
) -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind engine IPC listener: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to inspect engine IPC listener: {e}"))?
        .port();
    log_ipc(&format!("start_ipc_server port={port}"));

    tokio::spawn(async move {
        let accept_result = listener.accept().await;
        let (stream, _) = match accept_result {
            Ok(pair) => pair,
            Err(err) => {
                let _ = window.emit("engine-log", format!("IPC accept failed: {err}"));
                clear_ipc_sender(&state);
                return;
            }
        };
        log_ipc("ipc client connected");

        let (read_half, mut write_half) = stream.into_split();
        let (tx, mut rx) = unbounded_channel::<String>();

        if let Ok(mut lock) = state.command_tx.lock() {
            *lock = Some(tx);
        }

        let writer_window = window.clone();
        let writer_state = state.clone();
        tokio::spawn(async move {
            while let Some(line) = rx.recv().await {
                if write_half.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                if write_half.write_all(b"\n").await.is_err() {
                    break;
                }
                if write_half.flush().await.is_err() {
                    break;
                }
                log_ipc(&format!("send {line}"));
            }

            clear_ipc_sender(&writer_state);
            let _ = writer_window.emit("engine-log", "Engine IPC writer closed");
        });

        let mut reader = BufReader::new(read_half).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<Value>(&line) {
                Ok(json) => {
                    log_ipc(&format!("recv {}", json));
                    handle_incoming_message(&window, &state, json)
                }
                Err(_) => {
                    let _ = window.emit("engine-log", line);
                }
            }
        }

        clear_ipc_sender(&state);
        let _ = window.emit("engine-log", "Engine IPC reader closed");
    });

    Ok(port)
}

pub fn send_message(
    tx_lock: &Arc<Mutex<Option<UnboundedSender<String>>>>,
    message: Value,
) -> Result<(), String> {
    let tx = tx_lock
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("Engine IPC channel is not connected")?;

    tx.send(message.to_string())
        .map_err(|_| "Engine IPC send failed".to_string())
}

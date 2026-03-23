use std::sync::Mutex;
use std::{fs::OpenOptions, io::Write};

use objc::{sel, sel_impl};

#[derive(Clone, Copy, Debug)]
pub struct OverlayFrame {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

struct OverlayState {
    tauri_ns_window: usize,
    last_x: f64,
    last_y: f64,
    last_w: f64,
    last_h: f64,
}

static OVERLAY: Mutex<Option<OverlayState>> = Mutex::new(None);

fn log_overlay(message: &str) {
    let path = std::env::temp_dir().join("llm3d_overlay_rust.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}

pub fn embed_engine(tauri_ns_window: usize) -> Result<(), String> {
    let mut lock = OVERLAY.lock().map_err(|e| e.to_string())?;
    log_overlay(&format!("embed_engine ns_window={tauri_ns_window}"));
    *lock = Some(OverlayState {
        tauri_ns_window,
        last_x: 0.0,
        last_y: 0.0,
        last_w: 0.0,
        last_h: 0.0,
    });
    Ok(())
}

pub fn update_engine_frame(
    tauri_ns_window: usize,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    _scale_factor: f64,
) -> Result<Option<OverlayFrame>, String> {
    let mut lock = OVERLAY.lock().map_err(|e| e.to_string())?;
    let state = match lock.as_mut() {
        Some(s) => s,
        None => return Ok(None),
    };

    state.last_x = x;
    state.last_y = y;
    state.last_w = w;
    state.last_h = h;
    state.tauri_ns_window = tauri_ns_window;

    log_overlay(&format!(
        "update_engine_frame css x={x:.2} y={y:.2} w={w:.2} h={h:.2}"
    ));
    Ok(compute_frame(state))
}

pub fn reposition_to_last() -> Result<Option<OverlayFrame>, String> {
    let lock = OVERLAY.lock().map_err(|e| e.to_string())?;
    let state = match lock.as_ref() {
        Some(s) => s,
        None => return Ok(None),
    };

    Ok(compute_frame(state))
}

pub fn detach_engine() -> Result<(), String> {
    let mut lock = OVERLAY.lock().map_err(|e| e.to_string())?;
    log_overlay("detach_engine");
    *lock = None;
    Ok(())
}

#[allow(deprecated)]
fn compute_frame(state: &OverlayState) -> Option<OverlayFrame> {
    let ns_window = state.tauri_ns_window as cocoa::base::id;
    if ns_window.is_null() {
        return None;
    }

    unsafe {
        let parent_frame: cocoa::foundation::NSRect = objc::msg_send![ns_window, frame];
        let content_rect: cocoa::foundation::NSRect =
            objc::msg_send![ns_window, contentRectForFrameRect: parent_frame];

        let screen: cocoa::base::id = objc::msg_send![ns_window, screen];
        if screen.is_null() {
            return None;
        }

        let screen_frame: cocoa::foundation::NSRect = objc::msg_send![screen, frame];
        let screen_max_y = screen_frame.origin.y + screen_frame.size.height;

        let content_top_appkit = content_rect.origin.y + content_rect.size.height;
        let panel_top_appkit = content_top_appkit - state.last_y;

        let frame = OverlayFrame {
            x: (content_rect.origin.x + state.last_x).round() as i32,
            y: (screen_max_y - panel_top_appkit).round() as i32,
            width: state.last_w.round() as i32,
            height: state.last_h.round() as i32,
        };
        log_overlay(&format!(
            "compute_frame screen x={} y={} w={} h={}",
            frame.x, frame.y, frame.width, frame.height
        ));
        Some(frame)
    }
}

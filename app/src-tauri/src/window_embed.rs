use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use objc::{sel, sel_impl};

static LOG_SEQ: AtomicU64 = AtomicU64::new(0);

fn log(msg: &str) {
    let seq = LOG_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join("llm3d_embed.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "[{:04}] {}", seq, msg);
    }
}

struct OverlayState {
    project_path: PathBuf,
    /// Tauri NSWindow pointer — only used to read *our own* window frame.
    tauri_ns_window: usize,
    last_x: f64,
    last_y: f64,
    last_w: f64,
    last_h: f64,
}

static OVERLAY: Mutex<Option<OverlayState>> = Mutex::new(None);

/// Initialize the file-based overlay bridge.
/// `tauri_ns_window` is kept so we can read our own window's screen frame.
pub fn embed_engine(tauri_ns_window: usize, project_path: PathBuf) -> Result<(), String> {
    log(&format!(
        "embed_engine (file-based): project={}",
        project_path.display()
    ));

    let mut lock = OVERLAY.lock().map_err(|e| e.to_string())?;
    *lock = Some(OverlayState {
        project_path,
        tauri_ns_window,
        last_x: 0.0,
        last_y: 0.0,
        last_w: 0.0,
        last_h: 0.0,
    });

    log("embed_engine completed");
    Ok(())
}

/// Compute screen coordinates from CSS-relative bounds and write `window_frame` file.
/// `x, y, w, h` are CSS pixels from getBoundingClientRect (origin: top-left of content area).
///
/// Godot `DisplayServer.window_set_position` uses screen coords with top-left origin.
/// macOS AppKit uses bottom-left origin. We read the Tauri NSWindow frame (safe — same process)
/// and convert.
pub fn update_engine_frame(
    tauri_ns_window: usize,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    _scale_factor: f64,
) -> Result<(), String> {
    log(&format!(
        "update_engine_frame: x={}, y={}, w={}, h={}",
        x, y, w, h
    ));

    let mut lock = OVERLAY.lock().map_err(|e| e.to_string())?;
    let state = match lock.as_mut() {
        Some(s) => s,
        None => {
            log("update_engine_frame: no overlay yet, skipping");
            return Ok(());
        }
    };

    state.last_x = x;
    state.last_y = y;
    state.last_w = w;
    state.last_h = h;
    state.tauri_ns_window = tauri_ns_window;

    write_frame_file(state);
    Ok(())
}

/// Reposition using last saved CSS bounds but re-reading the current Tauri window frame.
pub fn reposition_to_last() -> Result<(), String> {
    let lock = OVERLAY.lock().map_err(|e| e.to_string())?;
    let state = match lock.as_ref() {
        Some(s) => s,
        None => return Ok(()),
    };

    write_frame_file(state);
    Ok(())
}

/// Show the Godot window via command.json.
pub fn show_engine() -> Result<(), String> {
    log("show_engine called");
    let lock = OVERLAY.lock().map_err(|e| e.to_string())?;
    let state = match lock.as_ref() {
        Some(s) => s,
        None => {
            log("show_engine: no overlay, skipping");
            return Ok(());
        }
    };
    write_command(&state.project_path, r#"{"action":"show_window"}"#);
    // Also re-write frame so Godot knows where to appear
    write_frame_file(state);
    Ok(())
}

/// Hide the Godot window via command.json.
pub fn hide_engine() -> Result<(), String> {
    log("hide_engine called");
    let lock = OVERLAY.lock().map_err(|e| e.to_string())?;
    let state = match lock.as_ref() {
        Some(s) => s,
        None => {
            log("hide_engine: no overlay, skipping");
            return Ok(());
        }
    };
    write_command(&state.project_path, r#"{"action":"hide_window"}"#);
    Ok(())
}

/// Clean up state.
pub fn detach_engine() -> Result<(), String> {
    log("detach_engine called");
    let mut lock = OVERLAY.lock().map_err(|e| e.to_string())?;
    if let Some(state) = lock.take() {
        // Clean up frame file
        let _ = std::fs::remove_file(state.project_path.join("window_frame"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Read the Tauri NSWindow frame via ObjC (safe — same process) and convert
/// CSS-relative coords to Godot screen coords (top-left origin).
#[allow(deprecated)]
fn write_frame_file(state: &OverlayState) {
    let ns_window = state.tauri_ns_window as cocoa::base::id;
    if ns_window.is_null() {
        log("write_frame_file: ns_window is null");
        return;
    }

    let (screen_x, screen_y) = unsafe {
        let parent_frame: cocoa::foundation::NSRect = objc::msg_send![ns_window, frame];
        let content_rect: cocoa::foundation::NSRect =
            objc::msg_send![ns_window, contentRectForFrameRect: parent_frame];

        // content_rect origin is bottom-left (AppKit).
        // Godot uses top-left origin. We need:
        //   godot_x = content_left + css_x
        //   godot_y = screen_height - (content_bottom + content_height - css_y)
        //           = screen_height - content_top + css_y - content_height ... no, simpler:
        //
        // In AppKit: content_rect.origin.y is distance from screen bottom to content bottom.
        // The top of the content area in AppKit coords:
        //   content_top_appkit = content_rect.origin.y + content_rect.size.height
        // The panel top in AppKit coords:
        //   panel_top_appkit = content_top_appkit - css_y
        // The panel bottom in AppKit coords:
        //   panel_bottom_appkit = panel_top_appkit - h
        //
        // To convert to top-left origin we need the main screen height.
        let screens: cocoa::base::id = objc::msg_send![objc::class!(NSScreen), screens];
        let main_screen: cocoa::base::id = objc::msg_send![screens, objectAtIndex: 0usize];
        let screen_frame: cocoa::foundation::NSRect = objc::msg_send![main_screen, frame];
        let screen_height = screen_frame.size.height;

        let content_top_appkit = content_rect.origin.y + content_rect.size.height;
        let panel_top_appkit = content_top_appkit - state.last_y;

        let godot_x = content_rect.origin.x + state.last_x;
        let godot_y = screen_height - panel_top_appkit;

        (godot_x, godot_y)
    };

    let frame_str = format!(
        "{},{},{},{}",
        screen_x as i32, screen_y as i32, state.last_w as i32, state.last_h as i32
    );

    let frame_file = state.project_path.join("window_frame");
    log(&format!("write_frame_file: {}", frame_str));

    // Atomic write: write to tmp then rename
    let tmp_file = state.project_path.join("window_frame.tmp");
    if let Ok(mut f) = std::fs::File::create(&tmp_file) {
        if f.write_all(frame_str.as_bytes()).is_ok() {
            let _ = std::fs::rename(&tmp_file, &frame_file);
        }
    }
}

fn write_command(project_path: &PathBuf, json: &str) {
    let cmd_file = project_path.join("command.json");
    log(&format!("write_command: {}", json));
    let tmp_file = project_path.join("command.json.tmp");
    if let Ok(mut f) = std::fs::File::create(&tmp_file) {
        if f.write_all(json.as_bytes()).is_ok() {
            let _ = std::fs::rename(&tmp_file, &cmd_file);
        }
    }
}

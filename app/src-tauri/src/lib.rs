mod engine;
mod llm;
mod project;
#[cfg(target_os = "macos")]
mod window_embed;

use engine::EngineProcess;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(EngineProcess::default())
        .invoke_handler(tauri::generate_handler![
            engine::start_engine,
            engine::stop_engine,
            engine::restart_engine,
            engine::engine_status,
            engine::send_engine_command,
            engine::poll_engine_result,
            engine::embed_engine_window,
            engine::update_engine_bounds,
            engine::reposition_engine_overlay,
            engine::show_engine_window,
            engine::hide_engine_window,
            llm::chat_completion,
            project::create_project,
            project::open_project,
            project::write_project_file,
            project::read_project_file,
            project::compile_project,
            project::get_engine_dts,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

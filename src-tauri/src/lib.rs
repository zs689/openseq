//! OpenSeq — Tauri application entry point.
//!
//! The interesting logic lives in the `logseq-core` crate; this crate is only
//! the adapter that turns it into a desktop application: managed state, the
//! `invoke` command surface and a filesystem watcher.

mod commands;
mod state;
mod watcher;

use std::sync::Mutex;

use state::AppState;
use tauri::Manager;

/// Build and run the desktop application.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::open_graph,
            commands::create_graph,
            commands::reload_graph,
            commands::current_graph,
            commands::list_pages,
            commands::get_page,
            commands::get_or_create_page,
            commands::save_page,
            commands::delete_page,
            commands::rename_page,
            commands::search,
            commands::backlinks,
            commands::graph_data,
            commands::get_journal,
            commands::ensure_block_id,
            commands::graph_root,
            commands::page_properties,
        ])
        .setup(|app| {
            // Re-open whatever graph was open last time, if it still exists.
            let config = state::load_config(app.handle());
            let last_graph = config.last_graph.clone();
            if let Some(path) = last_graph {
                let dir = std::path::PathBuf::from(&path);
                if dir.is_dir() {
                    if let Ok(graph) = logseq_core::Graph::open(&dir) {
                        let handle = app.handle().clone();
                        let state = app.state::<Mutex<AppState>>();
                        let mut guard = state.lock().unwrap();
                        guard.watcher = watcher::start(handle, graph.root()).ok();
                        guard.root = Some(graph.root().to_path_buf());
                        guard.graph = Some(graph);
                        guard.config = config;
                    }
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running OpenSeq");
}

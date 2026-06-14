mod embedding;
mod llm;
mod config;
mod server;
mod state;
mod db;
mod extractor;
mod indexer;
mod commands;

use tauri::Manager;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, Modifiers, Code};

use state::AppState;
use db::init_db;
use embedding::EmbeddingEngine;
use indexer::{setup_watcher, index_files_task};
use commands::{
    open_file, search, get_indexing_status, pick_folder, get_indexed_files,
    start_dragging, close_window, minimize_window, maximize_window, set_always_on_top, ask_rag, save_api_key, check_api_key_status,
    save_setting, get_setting
};

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app, shortcut, event| {
            if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed && shortcut.id() == 0 {
                let window = app.get_webview_window("main").unwrap();
                if window.is_visible().unwrap() { let _ = window.hide(); } else { let _ = window.show(); let _ = window.set_focus(); }
            }
        }).build())
        .setup(move |app| {
            let handle = app.handle();
            let _ = handle.global_shortcut().register(Shortcut::new(Some(Modifiers::ALT), Code::KeyS));
            let db_path = handle.path().app_data_dir().unwrap().join("vaultseek_cache.db");
            std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
            let engine = Arc::new(EmbeddingEngine::new(&handle.path().resource_dir().unwrap().join("resources/model.onnx"), &handle.path().resource_dir().unwrap().join("resources/tokenizer.json")).expect("Engine Error"));
            let conn = init_db(&db_path);
            let vector_index = crate::db::VectorIndex::new();
            if let Err(e) = vector_index.load_from_db(&conn) {
                eprintln!("Failed to load vector index: {}", e);
            }
            let saved_path: Option<String> = conn.query_row("SELECT value FROM config WHERE key = 'watch_path'", [], |r| r.get(0)).ok();
            let state = AppState { engine, progress: Arc::new(AtomicUsize::new(0)), total_to_index: Arc::new(AtomicUsize::new(0)), is_finished: Arc::new(Mutex::new(false)), db_path, watch_path: Arc::new(Mutex::new(saved_path.clone())), watcher: Arc::new(Mutex::new(None)), db_conn: Arc::new(std::sync::Mutex::new(conn)), debounce_notify: Arc::new(tokio::sync::Notify::new()), vector_index: Arc::new(vector_index) };
            app.manage(state);
            let managed_state = app.state::<AppState>();
            if let Some(path) = saved_path {
                let h = handle.clone();
                let sc = Arc::new(managed_state.clone_internal());
                tauri::async_runtime::spawn(async move { let _ = setup_watcher(h.clone(), sc.clone(), &path).await; index_files_task(h, path, sc).await; });
            }
            let window = app.get_webview_window("main").unwrap();
            window.show().unwrap();
            window.set_focus().unwrap();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![open_file, search, get_indexing_status, pick_folder, get_indexed_files, start_dragging, close_window, minimize_window, maximize_window, set_always_on_top, ask_rag, save_api_key, check_api_key_status, save_setting, get_setting])
        .run(tauri::generate_context!())
        .expect("error");
}

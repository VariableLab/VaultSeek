use std::sync::Arc;
use std::path::PathBuf;
use walkdir::WalkDir;
use notify::{Watcher, RecursiveMode, Config};
use tauri::{AppHandle, Emitter};
use std::sync::atomic::Ordering;
use rusqlite::params;
use crate::state::AppState;
use crate::extractor::{extract_text, chunk_text};

pub async fn setup_watcher(app: AppHandle, state: Arc<AppState>, path: &str) -> Result<(), String> {
    let mut watcher_lock = state.watcher.lock().await;
    *watcher_lock = None;
    let path_to_watch = PathBuf::from(path);
    let state_clone = state.clone();
    let watcher_res = notify::RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    state_clone.debounce_notify.notify_one();
                }
            }
        },
        Config::default(),
    ).map_err(|e| format!("Watcher creation failed: {}", e))?;
    let mut watcher = watcher_res;
    watcher.watch(&path_to_watch, RecursiveMode::Recursive).map_err(|e| format!("Watcher watch failed: {}", e))?;
    *watcher_lock = Some(watcher);

    let app_debounce = app.clone();
    let state_debounce = state.clone();
    let wp_debounce = path.to_string();
    tokio::spawn(async move {
        loop {
            state_debounce.debounce_notify.notified().await;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let wp = state_debounce.watch_path.lock().await.clone().unwrap_or(wp_debounce.clone());
            index_files_task(app_debounce.clone(), wp, state_debounce.clone()).await;
        }
    });

    Ok(())
}

pub async fn index_files_task(app_handle: AppHandle, watch_path: String, state: Arc<AppState>) {
    let entries: Vec<(PathBuf, u64)> = WalkDir::new(&watch_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let ext = e.path().extension().and_then(|s| s.to_str()).unwrap_or("");
            matches!(ext, "md" | "pdf" | "txt" | "docx" | "xlsx")
        })
        .map(|e| {
            let modified = e.metadata()
                .map(|m| m.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH))
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            (e.path().to_path_buf(), modified)
        })
        .collect();

    state.total_to_index.store(entries.len(), Ordering::SeqCst);
    
    // Process files sequentially but with async blocking for each file
    for (path, modified) in entries {
        let path_str = path.to_string_lossy().into_owned();
        let state_clone = state.clone();
        let engine = state.engine.clone();
        let progress = state.progress.clone();
        
        // Use spawn_blocking for the blocking file processing
        tokio::task::spawn_blocking(move || {
            if let Ok(conn) = state_clone.db_conn.lock() {
                let up_to_date = conn.query_row(
                    "SELECT 1 FROM files WHERE path = ? AND modified = ?",
                    params![&path_str, modified],
                    |_| Ok(true)
                ).unwrap_or(false);
                
                if !up_to_date {
                    let content = extract_text(&path);
                    let chunks = chunk_text(&content, 500, 50);
                    conn.execute_batch("BEGIN").ok();
                    let mut success = true;
                    if conn.execute("DELETE FROM chunks WHERE file_path = ?", params![&path_str]).is_err() {
                        success = false;
                    }
                    if conn.execute(
                        "INSERT OR REPLACE INTO files (path, name, modified) VALUES (?, ?, ?)",
                        params![&path_str, path.file_name().unwrap().to_string_lossy(), modified]
                    ).is_err() {
                        success = false;
                    }
                    for chunk in chunks {
                        if let Ok(vector) = engine.embed(&chunk) {
                            if let Ok(embedding_blob) = bincode::serialize(&vector) {
                                if conn.execute(
                                    "INSERT INTO chunks (id, file_path, content, embedding) VALUES (?, ?, ?, ?)",
                                    params![uuid::Uuid::new_v4().to_string(), path_str.clone(), chunk, embedding_blob]
                                ).is_err() {
                                    success = false;
                                }
                            }
                        }
                    }
                    if success {
                        conn.execute_batch("COMMIT").ok();
                    } else {
                        conn.execute_batch("ROLLBACK").ok();
                    }
                }
            }
            progress.fetch_add(1, Ordering::SeqCst);
        }).await.ok();
    }
    
    if let Ok(conn) = state.db_conn.lock() {
        if let Err(e) = state.vector_index.load_from_db(&conn) {
            eprintln!("Failed to reload vector index: {}", e);
        }
    }
    
    *state.is_finished.lock().await = true;
    let _ = app_handle.emit("indexing-finished", ());
}

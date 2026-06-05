mod embedding;

use tauri::{AppHandle, Manager, Emitter, WebviewWindow};
use walkdir::WalkDir;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::embedding::EmbeddingEngine;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::path::{Path, PathBuf};
use rusqlite::{params, Connection};
use tauri_plugin_dialog::DialogExt;
use notify::{Watcher, RecursiveMode, Config};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, Modifiers, Code};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChunkItem {
    id: String,
    file_path: String,
    file_name: String,
    content: String,
    score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileInfo {
    path: String,
    name: String,
    modified: u64,
}

struct AppState {
    engine: Arc<EmbeddingEngine>,
    progress: Arc<AtomicUsize>,
    total_to_index: Arc<AtomicUsize>,
    is_finished: Arc<Mutex<bool>>,
    db_path: PathBuf,
    watch_path: Arc<Mutex<Option<String>>>,
    watcher: Arc<Mutex<Option<notify::RecommendedWatcher>>>,
}

impl AppState {
    fn clone_internal(&self) -> Self {
        Self {
            engine: self.engine.clone(),
            progress: self.progress.clone(),
            total_to_index: self.total_to_index.clone(),
            is_finished: self.is_finished.clone(),
            db_path: self.db_path.clone(),
            watch_path: self.watch_path.clone(),
            watcher: self.watcher.clone(),
        }
    }
}

#[tauri::command]
fn start_dragging(window: WebviewWindow) {
    let _ = window.start_dragging();
}

#[tauri::command]
fn set_always_on_top(window: WebviewWindow, on_top: bool) {
    let _ = window.set_always_on_top(on_top);
}

#[tauri::command]
fn open_file(path: String) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
}

#[tauri::command]
async fn get_indexing_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let current = state.progress.load(Ordering::SeqCst);
    let total = state.total_to_index.load(Ordering::SeqCst);
    let is_finished = *state.is_finished.lock().await;
    let watch_path = state.watch_path.lock().await.clone();
    Ok(serde_json::json!({ "current": current, "total": total, "is_finished": is_finished, "watch_path": watch_path }))
}

#[tauri::command]
async fn get_indexed_files(state: tauri::State<'_, AppState>) -> Result<Vec<FileInfo>, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT path, name, modified FROM files ORDER BY modified DESC").unwrap();
    let rows = stmt.query_map([], |row| {
        Ok(FileInfo { path: row.get(0)?, name: row.get(1)?, modified: row.get(2)? })
    }).unwrap();
    let mut files = Vec::new();
    for row in rows { files.push(row.unwrap()); }
    Ok(files)
}

#[tauri::command]
fn pick_folder(app: AppHandle, state: tauri::State<'_, AppState>) {
    let state_inner = state.clone_internal();
    let app_handle = app.clone();
    app.dialog().file().pick_folder(move |folder| {
        if let Some(f) = folder {
            let path_str = f.to_string();
            let conn = Connection::open(&state_inner.db_path).unwrap();
            let _ = conn.execute("INSERT OR REPLACE INTO config (key, value) VALUES ('watch_path', ?)", params![&path_str]);
            {
                let mut wp = state_inner.watch_path.blocking_lock();
                *wp = Some(path_str.clone());
                state_inner.progress.store(0, Ordering::SeqCst);
                state_inner.total_to_index.store(0, Ordering::SeqCst);
                *state_inner.is_finished.blocking_lock() = false;
            }
            let ah = app_handle.clone();
            let sc = Arc::new(state_inner.clone_internal());
            let wp_val = path_str.clone();
            tauri::async_runtime::spawn(async move {
                setup_watcher(ah.clone(), sc.clone(), &wp_val).await;
                index_files_task(ah, wp_val, sc).await;
            });
        }
    });
}

async fn setup_watcher(app: AppHandle, state: Arc<AppState>, path: &str) {
    let mut watcher_lock = state.watcher.lock().await;
    *watcher_lock = None;
    let path_to_watch = PathBuf::from(path);
    let app_handle = app.clone();
    let state_clone = state.clone();
    let watcher_res = notify::RecommendedWatcher::new(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                let ah = app_handle.clone();
                let sc = state_clone.clone();
                let wp = sc.watch_path.blocking_lock().clone().unwrap_or_default();
                tauri::async_runtime::spawn(async move { index_files_task(ah, wp, sc).await; });
            }
        }
    }, Config::default());
    if let Ok(mut watcher) = watcher_res {
        let _ = watcher.watch(&path_to_watch, RecursiveMode::Recursive);
        *watcher_lock = Some(watcher);
    }
}

#[tauri::command]
async fn search(query: String, state: tauri::State<'_, AppState>) -> Result<Vec<ChunkItem>, String> {
    if query.trim().is_empty() { return Ok(Vec::new()); }
    let query_vector = state.engine.embed(&query)?;
    let query_lower = query.to_lowercase();
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT c.content, f.path, f.name, c.embedding FROM chunks c JOIN files f ON c.file_path = f.path").unwrap();
    let rows = stmt.query_map([], |row| {
        let content: String = row.get(0)?;
        let path: String = row.get(1)?;
        let name: String = row.get(2)?;
        let embedding_blob: Vec<u8> = row.get(3)?;
        let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
        Ok((content, path, name, embedding))
    }).unwrap();

    let mut results = Vec::new();
    for row in rows {
        let (content, path, name, embedding) = row.unwrap();
        if embedding.is_empty() { continue; }
        let semantic_score: f32 = query_vector.iter().zip(embedding.iter()).map(|(x, y)| x * y).sum();
        let content_lower = content.to_lowercase();
        let keyword_match_content = content_lower.contains(&query_lower);
        let keyword_match_name = name.to_lowercase().contains(&query_lower);
        let mut final_score = semantic_score;
        if keyword_match_content { final_score += 0.4; }
        if keyword_match_name { final_score += 0.5; }
        if !keyword_match_content && !keyword_match_name && semantic_score < 0.45 { continue; }
        results.push(ChunkItem { id: uuid::Uuid::new_v4().to_string(), file_path: path, file_name: name, content, score: final_score });
    }
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    Ok(results.into_iter().take(20).collect())
}

fn init_db(path: &Path) -> Connection {
    let conn = Connection::open(path).expect("Failed to open database");
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS files (path TEXT PRIMARY KEY, name TEXT NOT NULL, modified INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS chunks (id TEXT PRIMARY KEY, file_path TEXT, content TEXT, embedding BLOB, FOREIGN KEY(file_path) REFERENCES files(path) ON DELETE CASCADE);
        CREATE TABLE IF NOT EXISTS config (key TEXT PRIMARY KEY, value TEXT);
    ").expect("Failed to init tables");
    conn
}

async fn index_files_task(app_handle: AppHandle, watch_path: String, state: Arc<AppState>) {
    use rayon::prelude::*;
    let entries: Vec<(PathBuf, u64)> = WalkDir::new(&watch_path).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()).filter(|e| {
            let ext = e.path().extension().and_then(|s| s.to_str()).unwrap_or("");
            matches!(ext, "md" | "pdf" | "txt" | "docx")
        }).map(|e| {
            let modified = e.metadata().unwrap().modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
            (e.path().to_path_buf(), modified)
        }).collect();

    state.total_to_index.store(entries.len(), Ordering::SeqCst);
    entries.into_par_iter().for_each(|(path, modified)| {
        let path_str = path.to_string_lossy().into_owned();
        let conn = Connection::open(&state.db_path).unwrap();
        let up_to_date = conn.query_row("SELECT 1 FROM files WHERE path = ? AND modified = ?", params![&path_str, modified], |_| Ok(true)).unwrap_or(false);
        if !up_to_date {
            let content = extract_text(&path);
            let chunks = chunk_text(&content, 500, 50);
            let _ = conn.execute("DELETE FROM chunks WHERE file_path = ?", params![&path_str]);
            let _ = conn.execute("INSERT OR REPLACE INTO files (path, name, modified) VALUES (?, ?, ?)", params![&path_str, path.file_name().unwrap().to_string_lossy(), modified]);
            for chunk in chunks {
                if let Ok(vector) = state.engine.embed(&chunk) {
                    let embedding_blob = bincode::serialize(&vector).unwrap();
                    let _ = conn.execute("INSERT INTO chunks (id, file_path, content, embedding) VALUES (?, ?, ?, ?)", params![uuid::Uuid::new_v4().to_string(), path_str, chunk, embedding_blob]);
                }
            }
        }
        state.progress.fetch_add(1, Ordering::SeqCst);
    });
    *state.is_finished.lock().await = true;
    let _ = app_handle.emit("indexing-finished", ());
}

fn extract_text(path: &Path) -> String {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext {
        "pdf" => std::panic::catch_unwind(|| pdf_extract::extract_text(path).unwrap_or_default()).unwrap_or_default(),
        "docx" => {
            std::panic::catch_unwind(|| {
                let file = std::fs::File::open(path).unwrap();
                let mut archive = zip::ZipArchive::new(file).unwrap();
                let mut document_xml = archive.by_name("word/document.xml").unwrap();
                let mut xml_content = String::new();
                std::io::Read::read_to_string(&mut document_xml, &mut xml_content).unwrap();
                let mut raw_text = String::new();
                let mut in_tag = false;
                for c in xml_content.chars() {
                    if c == '<' { in_tag = true; }
                    else if c == '>' { in_tag = false; }
                    else if !in_tag { raw_text.push(c); }
                }
                raw_text
            }).unwrap_or_default()
        },
        _ => std::fs::read_to_string(path).unwrap_or_default(),
    }
}

fn chunk_text(text: &str, size: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = (start + size).min(chars.len());
        chunks.push(chars[start..end].iter().collect());
        if end == chars.len() { break; }
        start += size - overlap;
    }
    chunks
}

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
            let saved_path: Option<String> = conn.query_row("SELECT value FROM config WHERE key = 'watch_path'", [], |r| r.get(0)).ok();
            let state = AppState { engine, progress: Arc::new(AtomicUsize::new(0)), total_to_index: Arc::new(AtomicUsize::new(0)), is_finished: Arc::new(Mutex::new(false)), db_path, watch_path: Arc::new(Mutex::new(saved_path.clone())), watcher: Arc::new(Mutex::new(None)) };
            app.manage(state);
            let managed_state = app.state::<AppState>();
            if let Some(path) = saved_path {
                let h = handle.clone();
                let sc = Arc::new(managed_state.clone_internal());
                tauri::async_runtime::spawn(async move { setup_watcher(h.clone(), sc.clone(), &path).await; index_files_task(h, path, sc).await; });
            }
            let window = app.get_webview_window("main").unwrap();
            window.show().unwrap();
            window.set_focus().unwrap();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![open_file, search, get_indexing_status, pick_folder, get_indexed_files, start_dragging, set_always_on_top])
        .run(tauri::generate_context!())
        .expect("error");
}

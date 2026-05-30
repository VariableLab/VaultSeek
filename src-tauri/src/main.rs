// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileItem {
    id: String,
    name: String,
    path: String,
    content_preview: String,
    #[serde(skip_serializing)]
    embedding: Vec<f32>,
    score: f32,
    modified: u64,
}

struct AppState {
    indexed_files: Arc<Mutex<Vec<FileItem>>>,
    engine: Arc<EmbeddingEngine>,
    progress: Arc<AtomicUsize>,
    total_to_index: Arc<AtomicUsize>,
    is_finished: Arc<Mutex<bool>>,
    db_path: PathBuf,
    watch_path: Arc<Mutex<Option<String>>>,
}

#[tauri::command]
fn start_dragging(window: WebviewWindow) {
    let _ = window.start_dragging();
}

#[tauri::command]
fn set_always_on_top(window: tauri::WebviewWindow, on_top: bool) {
    let _ = window.set_always_on_top(on_top);
}

#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

#[tauri::command]
fn open_file(path: String) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").arg("/c").arg("start").arg(path).spawn();
}

#[tauri::command]
async fn get_files(state: tauri::State<'_, AppState>) -> Result<Vec<FileItem>, String> {
    let files = state.indexed_files.lock().await;
    Ok(files.clone())
}

#[tauri::command]
async fn get_indexing_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let current = state.progress.load(Ordering::SeqCst);
    let total = state.total_to_index.load(Ordering::SeqCst);
    let is_finished = *state.is_finished.lock().await;
    let watch_path = state.watch_path.lock().await.clone();
    
    Ok(serde_json::json!({
        "current": current,
        "total": total,
        "is_finished": is_finished,
        "watch_path": watch_path
    }))
}

#[tauri::command]
async fn pick_folder(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    
    app.dialog().file().pick_folder(move |folder| {
        let _ = tx.send(folder);
    });

    let folder = rx.await.unwrap();
    
    if let Some(f) = folder {
        let path_str = f.to_string();
        
        // 1. 存入数据库
        let conn = open_db(&state.db_path);
        conn.execute("CREATE TABLE IF NOT EXISTS config (key TEXT PRIMARY KEY, value TEXT)", []).unwrap();
        conn.execute("INSERT OR REPLACE INTO config (key, value) VALUES ('watch_path', ?)", params![&path_str]).unwrap();
        
        // 2. 更新内存状态
        {
            let mut wp = state.watch_path.lock().await;
            *wp = Some(path_str.clone());
            
            // 重置索引状态
            state.progress.store(0, Ordering::SeqCst);
            state.total_to_index.store(0, Ordering::SeqCst);
            let mut finished = state.is_finished.lock().await;
            *finished = false;
        }

        // 3. 启动索引任务
        let app_handle = app.clone();
        let state_clone = state.inner().clone_state();
        let watch_path_clone = path_str.clone();
        tauri::async_runtime::spawn(async move {
            index_files_task(app_handle, watch_path_clone, state_clone).await;
        });

        return Ok(Some(path_str));
    }
    
    Ok(None)
}

// 辅助方法用于克隆状态引用
impl AppState {
    fn clone_state(&self) -> Arc<AppState> {
        Arc::new(AppState {
            indexed_files: self.indexed_files.clone(),
            engine: self.engine.clone(),
            progress: self.progress.clone(),
            total_to_index: self.total_to_index.clone(),
            is_finished: self.is_finished.clone(),
            db_path: self.db_path.clone(),
            watch_path: self.watch_path.clone(),
        })
    }
}

#[tauri::command]
async fn search(query: String, state: tauri::State<'_, AppState>) -> Result<Vec<FileItem>, String> {
    if query.trim().is_empty() { return Ok(Vec::new()); }
    let query_vector = state.engine.embed(&query)?;
    let files = state.indexed_files.lock().await;
    let mut results = Vec::new();

    for file in files.iter() {
        if file.embedding.is_empty() { continue; }
        let dot_product: f32 = query_vector.iter().zip(file.embedding.iter()).map(|(x, y)| x * y).sum();
        let name_match = file.name.to_lowercase().contains(&query.to_lowercase());
        
        if dot_product > 0.35 || name_match {
            let mut res = file.clone();
            res.score = if name_match { dot_product + 0.25 } else { dot_product };
            results.push(res);
        }
    }
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    Ok(results.into_iter().take(20).collect())
}

fn open_db(path: &Path) -> Connection {
    let conn = Connection::open(path).expect("Failed to open database");
    conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA busy_timeout = 5000;
    ").unwrap();
    conn
}

fn init_db(path: &Path) -> Connection {
    let conn = open_db(path);
    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            path TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            content_preview TEXT,
            embedding BLOB,
            modified INTEGER NOT NULL
        )",
        [],
    ).expect("Failed to create table");
    conn.execute("CREATE TABLE IF NOT EXISTS config (key TEXT PRIMARY KEY, value TEXT)", []).unwrap();
    conn
}

async fn index_files_task(app_handle: AppHandle, watch_path: String, state: Arc<AppState>) {
    use rayon::prelude::*;
    let _ = init_db(&state.db_path);
    
    let entries: Vec<(PathBuf, u64)> = WalkDir::new(&watch_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let ext = e.path().extension().and_then(|s| s.to_str()).unwrap_or("");
            ext == "md" || ext == "pdf" || ext == "txt" || ext == "docx"
        })
        .filter(|e| !e.path().to_string_lossy().contains("/."))
        .map(|e| {
            let meta = e.metadata().unwrap();
            let modified = meta.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
            (e.path().to_path_buf(), modified)
        })
        .collect();

    let total = entries.len();
    state.total_to_index.store(total, Ordering::SeqCst);
    
    if total == 0 {
        let mut finished = state.is_finished.lock().await;
        *finished = true;
        let _ = app_handle.emit("indexing-finished", ());
        return;
    }

    let results: Vec<FileItem> = entries.into_par_iter().map(|(path, modified)| {
        let path_str = path.to_string_lossy().into_owned();
        let cached: Option<FileItem> = {
            let conn = open_db(&state.db_path);
            let mut stmt = conn.prepare("SELECT name, content_preview, embedding, modified FROM files WHERE path = ?").unwrap();
            stmt.query_row([&path_str], |row| {
                let cached_modified: u64 = row.get(3).unwrap();
                if cached_modified == modified {
                    let embedding_blob: Vec<u8> = row.get(2).unwrap();
                    let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
                    Ok(Some(FileItem {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: row.get(0).unwrap(),
                        path: path_str.clone(),
                        content_preview: row.get(1).unwrap(),
                        embedding,
                        score: 0.0,
                        modified,
                    }))
                } else { Ok(None) }
            }).unwrap_or(None)
        };

        if let Some(item) = cached {
            state.progress.fetch_add(1, Ordering::SeqCst);
            return item;
        }

        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let content = if ext == "pdf" {
            std::panic::catch_unwind(|| {
                pdf_extract::extract_text(&path).unwrap_or_default()
            }).unwrap_or_default()
        } else if ext == "docx" {
            std::panic::catch_unwind(|| {
                // 读取 docx (本质是 zip 里的 word/document.xml)
                let file = std::fs::File::open(&path).unwrap();
                let mut archive = zip::ZipArchive::new(file).unwrap();
                let mut document_xml = archive.by_name("word/document.xml").unwrap();
                let mut xml_content = String::new();
                std::io::Read::read_to_string(&mut document_xml, &mut xml_content).unwrap();

                // 简单的 XML 标签剔除，只留纯文本
                let mut raw_text = String::new();
                let mut in_tag = false;
                for c in xml_content.chars() {
                    if c == '<' { in_tag = true; }
                    else if c == '>' { in_tag = false; }
                    else if !in_tag { raw_text.push(c); }
                }
                raw_text
            }).unwrap_or_default()
        } else {
            std::fs::read_to_string(&path).unwrap_or_default()
        };

        let preview = content.chars().take(200).collect::<String>();
        let model_input = content.chars().take(500).collect::<String>();
        let vector = state.engine.embed(&model_input).unwrap_or_default();

        let conn = open_db(&state.db_path);
        let embedding_blob = bincode::serialize(&vector).unwrap();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO files (path, name, content_preview, embedding, modified) VALUES (?, ?, ?, ?, ?)",
            params![path_str, path.file_name().unwrap().to_string_lossy(), preview, embedding_blob, modified],
        );

        state.progress.fetch_add(1, Ordering::SeqCst);
        FileItem {
            id: uuid::Uuid::new_v4().to_string(),
            name: path.file_name().unwrap().to_string_lossy().into_owned(),
            path: path_str, content_preview: preview, embedding: vector, score: 0.0, modified
        }
    }).collect();

    {
        let mut files = state.indexed_files.lock().await;
        *files = results;
        let mut finished = state.is_finished.lock().await;
        *finished = true;
    }
    let _ = app_handle.emit("indexing-finished", ());
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let app_handle = app.handle().clone();
            
            // 1. 使用 Tauri 原生 API 寻找绑定的资源
            // 在 macOS 的 .app 内部，它对应 Contents/Resources/resources/model.onnx
            let resource_dir = app_handle.path().resource_dir().expect("Failed to get resource dir");
            let model_path = resource_dir.join("resources").join("model.onnx");
            let tokenizer_path = resource_dir.join("resources").join("tokenizer.json");

            if !model_path.exists() {
                println!("====== 模型文件加载失败 ======");
                println!("尝试查找路径: {:?}", model_path);
                panic!("Missing AI models! Make sure they are bundled."); 
            }

            // 2. 数据库路径
            let app_data_dir = app_handle.path().app_data_dir().expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).unwrap();
            let db_path = app_data_dir.join("vaultseek_cache.db");

            let engine = Arc::new(EmbeddingEngine::new(&model_path, &tokenizer_path).expect("Engine Error"));
            
            // 从数据库加载保存的路径
            let mut saved_watch_path = None;
            if db_path.exists() {
                let conn = Connection::open(&db_path).unwrap();
                let _ = conn.execute("CREATE TABLE IF NOT EXISTS config (key TEXT PRIMARY KEY, value TEXT)", []);
                let mut stmt = conn.prepare("SELECT value FROM config WHERE key = 'watch_path'").unwrap();
                saved_watch_path = stmt.query_row([], |row| row.get(0)).ok();
            }

            let state = Arc::new(AppState {
                indexed_files: Arc::new(Mutex::new(Vec::new())),
                engine: engine.clone(),
                progress: Arc::new(AtomicUsize::new(0)),
                total_to_index: Arc::new(AtomicUsize::new(0)),
                is_finished: Arc::new(Mutex::new(false)),
                db_path: db_path.clone(),
                watch_path: Arc::new(Mutex::new(saved_watch_path.clone())),
            });

            let state_for_setup = state.clone();
            
            app.manage(AppState {
                indexed_files: state.indexed_files.clone(),
                engine: state.engine.clone(),
                progress: state.progress.clone(),
                total_to_index: state.total_to_index.clone(),
                is_finished: state.is_finished.clone(),
                db_path: state.db_path.clone(),
                watch_path: state.watch_path.clone(),
            });

            if let Some(path) = saved_watch_path {
                tauri::async_runtime::spawn(async move {
                    index_files_task(app_handle, path, state_for_setup).await;
                });
            }

            let window = app.get_webview_window("main").unwrap();
            window.center().unwrap();
            window.show().unwrap();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![hide_window, open_file, get_files, search, get_indexing_status, set_always_on_top, start_dragging, pick_folder])
        .run(tauri::generate_context!())
        .expect("error");
}

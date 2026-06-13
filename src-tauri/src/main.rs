mod embedding;
mod llm;
mod config;
mod server;

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
use calamine::{Reader, Xlsx, open_workbook};
use zip::ZipArchive;

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
fn start_dragging(window: WebviewWindow) { let _ = window.start_dragging(); }

#[tauri::command]
fn set_always_on_top(window: WebviewWindow, on_top: bool) { let _ = window.set_always_on_top(on_top); }

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
    let mut stmt = conn.prepare("SELECT path, name, modified FROM files ORDER BY modified DESC")
        .map_err(|e| format!("Prepare failed: {}", e))?;
    let rows = stmt.query_map([], |row| {
        Ok(FileInfo { path: row.get(0)?, name: row.get(1)?, modified: row.get(2)? })
    }).map_err(|e| format!("Query map failed: {}", e))?;
    let mut files = Vec::new();
    for row in rows {
        files.push(row.map_err(|e| format!("Row error: {}", e))?);
    }
    Ok(files)
}

#[tauri::command]
fn pick_folder(app: AppHandle, state: tauri::State<'_, AppState>) {
    let state_inner = state.clone_internal();
    let app_handle = app.clone();
    app.dialog().file().pick_folder(move |folder| {
        if let Some(f) = folder {
            let path_str = f.to_string();
            let conn = Connection::open(&state_inner.db_path).map_err(|e| eprintln!("DB open failed: {}", e)).ok();
            if let Some(conn) = conn {
                let _ = conn.execute("INSERT OR REPLACE INTO config (key, value) VALUES ('watch_path', ?)", params![&path_str]);
            }
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
                let _ = setup_watcher(ah.clone(), sc.clone(), &wp_val).await;
                index_files_task(ah, wp_val, sc).await;
            });
        }
    });
}

async fn setup_watcher(app: AppHandle, state: Arc<AppState>, path: &str) -> Result<(), String> {
    let mut watcher_lock = state.watcher.lock().await;
    *watcher_lock = None;
    let path_to_watch = PathBuf::from(path);
    let app_handle = app.clone();
    let state_clone = state.clone();
    let watcher_res = notify::RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    let ah = app_handle.clone();
                    let sc = state_clone.clone();
                    let wp = sc.watch_path.blocking_lock().clone().unwrap_or_default();
                    tauri::async_runtime::spawn(async move { index_files_task(ah, wp, sc).await; });
                }
            }
        },
        Config::default(),
    ).map_err(|e| format!("Watcher creation failed: {}", e))?;
    let mut watcher = watcher_res;
    watcher.watch(&path_to_watch, RecursiveMode::Recursive).map_err(|e| format!("Watcher watch failed: {}", e))?;
    *watcher_lock = Some(watcher);
    Ok(())
}

#[tauri::command]
async fn search(query: String, expanded_query: String, state: tauri::State<'_, AppState>) -> Result<Vec<ChunkItem>, String> {
    if query.trim().is_empty() { return Ok(Vec::new()); }
    
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    
    if query == "__SUMMARIZE_ALL__" {
        let mut stmt = conn.prepare("SELECT c.content, f.path, f.name, c.id FROM chunks c JOIN files f ON c.file_path = f.path ORDER BY f.modified DESC LIMIT 10")
            .map_err(|e| format!("Prepare failed: {}", e))?;
        let rows = stmt.query_map([], |row| {
            Ok(ChunkItem { id: row.get(3)?, file_path: row.get(1)?, file_name: row.get(2)?, content: row.get(0)?, score: 1.0 })
        }).map_err(|e| format!("Query map failed: {}", e))?;
        let mut results = Vec::new();
        for row in rows { results.push(row.map_err(|e| format!("Row error: {}", e))?); }
        return Ok(results);
    }

    let query_vector = state.engine.embed(&query)?;
    
    let keywords: Vec<String> = format!("{}, {}", query, expanded_query)
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    
    let mut stmt = conn.prepare("SELECT c.content, f.path, f.name, c.embedding, c.id FROM chunks c JOIN files f ON c.file_path = f.path")
        .map_err(|e| format!("Prepare failed: {}", e))?;
    
    let rows = stmt.query_map([], |row| {
        let content: String = row.get(0)?;
        let path: String = row.get(1)?;
        let name: String = row.get(2)?;
        let embedding_blob: Vec<u8> = row.get(3)?;
        let id: String = row.get(4)?;
        Ok((content, path, name, embedding_blob, id))
    }).map_err(|e| format!("Query map failed: {}", e))?;

    let collected_rows: Vec<_> = rows.filter_map(|r| r.ok()).collect();
    
    use rayon::prelude::*;
    let mut results: Vec<ChunkItem> = collected_rows.into_par_iter().filter_map(|(content, path, name, embedding_blob, id)| {
        let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
        if embedding.is_empty() { return None; }
        
        let semantic_score: f32 = query_vector.iter().zip(embedding.iter()).map(|(x, y)| x * y).sum();
        
        let content_lower = content.to_lowercase();
        let name_lower = name.to_lowercase();
        let keyword_hits = keywords.iter().filter(|k| content_lower.contains(*k) || name_lower.contains(*k)).count();
        
        let final_score = semantic_score + (keyword_hits as f32 * 0.2); 
        
        Some(ChunkItem { id, file_path: path, file_name: name, content, score: final_score })
    }).collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    
    if let Some(first) = results.first() {
        println!(">>> SEARCH: 检索到结果，最高分: {}", first.score);
    } else {
        println!(">>> SEARCH: 未检索到任何结果");
    }

    Ok(results.into_iter().take(20).collect())
}

#[tauri::command]
async fn ask_rag(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    query: String,
    selected_ids: Vec<String>,
) -> Result<Vec<ChunkItem>, String> {
    if query.trim().is_empty() { return Ok(Vec::new()); }

    let api_key_result = get_api_key(state.clone());
    let api_url = get_setting("api_url".to_string(), state.clone()).unwrap_or_else(|_| "https://apihub.agnes-ai.com/v1/chat/completions".to_string());
    let model = get_setting("model".to_string(), state.clone()).unwrap_or_else(|_| "moonshotai/kimi-k2.6".to_string());

    let expanded_query = if query == "__SUMMARIZE_ALL__" || api_key_result.is_err() {
        "".to_string()
    } else {
        llm::expand_query(api_key_result.as_ref().unwrap().clone(), model.clone(), api_url.clone(), query.clone()).await
            .unwrap_or_else(|_| "".to_string())
    };

    let mut results = if !selected_ids.is_empty() {
        // --- 核心复刻逻辑：来源锁定 ---
        let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
        let query_vector = state.engine.embed(&query)?;

        let id_placeholders: String = selected_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let mut stmt = conn.prepare(&format!(
            "SELECT c.content, f.path, f.name, c.embedding, c.id FROM chunks c JOIN files f ON c.file_path = f.path WHERE c.id IN ({})",
            id_placeholders
        )).map_err(|e| format!("Prepare failed: {}", e))?;

        let params: Vec<&dyn rusqlite::ToSql> = selected_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            let content: String = row.get(0)?;
            let path: String = row.get(1)?;
            let name: String = row.get(2)?;
            let embedding_blob: Vec<u8> = row.get(3)?;
            let id: String = row.get(4)?;
            Ok((content, path, name, embedding_blob, id))
        }).map_err(|e| format!("Query map failed: {}", e))?;

        let mut filtered_results = Vec::new();
        for row in rows {
            let (content, path, name, embedding_blob, id) = row.map_err(|e| format!("Row error: {}", e))?;
            let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
            let score: f32 = query_vector.iter().zip(embedding.iter()).map(|(x, y)| x * y).sum();
            filtered_results.push(ChunkItem { id, file_path: path, file_name: name, content, score });
        }
        filtered_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        filtered_results
    } else {
        search(query.clone(), expanded_query, state.clone()).await?
    };

    results.truncate(5);
    
    if results.is_empty() { return Err("知识库中未找到相关内容".to_string()); }

    let mut context_str = String::new();
    for (i, res) in results.iter().enumerate() {
        context_str.push_str(&format!("【片段 {}】(来自: {})\n{}\n\n", i + 1, res.file_name, res.content));
    }

    let system_prompt = r#"你是一个专业的“知识档案分析官”。你的任务是根据提供的【知识库片段】回答用户问题。

### 规则：
1. **事实优先**：只基于片段内容回答。如果片段中没有提到，请直白回答“根据现有本地档案，未找到相关记录”。
2. **结构化输出**：
   - 使用 `##` 标题划分模块。
   - 使用 `-` 或 `1.` 列表整理要点。
   - 关键术语、日期、数据请 **加粗**。
3. **术语对齐**：使用片段中出现的专业术语，不要自行发明。
4. **语言风格**：专业、严谨、客观。

请开始分析："#.to_string();

    let llm_query = if query == "__SUMMARIZE_ALL__" {
        "请对当前检索到的知识库资产进行全景式综述，提取核心主题、关键项目和重要结论。".to_string()
    } else {
        query.clone()
    };

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        match api_key_result {
            Ok(api_key) => {
                if let Err(e) = llm::stream_chat(app_handle.clone(), api_key, model, api_url, llm_query, context_str, system_prompt).await {
                    println!(">>> RAG: LLM 请求失败: {}", e);
                    let _ = app_handle.emit("chat-error", e);
                }
            },
            Err(_) => {
                let _ = app_handle.emit("chat-token", "**[本地检索模式]**\n\n知识库检索完成。如需 AI 总结，请设置 API Key。");
            }
        }
        let _ = app_handle.emit("chat-done", ());
    });
    
    Ok(results)
}

#[tauri::command]
fn save_api_key(key: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO config (key, value) VALUES ('api_key', ?)", params![&key]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_api_key(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    conn.query_row("SELECT value FROM config WHERE key = 'api_key'", [], |r| r.get(0)).map_err(|_| "No API Key found".to_string())
}

#[tauri::command]
fn save_setting(key: String, value: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO config (key, value) VALUES (?, ?)", params![&key, &value]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_setting(key: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let conn = Connection::open(&state.db_path).map_err(|e| e.to_string())?;
    conn.query_row("SELECT value FROM config WHERE key = ?", params![&key], |r| r.get(0)).map_err(|_| "Not found".to_string())
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
        let db_path = state.db_path.clone();
        let engine = state.engine.clone();
        let progress = state.progress.clone();
        
        // Use spawn_blocking for the blocking file processing
        tokio::task::spawn_blocking(move || {
            if let Ok(conn) = Connection::open(&db_path) {
                let up_to_date = conn.query_row(
                    "SELECT 1 FROM files WHERE path = ? AND modified = ?",
                    params![&path_str, modified],
                    |_| Ok(true)
                ).unwrap_or(false);
                
                if !up_to_date {
                    let content = extract_text(&path);
                    let chunks = chunk_text(&content, 500, 50);
                    let _ = conn.execute("DELETE FROM chunks WHERE file_path = ?", params![&path_str]);
                    let _ = conn.execute(
                        "INSERT OR REPLACE INTO files (path, name, modified) VALUES (?, ?, ?)",
                        params![&path_str, path.file_name().unwrap().to_string_lossy(), modified]
                    );
                    for chunk in chunks {
                        if let Ok(vector) = engine.embed(&chunk) {
                            if let Ok(embedding_blob) = bincode::serialize(&vector) {
                                let _ = conn.execute(
                                    "INSERT INTO chunks (id, file_path, content, embedding) VALUES (?, ?, ?, ?)",
                                    params![uuid::Uuid::new_v4().to_string(), path_str.clone(), chunk, embedding_blob]
                                );
                            }
                        }
                    }
                }
            }
            progress.fetch_add(1, Ordering::SeqCst);
        }).await.ok();
    }
    
    *state.is_finished.lock().await = true;
    let _ = app_handle.emit("indexing-finished", ());
}

fn extract_text(path: &Path) -> String {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext {
        "pdf" => std::panic::catch_unwind(|| pdf_extract::extract_text(path).unwrap_or_default()).unwrap_or_default(),
        "docx" => {
            std::panic::catch_unwind(|| {
                let file = std::fs::File::open(path).ok()?;
                let mut archive = ZipArchive::new(file).ok()?;
                let mut document_xml = archive.by_name("word/document.xml").ok()?;
                let mut xml_content = String::new();
                std::io::Read::read_to_string(&mut document_xml, &mut xml_content).ok()?;
                
                // 简单的 XML 标签过滤增强版：处理换行符和空格
                let mut text = String::new();
                let mut in_tag = false;
                let mut last_was_tag_end = false;
                for c in xml_content.chars() {
                    if c == '<' {
                        in_tag = true;
                    } else if c == '>' {
                        in_tag = false;
                        last_was_tag_end = true;
                    } else if !in_tag {
                        if last_was_tag_end && !text.is_empty() && !text.ends_with(' ') {
                             // 简单猜测段落间隔
                        }
                        text.push(c);
                        last_was_tag_end = false;
                    }
                }
                Some(text)
            }).unwrap_or_default().unwrap_or_default()
        },
        "xlsx" => {
            std::panic::catch_unwind(|| {
                let mut workbook: Xlsx<_> = open_workbook(path).unwrap();
                let mut markdown = String::new();
                if let Some(Ok(r)) = workbook.worksheet_range_at(0) {
                    for row in r.rows() {
                        markdown.push_str("| ");
                        for cell in row { markdown.push_str(&format!("{:?} | ", cell)); }
                        markdown.push_str("\n");
                    }
                }
                markdown
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
                tauri::async_runtime::spawn(async move { let _ = setup_watcher(h.clone(), sc.clone(), &path).await; index_files_task(h, path, sc).await; });
            }
            let window = app.get_webview_window("main").unwrap();
            window.show().unwrap();
            window.set_focus().unwrap();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![open_file, search, get_indexing_status, pick_folder, get_indexed_files, start_dragging, set_always_on_top, ask_rag, save_api_key, get_api_key, save_setting, get_setting])
        .run(tauri::generate_context!())
        .expect("error");
}

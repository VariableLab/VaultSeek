use tauri::{AppHandle, WebviewWindow, Emitter};
use crate::state::{AppState, ChunkItem, FileInfo, DEFAULT_API_URL};
use crate::db::core_search;
use crate::indexer::{setup_watcher, index_files_task};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri_plugin_dialog::DialogExt;
use rusqlite::params;
use std::path::PathBuf;
use crate::llm;

#[tauri::command]
pub fn start_dragging(window: WebviewWindow) { let _ = window.start_dragging(); }

#[tauri::command]
pub fn close_window(window: WebviewWindow) { let _ = window.close(); }

#[tauri::command]
pub fn minimize_window(window: WebviewWindow) { let _ = window.minimize(); }

#[tauri::command]
pub fn maximize_window(window: WebviewWindow) { 
    if let Ok(is_max) = window.is_maximized() {
        if is_max { let _ = window.unmaximize(); } 
        else { let _ = window.maximize(); }
    }
}

#[tauri::command]
pub fn set_always_on_top(window: WebviewWindow, on_top: bool) { let _ = window.set_always_on_top(on_top); }

#[tauri::command]
pub async fn open_file(path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    // C2: 路径验证 — 防止路径遍历攻击
    let canonical_path = std::fs::canonicalize(&path)
        .map_err(|e| format!("无法解析路径 '{}': {}", path, e))?;
    let watch_path = state.watch_path.lock().await;
    if let Some(ref wp) = *watch_path {
        let canonical_watch = std::fs::canonicalize(wp)
            .map_err(|e| format!("无法解析监控路径 '{}': {}", wp, e))?;
        if !canonical_path.starts_with(&canonical_watch) {
            return Err(format!("安全错误: 路径 '{}' 不在监控目录 '{}' 之下", path, wp));
        }
    } else {
        return Err("尚未设置监控目录，无法打开文件".to_string());
    }
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(&path).spawn();
    Ok(())
}

#[tauri::command]
pub async fn get_indexing_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let current = state.progress.load(Ordering::SeqCst);
    let total = state.total_to_index.load(Ordering::SeqCst);
    let is_finished = *state.is_finished.lock().await;
    let watch_path = state.watch_path.lock().await.clone();
    Ok(serde_json::json!({ "current": current, "total": total, "is_finished": is_finished, "watch_path": watch_path }))
}

#[tauri::command]
pub async fn get_indexed_files(state: tauri::State<'_, AppState>) -> Result<Vec<FileInfo>, String> {
    let conn = state.db_conn.lock().map_err(|e| e.to_string())?;
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
pub fn pick_folder(app: AppHandle, state: tauri::State<'_, AppState>) {
    let state_inner = state.clone_internal();
    let app_handle = app.clone();
    app.dialog().file().pick_folder(move |folder| {
        if let Some(f) = folder {
            let path_str = f.to_string();
            let conn = state_inner.db_conn.lock().map_err(|e| eprintln!("DB lock failed: {}", e)).ok();
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

#[tauri::command]
pub async fn search(query: String, expanded_query: String, state: tauri::State<'_, AppState>) -> Result<Vec<ChunkItem>, String> {
    if query.trim().is_empty() { return Ok(Vec::new()); }
    
    let conn = state.db_conn.lock().map_err(|e| e.to_string())?;
    
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

    let results = core_search(&conn, &state.engine, &state.vector_index, &query, &expanded_query, 20, None)?;
    
    if let Some(first) = results.first() {
        println!(">>> SEARCH: 检索到结果，最高分: {}", first.score);
    } else {
        println!(">>> SEARCH: 未检索到任何结果");
    }

    Ok(results)
}

#[tauri::command]
pub async fn ask_rag(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    query: String,
    selected_ids: Vec<String>,
    persona: Option<String>,
) -> Result<Vec<ChunkItem>, String> {
    if query.trim().is_empty() { return Ok(Vec::new()); }

    let api_key_result = get_api_key_internal(&state);
    let api_url = get_setting("api_url".to_string(), state.clone()).unwrap_or_else(|_| DEFAULT_API_URL.to_string());
    let model = get_setting("model".to_string(), state.clone()).unwrap_or_else(|_| "moonshotai/kimi-k2.6".to_string());

    let expanded_query = if query == "__SUMMARIZE_ALL__" || api_key_result.is_err() {
        "".to_string()
    } else {
        llm::expand_query(api_key_result.as_ref().unwrap().clone(), model.clone(), api_url.clone(), query.clone()).await
            .unwrap_or_else(|_| "".to_string())
    };

    let mut results = if !selected_ids.is_empty() {
        // --- 核心复刻逻辑：来源锁定 ---
        let conn = state.db_conn.lock().map_err(|e| e.to_string())?;
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
        filtered_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
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

    let persona_type = persona.unwrap_or_else(|| "default".to_string());
    let system_prompt = match persona_type.as_str() {
        "medical" => r#"你是一个专业的“医学审查专家”。你的任务是根据提供的【知识库片段】回答用户问题。
### 规则：
1. **事实提取**：首先列出本地文献中的核心医学事实。
2. **批判性审查**：主动指出文献中可能的局限性（如缺乏对照组、样本量小、副作用等）。基于医生的视角，提出超前的并发症预警或交互作用分析。
3. 如果未找到记录，明确回答未找到。使用专业医学术语。"#.to_string(),
        
        "legal" => r#"你是一个资深的“法务合规审查专家”。你的任务是根据提供的【知识库片段】回答问题。
### 规则：
1. **事实归纳**：提炼合同、条款或法案的核心信息。
2. **风险洞察**：敏锐地指出可能的法律风险、违约隐患或免责条款的漏洞。主动警示潜在合规问题。
3. 保持严谨客观的法律顾问风格。"#.to_string(),

        "coder" => r#"你是一个“资深系统架构师”。你的任务是根据提供的【代码库/技术文档片段】回答问题。
### 规则：
1. **技术解析**：快速总结代码逻辑或架构意图。
2. **架构审查**：指出潜在的性能瓶颈、安全漏洞、设计缺陷或重构建议。提供更高的架构视野。
3. 给出具体且优雅的代码改进建议。"#.to_string(),

        _ => r#"你是一个专业的“知识档案分析官”。你的任务是根据提供的【知识库片段】回答用户问题。
### 规则：
1. **事实优先**：只基于片段内容回答。如果片段中没有提到，请直白回答“根据现有本地档案，未找到相关记录”。
2. **结构化输出**：使用 `##` 标题划分模块，使用 `-` 列表整理要点。关键数据加粗。
3. 语言风格：专业、严谨、客观。"#.to_string(),
    };

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

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn get_api_key_path(state: &tauri::State<'_, AppState>) -> PathBuf {
    state.db_path.parent().unwrap().join(".api_key.sec")
}

#[tauri::command]
pub fn save_api_key(key: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let key_path = get_api_key_path(&state);
    std::fs::write(&key_path, &key).map_err(|e| format!("Failed to write key file: {}", e))?;
    #[cfg(unix)]
    {
        if let Ok(mut perms) = std::fs::metadata(&key_path).map(|m| m.permissions()) {
            perms.set_mode(0o600);
            let _ = std::fs::set_permissions(&key_path, perms);
        }
    }
    Ok(())
}

pub fn get_api_key_internal(state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let key_path = get_api_key_path(state);
    if key_path.exists() {
        let key = std::fs::read_to_string(&key_path)
            .map(|s| s.trim().to_string())
            .map_err(|_| "Failed to read API key file".to_string())?;
        return Ok(key);
    }

    // 向后兼容：从 SQLite 读取并迁移到文件
    let conn = state.db_conn.lock().map_err(|e| e.to_string())?;
    if let Ok(old_key) = conn.query_row("SELECT value FROM config WHERE key = 'api_key'", [], |r| r.get::<_, String>(0)) {
        let _ = std::fs::write(&key_path, &old_key);
        #[cfg(unix)]
        {
            if let Ok(mut perms) = std::fs::metadata(&key_path).map(|m| m.permissions()) {
                perms.set_mode(0o600);
                let _ = std::fs::set_permissions(&key_path, perms);
            }
        }
        let _ = conn.execute("DELETE FROM config WHERE key = 'api_key'", []);
        Ok(old_key)
    } else {
        Err("No API Key found".to_string())
    }
}

#[tauri::command]
pub fn check_api_key_status(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    Ok(get_api_key_internal(&state).is_ok())
}

#[tauri::command]
pub fn save_setting(key: String, value: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let conn = state.db_conn.lock().map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO config (key, value) VALUES (?, ?)", params![&key, &value]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_setting(key: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let conn = state.db_conn.lock().map_err(|e| e.to_string())?;
    conn.query_row("SELECT value FROM config WHERE key = ?", params![&key], |r| r.get(0)).map_err(|_| "Not found".to_string())
}

use axum::{
    extract::{State},
    http::StatusCode,
    response::{Json, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use crate::embedding::EmbeddingEngine;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub expanded_q: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<crate::ChunkItem>,
    pub total: usize,
    pub query: String,
}

#[derive(Deserialize)]
pub struct ChatQuery {
    pub query: String,
    pub selected_ids: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub sources: Vec<crate::ChunkItem>,
    pub context: String,
}

pub struct ServerState {
    pub app_state: Arc<AppState>,
    pub engine: Arc<EmbeddingEngine>,
    pub db_path: std::path::PathBuf,
}

impl Clone for ServerState {
    fn clone(&self) -> Self {
        Self {
            app_state: self.app_state.clone(),
            engine: self.engine.clone(),
            db_path: self.db_path.clone(),
        }
    }
}

pub fn create_router(app_state: Arc<AppState>) -> Router {
    let engine = app_state.engine.clone();
    let db_path = app_state.db_path.clone();
    
    let state = Arc::new(ServerState {
        app_state,
        engine,
        db_path,
    });

    Router::new()
        .route("/api/health", get(health_check))
        .route("/api/search", post(search_handler))
        .route("/api/chat", post(chat_handler))
        .route("/api/status", get(status_handler))
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "service": "vaultseek" }))
}

async fn status_handler(State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    let current = state.app_state.progress.load(std::sync::atomic::Ordering::SeqCst);
    let total = state.app_state.total_to_index.load(std::sync::atomic::Ordering::SeqCst);
    let is_finished = *state.app_state.is_finished.lock().await;
    let watch_path = state.app_state.watch_path.lock().await.clone();
    
    Json(serde_json::json!({
        "current": current,
        "total": total,
        "is_finished": is_finished,
        "watch_path": watch_path
    }))
}

async fn search_handler(
    State(state): State<Arc<ServerState>>,
    Json(query): Json<SearchQuery>,
) -> impl IntoResponse {
    if query.q.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Query cannot be empty" }))).into_response();
    }

    let expanded_query = query.expanded_q.unwrap_or_default();
    let limit = query.limit.unwrap_or(20).min(100);
    
    let conn = match rusqlite::Connection::open(&state.db_path) {
        Ok(conn) => conn,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Database error: {}", e) }))).into_response(),
    };

    let query_vector = match state.engine.embed(&query.q) {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Embedding error: {}", e) }))).into_response(),
    };

    let keywords: Vec<String> = format!("{}, {}", query.q, expanded_query)
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let mut stmt = match conn.prepare("SELECT c.content, f.path, f.name, c.embedding FROM chunks c JOIN files f ON c.file_path = f.path") {
        Ok(stmt) => stmt,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Prepare error: {}", e) }))).into_response(),
    };

    let rows = match stmt.query_map([], |row| {
        let content: String = row.get(0)?;
        let path: String = row.get(1)?;
        let name: String = row.get(2)?;
        let embedding_blob: Vec<u8> = row.get(3)?;
        Ok((content, path, name, embedding_blob))
    }) {
        Ok(rows) => rows,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Query error: {}", e) }))).into_response(),
    };

    let collected_rows: Vec<_> = rows.filter_map(|r| r.ok()).collect();

    use rayon::prelude::*;
    let mut results: Vec<crate::ChunkItem> = collected_rows.into_par_iter().filter_map(|(content, path, name, embedding_blob)| {
        let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
        if embedding.is_empty() { return None; }
        
        let semantic_score: f32 = query_vector.iter().zip(embedding.iter()).map(|(x, y)| x * y).sum();
        
        let content_lower = content.to_lowercase();
        let name_lower = name.to_lowercase();
        let keyword_hits = keywords.iter().filter(|k| content_lower.contains(*k) || name_lower.contains(*k)).count();
        
        let final_score = semantic_score + (keyword_hits as f32 * 0.2); 
        
        if final_score < 0.4 { return None; }
        Some(crate::ChunkItem { 
            id: uuid::Uuid::new_v4().to_string(), 
            file_path: path, 
            file_name: name, 
            content, 
            score: final_score 
        })
    }).collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    let total = results.len();
    results.truncate(limit);
    
    Json(SearchResponse {
        results,
        total,
        query: query.q,
    }).into_response()
}

async fn chat_handler(
    State(state): State<Arc<ServerState>>,
    Json(query): Json<ChatQuery>,
) -> impl IntoResponse {
    if query.query.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Query cannot be empty" }))).into_response();
    }

    let selected_ids = query.selected_ids.unwrap_or_default();
    let mut results = Vec::new();
    
    let conn = match rusqlite::Connection::open(&state.db_path) {
        Ok(conn) => conn,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Database error: {}", e) }))).into_response(),
    };

    let query_vector = match state.engine.embed(&query.query) {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Embedding error: {}", e) }))).into_response(),
    };

    if !selected_ids.is_empty() {
        let id_placeholders: String = selected_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let mut stmt = match conn.prepare(&format!(
            "SELECT c.content, f.path, f.name, c.embedding, c.id FROM chunks c JOIN files f ON c.file_path = f.path WHERE c.id IN ({})",
            id_placeholders
        )) {
            Ok(stmt) => stmt,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Prepare error: {}", e) }))).into_response(),
        };

        let params: Vec<&dyn rusqlite::ToSql> = selected_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = match stmt.query_map(rusqlite::params_from_iter(params), |row| {
            let content: String = row.get(0)?;
            let path: String = row.get(1)?;
            let name: String = row.get(2)?;
            let embedding_blob: Vec<u8> = row.get(3)?;
            let id: String = row.get(4)?;
            Ok((content, path, name, embedding_blob, id))
        }) {
            Ok(rows) => rows,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Query error: {}", e) }))).into_response(),
        };

        let mut filtered_results = Vec::new();
        for row in rows {
            if let Ok((content, path, name, embedding_blob, id)) = row {
                let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
                let score: f32 = query_vector.iter().zip(embedding.iter()).map(|(x, y)| x * y).sum();
                filtered_results.push(crate::ChunkItem { id, file_path: path, file_name: name, content, score });
            }
        }
        filtered_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results = filtered_results;
    } else {
        // Fallback to full search
        let mut stmt = match conn.prepare("SELECT c.content, f.path, f.name, c.embedding FROM chunks c JOIN files f ON c.file_path = f.path") {
            Ok(stmt) => stmt,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Prepare error: {}", e) }))).into_response(),
        };

        let rows = match stmt.query_map([], |row| {
            let content: String = row.get(0)?;
            let path: String = row.get(1)?;
            let name: String = row.get(2)?;
            let embedding_blob: Vec<u8> = row.get(3)?;
            Ok((content, path, name, embedding_blob))
        }) {
            Ok(rows) => rows,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Query error: {}", e) }))).into_response(),
        };

        let collected_rows: Vec<_> = rows.filter_map(|r| r.ok()).collect();

        use rayon::prelude::*;
        let mut full_results: Vec<crate::ChunkItem> = collected_rows.into_par_iter().filter_map(|(content, path, name, embedding_blob)| {
            let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
            if embedding.is_empty() { return None; }
            
            let semantic_score: f32 = query_vector.iter().zip(embedding.iter()).map(|(x, y)| x * y).sum();
            let final_score = semantic_score; 
            
            if final_score < 0.4 { return None; }
            Some(crate::ChunkItem { 
                id: uuid::Uuid::new_v4().to_string(), 
                file_path: path, 
                file_name: name, 
                content, 
                score: final_score 
            })
        }).collect();

        full_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results = full_results;
    }

    results.truncate(5);
    
    if results.is_empty() {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "知识库中未找到相关内容" }))).into_response();
    }

    let mut context_str = String::new();
    for (i, res) in results.iter().enumerate() {
        context_str.push_str(&format!("【片段 {}】(来自: {})\n{}\n\n", i + 1, res.file_name, res.content));
    }

    Json(ChatResponse {
        sources: results,
        context: context_str,
    }).into_response()
}

pub async fn start_server(app_state: Arc<AppState>, port: u16) -> Result<(), String> {
    let app = create_router(app_state);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| format!("Failed to bind port {}: {}", port, e))?;
    
    println!("VaultSeek HTTP server started on http://127.0.0.1:{}", port);
    
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))
}
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::path::PathBuf;
use rusqlite::Connection;
use crate::embedding::EmbeddingEngine;

pub const DEFAULT_API_URL: &str = "https://apihub.agnes-ai.com/v1/chat/completions";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkItem {
    pub id: String,
    pub file_path: String,
    pub file_name: String,
    pub content: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub modified: u64,
}

pub struct AppState {
    pub engine: Arc<EmbeddingEngine>,
    pub progress: Arc<AtomicUsize>,
    pub total_to_index: Arc<AtomicUsize>,
    pub is_finished: Arc<Mutex<bool>>,
    pub db_path: PathBuf,
    pub watch_path: Arc<Mutex<Option<String>>>,
    pub watcher: Arc<Mutex<Option<notify::RecommendedWatcher>>>,
    pub db_conn: Arc<std::sync::Mutex<Connection>>,
    pub debounce_notify: Arc<tokio::sync::Notify>,
    pub vector_index: Arc<crate::db::VectorIndex>,
}

impl AppState {
    pub fn clone_internal(&self) -> Self {
        Self {
            engine: self.engine.clone(),
            progress: self.progress.clone(),
            total_to_index: self.total_to_index.clone(),
            is_finished: self.is_finished.clone(),
            db_path: self.db_path.clone(),
            watch_path: self.watch_path.clone(),
            watcher: self.watcher.clone(),
            db_conn: self.db_conn.clone(),
            debounce_notify: self.debounce_notify.clone(),
            vector_index: self.vector_index.clone(),
        }
    }
}

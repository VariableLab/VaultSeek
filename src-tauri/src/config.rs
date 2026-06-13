use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub model: Option<String>,
    pub watch_path: Option<String>,
    pub embedding_model_path: Option<String>,
    pub tokenizer_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            api_url: Some("https://apihub.agnes-ai.com/v1/chat/completions".to_string()),
            model: Some("moonshotai/kimi-k2.6".to_string()),
            watch_path: None,
            embedding_model_path: None,
            tokenizer_path: None,
        }
    }
}

pub fn get_app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))
}

pub fn get_db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = get_app_data_dir(app)?;
    std::fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data dir: {}", e))?;
    Ok(data_dir.join("vaultseek_cache.db"))
}

pub fn get_model_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))
        .map(|p| p.join("resources"))
}

pub async fn load_config(state: &AppState) -> AppConfig {
    let conn = match rusqlite::Connection::open(&state.db_path) {
        Ok(conn) => conn,
        Err(_) => return AppConfig::default(),
    };

    let mut config = AppConfig::default();
    
    if let Ok(key) = conn.query_row("SELECT value FROM config WHERE key = 'api_key'", [], |r| r.get::<_, String>(0)) {
        config.api_key = Some(key);
    }
    
    if let Ok(url) = conn.query_row("SELECT value FROM config WHERE key = 'api_url'", [], |r| r.get::<_, String>(0)) {
        config.api_url = Some(url);
    }
    
    if let Ok(model) = conn.query_row("SELECT value FROM config WHERE key = 'model'", [], |r| r.get::<_, String>(0)) {
        config.model = Some(model);
    }
    
    if let Ok(path) = conn.query_row("SELECT value FROM config WHERE key = 'watch_path'", [], |r| r.get::<_, String>(0)) {
        config.watch_path = Some(path);
    }

    config
}

pub fn save_config_value(db_path: &std::path::Path, key: &str, value: &str) -> Result<(), String> {
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES (?, ?)",
        rusqlite::params![key, value],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
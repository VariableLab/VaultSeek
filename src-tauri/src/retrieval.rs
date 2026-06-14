use crate::ChunkItem;
use crate::embedding::EmbeddingEngine;
use rusqlite::Connection;

pub async fn search_library(db_path: &std::path::Path, engine: &EmbeddingEngine, query: &str) -> Result<Vec<ChunkItem>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    crate::core_search(&conn, engine, query, "", 20, Some(0.45))
}

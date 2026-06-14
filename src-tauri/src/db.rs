use rusqlite::Connection;
use crate::embedding::EmbeddingEngine;
use crate::state::ChunkItem;
use std::path::Path;
use parking_lot::RwLock;
use std::sync::Arc;
use rayon::prelude::*;

pub fn init_db(path: &Path) -> Connection {
    let conn = Connection::open(path).expect("Failed to open database");
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS files (path TEXT PRIMARY KEY, name TEXT NOT NULL, modified INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS chunks (id TEXT PRIMARY KEY, file_path TEXT, content TEXT, embedding BLOB, FOREIGN KEY(file_path) REFERENCES files(path) ON DELETE CASCADE);
        CREATE TABLE IF NOT EXISTS config (key TEXT PRIMARY KEY, value TEXT);
    ").expect("Failed to init tables");
    conn
}

pub struct VectorRecord {
    pub id: String,
    pub file_path: String,
    pub file_name: String,
    pub content_lower: String, // Used for fast keyword filtering in memory
    pub embedding: Vec<f32>,
}

pub struct VectorIndex {
    pub records: RwLock<Vec<VectorRecord>>,
}

impl VectorIndex {
    pub fn new() -> Self {
        Self { records: RwLock::new(Vec::new()) }
    }

    pub fn load_from_db(&self, conn: &Connection) -> Result<(), String> {
        let mut stmt = conn.prepare("SELECT c.id, f.path, f.name, c.content, c.embedding FROM chunks c JOIN files f ON c.file_path = f.path")
            .map_err(|e| format!("Prepare failed: {}", e))?;
        
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let file_path: String = row.get(1)?;
            let file_name: String = row.get(2)?;
            let content: String = row.get(3)?;
            let embedding_blob: Vec<u8> = row.get(4)?;
            Ok((id, file_path, file_name, content, embedding_blob))
        }).map_err(|e| format!("Query map failed: {}", e))?;

        let mut new_records = Vec::new();
        for row in rows {
            if let Ok((id, file_path, file_name, content, embedding_blob)) = row {
                let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
                if !embedding.is_empty() {
                    new_records.push(VectorRecord {
                        id,
                        file_path,
                        file_name,
                        content_lower: content.to_lowercase(),
                        embedding,
                    });
                }
            }
        }

        *self.records.write() = new_records;
        println!(">>> C5 Engine: Loaded {} vectors into memory index.", self.records.read().len());
        Ok(())
    }

    pub fn search(
        &self,
        conn: &Connection,
        query_vector: &[f32],
        keywords: &[String],
        limit: usize,
        min_score: Option<f32>,
    ) -> Result<Vec<ChunkItem>, String> {
        let threshold = min_score.unwrap_or(0.0);
        let records = self.records.read();
        
        let mut scored_records: Vec<(&VectorRecord, f32)> = records.par_iter().filter_map(|record| {
            let semantic_score: f32 = query_vector.iter().zip(record.embedding.iter()).map(|(x, y)| x * y).sum();
            let keyword_hits = keywords.iter().filter(|k| record.content_lower.contains(k.as_str()) || record.file_name.to_lowercase().contains(k.as_str())).count();
            let final_score = semantic_score + (keyword_hits as f32 * 0.2);
            
            if final_score < threshold { return None; }
            Some((record, final_score))
        }).collect();

        scored_records.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top_records: Vec<_> = scored_records.into_iter().take(limit).collect();
        
        if top_records.is_empty() { return Ok(Vec::new()); }

        let id_placeholders: String = top_records.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("SELECT id, content FROM chunks WHERE id IN ({})", id_placeholders);
        let mut stmt = conn.prepare(&query).map_err(|e| format!("Prepare failed: {}", e))?;
        
        let params: Vec<&dyn rusqlite::ToSql> = top_records.iter().map(|(r, _)| &r.id as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            Ok((id, content))
        }).map_err(|e| format!("Query map failed: {}", e))?;

        let mut content_map = std::collections::HashMap::new();
        for row in rows {
            if let Ok((id, content)) = row {
                content_map.insert(id, content);
            }
        }

        let mut final_results = Vec::new();
        for (record, score) in top_records {
            let content = content_map.remove(&record.id).unwrap_or_else(|| "".to_string());
            final_results.push(ChunkItem {
                id: record.id.clone(),
                file_path: record.file_path.clone(),
                file_name: record.file_name.clone(),
                content,
                score,
            });
        }

        Ok(final_results)
    }
}

// 保持对外的 core_search 接口兼容，但内部转发给 VectorIndex
pub fn core_search(
    conn: &Connection,
    engine: &EmbeddingEngine,
    index: &VectorIndex,
    query: &str,
    expanded_query: &str,
    limit: usize,
    min_score: Option<f32>,
) -> Result<Vec<ChunkItem>, String> {
    if query.trim().is_empty() { return Ok(Vec::new()); }
    
    let query_vector = engine.embed(query)?;
    let keywords: Vec<String> = format!("{}, {}", query, expanded_query)
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    
    index.search(conn, &query_vector, &keywords, limit, min_score)
}

use crate::types::ChunkItem;
use crate::embedding::EmbeddingEngine;
use rusqlite::Connection;

pub async fn search_library(db_path: &std::path::Path, engine: &EmbeddingEngine, query: &str) -> Result<Vec<ChunkItem>, String> {
    if query.trim().is_empty() { return Ok(Vec::new()); }
    let query_vector = engine.embed(query)?;
    let query_lower = query.to_lowercase();
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
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

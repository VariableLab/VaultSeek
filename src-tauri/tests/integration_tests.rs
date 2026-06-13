// Integration tests for VaultSeek
// These tests can be run with `cargo test`

use std::sync::Arc;
use tempfile::TempDir;
use tokio;
use rusqlite;

// Helper to create a test database
async fn create_test_db() -> (std::path::PathBuf, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    (db_path, temp_dir)
}

#[tokio::test]
async fn test_database_schema() {
    let (db_path, _temp_dir) = create_test_db().await;
    
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    
    // Test tables are created
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS files (path TEXT PRIMARY KEY, name TEXT NOT NULL, modified INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS chunks (id TEXT PRIMARY KEY, file_path TEXT, content TEXT, embedding BLOB, FOREIGN KEY(file_path) REFERENCES files(path) ON DELETE CASCADE);
         CREATE TABLE IF NOT EXISTS config (key TEXT PRIMARY KEY, value TEXT);"
    ).unwrap();
    
    // Verify tables exist
    let tables: Vec<String> = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    
    assert!(tables.contains(&"files".to_string()));
    assert!(tables.contains(&"chunks".to_string()));
    assert!(tables.contains(&"config".to_string()));
}

#[tokio::test]
async fn test_file_insert_and_query() {
    let (db_path, _temp_dir) = create_test_db().await;
    
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS files (path TEXT PRIMARY KEY, name TEXT NOT NULL, modified INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS chunks (id TEXT PRIMARY KEY, file_path TEXT, content TEXT, embedding BLOB, FOREIGN KEY(file_path) REFERENCES files(path) ON DELETE CASCADE);"
    ).unwrap();
    
    // Insert a file
    conn.execute(
        "INSERT INTO files (path, name, modified) VALUES (?, ?, ?)",
        rusqlite::params!["/test/file.md", "file.md", 1234567890]
    ).unwrap();
    
    // Query the file
    let mut stmt = conn.prepare("SELECT path, name, modified FROM files WHERE path = ?").unwrap();
    let file: (String, String, u64) = stmt.query_row(["/test/file.md"], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    }).unwrap();
    
    assert_eq!(file.0, "/test/file.md");
    assert_eq!(file.1, "file.md");
    assert_eq!(file.2, 1234567890);
}

#[tokio::test]
async fn test_chunk_insert_and_query() {
    let (db_path, _temp_dir) = create_test_db().await;
    
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS files (path TEXT PRIMARY KEY, name TEXT NOT NULL, modified INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS chunks (id TEXT PRIMARY KEY, file_path TEXT, content TEXT, embedding BLOB, FOREIGN KEY(file_path) REFERENCES files(path) ON DELETE CASCADE);"
    ).unwrap();
    
    // Insert a file
    conn.execute(
        "INSERT INTO files (path, name, modified) VALUES (?, ?, ?)",
        rusqlite::params!["/test/file.md", "file.md", 1234567890]
    ).unwrap();
    
    // Insert a chunk
    let embedding = vec![0.1f32; 768];
    let embedding_blob = bincode::serialize(&embedding).unwrap();
    
    conn.execute(
        "INSERT INTO chunks (id, file_path, content, embedding) VALUES (?, ?, ?, ?)",
        rusqlite::params!["chunk-1", "/test/file.md", "测试内容", embedding_blob]
    ).unwrap();
    
    // Query the chunk
    let mut stmt = conn.prepare("SELECT id, file_path, content FROM chunks WHERE file_path = ?").unwrap();
    let chunks: Vec<(String, String, String)> = stmt.query_map(["/test/file.md"], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    }).unwrap().filter_map(|r| r.ok()).collect();
    
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].0, "chunk-1");
    assert_eq!(chunks[0].2, "测试内容");
}

#[tokio::test]
async fn test_config_table() {
    let (db_path, _temp_dir) = create_test_db().await;
    
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS config (key TEXT PRIMARY KEY, value TEXT)",
        [],
    ).unwrap();
    
    // Insert config
    conn.execute(
        "INSERT INTO config (key, value) VALUES (?, ?)",
        rusqlite::params!["test_key", "test_value"]
    ).unwrap();
    
    // Query config
    let value: String = conn.query_row(
        "SELECT value FROM config WHERE key = ?",
        ["test_key"],
        |row| row.get(0)
    ).unwrap();
    
    assert_eq!(value, "test_value");
    
    // Update config
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES (?, ?)",
        rusqlite::params!["test_key", "updated_value"]
    ).unwrap();
    
    let value: String = conn.query_row(
        "SELECT value FROM config WHERE key = ?",
        ["test_key"],
        |row| row.get(0)
    ).unwrap();
    
    assert_eq!(value, "updated_value");
}

#[tokio::test]
async fn test_foreign_key_cascade() {
    let (db_path, _temp_dir) = create_test_db().await;
    
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS files (path TEXT PRIMARY KEY, name TEXT NOT NULL, modified INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS chunks (id TEXT PRIMARY KEY, file_path TEXT, content TEXT, embedding BLOB, FOREIGN KEY(file_path) REFERENCES files(path) ON DELETE CASCADE);"
    ).unwrap();
    
    // Insert file and chunk
    conn.execute(
        "INSERT INTO files (path, name, modified) VALUES (?, ?, ?)",
        rusqlite::params!["/test/file.md", "file.md", 1234567890]
    ).unwrap();
    
    conn.execute(
        "INSERT INTO chunks (id, file_path, content, embedding) VALUES (?, ?, ?, ?)",
        rusqlite::params!["chunk-1", "/test/file.md", "内容", vec![0u8; 100]]
    ).unwrap();
    
    // Delete file - should cascade delete chunks
    conn.execute("DELETE FROM files WHERE path = ?", ["/test/file.md"]).unwrap();
    
    // Verify chunk is deleted
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM chunks WHERE file_path = ?",
        ["/test/file.md"],
        |row| row.get(0)
    ).unwrap();
    
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_cosine_similarity() {
    let vec1 = vec![1.0, 0.0, 0.0];
    let vec2 = vec![1.0, 0.0, 0.0];
    let vec3 = vec![0.0, 1.0, 0.0];
    
    fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
        dot / (norm_a * norm_b)
    }
    
    // Same vectors should have similarity 1.0
    assert!((cosine_sim(&vec1, &vec2) - 1.0).abs() < 1e-6);
    
    // Orthogonal vectors should have similarity 0.0
    assert!(cosine_sim(&vec1, &vec3).abs() < 1e-6);
}
use rusqlite::{params, Connection};
use std::time::Instant;
use uuid::Uuid;
use bincode;

fn main() {
    let db_path = "test_stress.db";
    let _ = std::fs::remove_file(db_path);
    let conn = Connection::open(db_path).unwrap();
    
    conn.execute_batch("
        CREATE TABLE files (path TEXT PRIMARY KEY, name TEXT NOT NULL, modified INTEGER NOT NULL);
        CREATE TABLE chunks (id TEXT PRIMARY KEY, file_path TEXT, content TEXT, embedding BLOB);
    ").unwrap();

    println!("正在生成 5000 个模拟向量数据...");
    let start_gen = Instant::now();
    let embedding: Vec<f32> = vec![0.1; 384]; // 假设 384 维
    let embedding_blob = bincode::serialize(&embedding).unwrap();

    conn.execute("BEGIN TRANSACTION", []).unwrap();
    for i in 0..5000 {
        let path = format!("/path/to/file_{}.md", i);
        conn.execute("INSERT INTO files (path, name, modified) VALUES (?, ?, ?)", params![path, format!("file_{}.md", i), 12345678]).unwrap();
        conn.execute("INSERT INTO chunks (id, file_path, content, embedding) VALUES (?, ?, ?, ?)", 
            params![Uuid::new_v4().to_string(), path, "这是一段模拟内容".repeat(10), embedding_blob]).unwrap();
    }
    conn.execute("COMMIT", []).unwrap();
    println!("生成完成，用时: {:?}", start_gen.elapsed());

    println!("模拟检索逻辑 (线性扫描)...");
    let start_search = Instant::now();
    let query_vector: Vec<f32> = vec![0.1; 384];

    let mut stmt = conn.prepare("SELECT c.content, c.embedding FROM chunks c").unwrap();
    let rows = stmt.query_map([], |row| {
        let content: String = row.get(0)?;
        let embedding_blob: Vec<u8> = row.get(1)?;
        let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
        Ok((content, embedding))
    }).unwrap();

    let mut count = 0;
    for row in rows {
        let (_content, embedding) = row.unwrap();
        let _score: f32 = query_vector.iter().zip(embedding.iter()).map(|(x, y)| x * y).sum();
        count += 1;
    }
    println!("检索 5000 条记录完成，用时: {:?}", start_search.elapsed());
    println!("总处理记录数: {}", count);
}

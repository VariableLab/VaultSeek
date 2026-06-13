use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Value;
use tokenizers::Tokenizer;
use ndarray::{Array2};
use std::path::Path;
use std::sync::Mutex;

pub struct EmbeddingEngine {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
}

impl EmbeddingEngine {
    pub fn new(model_path: &Path, tokenizer_path: &Path) -> Result<Self, String> {
        // 1. 初始化 ONNX Runtime 会话
        let session = Session::builder()
            .map_err(|e| format!("Session builder error: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| format!("Optimization level error: {}", e))?
            .with_intra_threads(4) // 限制线程数，避免占用过多 CPU
            .map_err(|e| format!("Thread config error: {}", e))?
            .commit_from_file(model_path)
            .map_err(|e| format!("Model load error: {}", e))?;

        // 2. 加载分词器
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| format!("Tokenizer load error: {}", e))?;

        Ok(Self { 
            session: Mutex::new(session), 
            tokenizer 
        })
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        // 1. 文本分词
        let encoding = self.tokenizer.encode(text, true)
            .map_err(|e| format!("Encoding error: {}", e))?;
        
        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&mask| mask as i64).collect();
        let token_type_ids: Vec<i64> = encoding.get_type_ids().iter().map(|&id| id as i64).collect();

        let seq_len = input_ids.len();
        
        // 2. 准备输入 Tensor - 使用安全的转换避免 unwrap
        let input_ids_array = Array2::from_shape_vec((1, seq_len), input_ids)
            .map_err(|e| format!("Input IDs shape error: {}", e))?;
        let attention_mask_array = Array2::from_shape_vec((1, seq_len), attention_mask)
            .map_err(|e| format!("Attention mask shape error: {}", e))?;
        let token_type_ids_array = Array2::from_shape_vec((1, seq_len), token_type_ids)
            .map_err(|e| format!("Token type IDs shape error: {}", e))?;

        // 3. 运行推理
        let inputs = ort::inputs![
            "input_ids" => Value::from_array(input_ids_array).map_err(|e| format!("Input IDs value error: {}", e))?,
            "attention_mask" => Value::from_array(attention_mask_array).map_err(|e| format!("Attention mask value error: {}", e))?,
            "token_type_ids" => Value::from_array(token_type_ids_array).map_err(|e| format!("Token type IDs value error: {}", e))?,
        ];

        let mut session = self.session.lock().map_err(|e| format!("Session lock error: {}", e))?;
        let outputs = session.run(inputs)
            .map_err(|e| format!("Inference error: {}", e))?;

        // 4. 获取输出
        let output_value = outputs.get("last_hidden_state")
            .ok_or_else(|| "Output 'last_hidden_state' not found".to_string())?;
        let (shape, data) = output_value.try_extract_tensor::<f32>()
            .map_err(|e| format!("Tensor extraction error: {}", e))?;
        
        if shape.len() < 3 || shape[0] == 0 || shape[1] == 0 {
            return Err("Unexpected output shape".to_string());
        }
        
        let dim = shape[2] as usize;
        if data.len() < dim {
            return Err("Output data length mismatch".to_string());
        }
        
        let mut embedding: Vec<f32> = data[0..dim].to_vec();

        let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut embedding {
                *x /= norm;
            }
        }

        Ok(embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_embedding_differentiation() {
        // Assume running from src-tauri root
        let model_path = PathBuf::from("resources/model.onnx");
        let tokenizer_path = PathBuf::from("resources/tokenizer.json");
        
        if !model_path.exists() {
            println!("Skipping test: model not found");
            return;
        }

        let engine = EmbeddingEngine::new(&model_path, &tokenizer_path).unwrap();
        
        let v1 = engine.embed("如何提高团队效率？").unwrap();
        let v2 = engine.embed("提升组织生产力的方法").unwrap();
        let v3 = engine.embed("数据库连接池配置").unwrap();
        let v4 = engine.embed("数据库连接池配置").unwrap(); // identical

        let score_similar = v1.iter().zip(v2.iter()).map(|(x, y)| x * y).sum::<f32>();
        let score_different = v1.iter().zip(v3.iter()).map(|(x, y)| x * y).sum::<f32>();
        let score_identical = v3.iter().zip(v4.iter()).map(|(x, y)| x * y).sum::<f32>();

        println!("Score identical: {}", score_identical);
        println!("Score similar: {}", score_similar);
        println!("Score different: {}", score_different);
        println!("V1 top 5: {:?}", &v1[0..5]);
        
        assert!(score_identical > 0.99);
        assert!(score_similar > score_different);
        // We should expect score_different to be significantly lower than score_similar
    }
}


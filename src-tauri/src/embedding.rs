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
            .map_err(|e| e.to_string())?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| e.to_string())?
            .with_intra_threads(4) // 限制线程数，避免占用过多 CPU
            .map_err(|e| e.to_string())?
            .commit_from_file(model_path)
            .map_err(|e| e.to_string())?;

        // 2. 加载分词器
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| e.to_string())?;

        Ok(Self { 
            session: Mutex::new(session), 
            tokenizer 
        })
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        // 1. 文本分词
        let encoding = self.tokenizer.encode(text, true)
            .map_err(|e| e.to_string())?;
        
        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&mask| mask as i64).collect();
        let token_type_ids: Vec<i64> = encoding.get_type_ids().iter().map(|&id| id as i64).collect();

        let seq_len = input_ids.len();
        
        // 2. 准备输入 Tensor
        let input_ids_array = Array2::from_shape_vec((1, seq_len), input_ids).unwrap();
        let attention_mask_array = Array2::from_shape_vec((1, seq_len), attention_mask).unwrap();
        let token_type_ids_array = Array2::from_shape_vec((1, seq_len), token_type_ids).unwrap();

        // 3. 运行推理
        let inputs = ort::inputs![
            "input_ids" => Value::from_array(input_ids_array).map_err(|e| e.to_string())?,
            "attention_mask" => Value::from_array(attention_mask_array).map_err(|e| e.to_string())?,
            "token_type_ids" => Value::from_array(token_type_ids_array).map_err(|e| e.to_string())?,
        ];

        let mut session = self.session.lock().map_err(|e| e.to_string())?;
        let outputs = session.run(inputs)
            .map_err(|e| e.to_string())?;

        // 4. 获取输出
        let output_value = &outputs["last_hidden_state"];
        let (shape, data) = output_value.try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
        
        let dim = shape[2] as usize;
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

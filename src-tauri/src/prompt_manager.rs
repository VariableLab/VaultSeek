use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromptTemplate {
    pub name: String,
    pub system_prompt: String,
    pub description: String,
}

pub struct PromptManager {
    pub templates: HashMap<String, PromptTemplate>,
}

impl PromptManager {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        templates.insert("default".to_string(), PromptTemplate {
            name: "默认助手".to_string(),
            system_prompt: "你是一个专业的知识库助手。请根据提供的本地知识库片段来回答用户的问题。".to_string(),
            description: "通用问答模式".to_string(),
        });
        templates.insert("legal".to_string(), PromptTemplate {
            name: "法务专家".to_string(),
            system_prompt: "你是一位资深法务专家。请结合知识库，从合规、风险控制和法律条款的角度严谨回答问题。".to_string(),
            description: "合同审核与合规建议".to_string(),
        });
        PromptManager { templates }
    }
}

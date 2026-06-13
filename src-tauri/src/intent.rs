use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum IntentType {
    ClearQuery,
    Ambiguous,
    NeedsContext,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IntentDiagnosis {
    pub intent_type: IntentType,
    pub suggested_query: Option<String>,
    pub response_message: String,
}

pub struct IntentAnalyzer;

impl IntentAnalyzer {
    pub async fn analyze_intent(query: &str) -> IntentDiagnosis {
        // 模拟 LLM 对意图的分析逻辑
        if query.len() < 5 {
            IntentDiagnosis {
                intent_type: IntentType::Ambiguous,
                suggested_query: None,
                response_message: "您的提问太简短了，能提供更多背景信息吗？".to_string(),
            }
        } else {
            IntentDiagnosis {
                intent_type: IntentType::ClearQuery,
                suggested_query: Some(query.to_string()),
                response_message: "正在为您检索...".to_string(),
            }
        }
    }
}

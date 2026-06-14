use serde::{Deserialize, Serialize};
use reqwest_eventsource::{Event, EventSource};
use futures_util::stream::StreamExt;
use tauri::{AppHandle, Emitter};
use crate::state::DEFAULT_API_URL;

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Deserialize, Debug)]
struct ChatResponseDelta {
    choices: Vec<ChoiceDelta>,
}

#[derive(Deserialize, Debug)]
struct ChoiceDelta {
    delta: DeltaContent,
}

#[derive(Deserialize, Debug)]
struct DeltaContent {
    content: Option<String>,
}

pub async fn expand_query(api_key: String, model: String, api_url: String, query: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    // 优先使用传入的 api_url，如果没有则尝试从环境变量读取
    let final_url = if api_url.is_empty() {
        std::env::var("VAULTSEEK_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string())
    } else {
        api_url
    };

    let system_prompt = "你是一个检索意图优化专家。请将用户的大白话提问扩充为 5 个适用于本地知识库检索的专业关键词，以逗号分隔，不要包含任何多余解释。";

    let request_body = ChatRequest {
        model,
        messages: vec![
            ChatMessage { role: "system".to_string(), content: system_prompt.to_string() },
            ChatMessage { role: "user".to_string(), content: query },
        ],
        stream: false,
    };

    let response = client
        .post(final_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;

    let expanded = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(expanded)
}

pub async fn stream_chat(
    app: AppHandle,
    api_key: String,
    model: String,
    api_url: String,
    query: String,
    context: String,
    system_prompt_template: String,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    
    // 智能处理 URL：确保指向正确的 API 路径
    let final_url = if api_url.is_empty() {
        DEFAULT_API_URL.to_string()
    } else if !api_url.contains("/chat/completions") {
        let base = api_url.trim_end_matches('/');
        if base.ends_with("/v1") {
            format!("{}/chat/completions", base)
        } else {
            format!("{}/v1/chat/completions", base)
        }
    } else {
        api_url
    };

    println!(">>> LLM: 尝试请求 URL: {}", final_url);

    let system_prompt = format!(
        "{}\n\n如果知识库片段中没有相关信息，请明确说明，不要编造。\n\n知识库片段如下：\n\n{}",
        system_prompt_template,
        context
    );

    let request_body = ChatRequest {
        model,
        messages: vec![
            ChatMessage { role: "system".to_string(), content: system_prompt },
            ChatMessage { role: "user".to_string(), content: query },
        ],
        stream: true,
    };

    let builder = client
        .post(&final_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body);

    let mut es = EventSource::new(builder).map_err(|e| e.to_string())?;

    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Open) => println!(">>> LLM: 连接已建立"),
            Ok(Event::Message(message)) => {
                let data = message.data.trim();
                if data == "[DONE]" {
                    let _ = app.emit("chat-done", ());
                    break;
                }
                
                // 增加对 JSON 解析的健壮性：某些代理可能返回非标准格式
                if let Ok(parsed) = serde_json::from_str::<ChatResponseDelta>(data) {
                    if let Some(choice) = parsed.choices.first() {
                        if let Some(ref text) = choice.delta.content {
                            let _ = app.emit("chat-token", text);
                        }
                    }
                } else {
                    // 如果解析失败，尝试检查是否是包含错误信息的 JSON
                    if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(data) {
                         if let Some(error) = err_json.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                             let _ = app.emit("chat-error", error.to_string());
                         }
                    }
                }
            }
            Err(err) => {
                // 如果是 UnexpectedEof 且我们已经收到过消息，通常可以忽略并视为结束
                println!(">>> LLM: 流错误类型: {:?}", err);
                if err.to_string().contains("UnexpectedEof") {
                    println!(">>> LLM: 检测到 UnexpectedEof，尝试平滑结束");
                    let _ = app.emit("chat-done", ());
                    return Ok(());
                }
                let err_msg = format!("流式传输异常: {}。请检查网络或 API URL 配置。", err);
                return Err(err_msg);
            }
        }
    }

    Ok(())
}

use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Window};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub api_key: String,
    pub endpoint: String,
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmStreamEvent {
    pub chunk: String,
    pub done: bool,
}

#[tauri::command]
pub async fn chat_completion(
    window: Window,
    config: LlmConfig,
    messages: Vec<ChatMessage>,
    system_prompt: Option<String>,
) -> Result<String, String> {
    let client = Client::new();

    // Build message list with system prompt
    let mut full_messages = Vec::new();
    if let Some(sys) = system_prompt {
        full_messages.push(ChatMessage {
            role: "system".into(),
            content: sys,
        });
    }
    full_messages.extend(messages);

    let request = ApiRequest {
        model: config.model,
        messages: full_messages,
        stream: true,
    };

    let response = client
        .post(&format!("{}/chat/completions", config.endpoint))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    let mut stream = response.bytes_stream();
    let mut full_response = String::new();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // Process SSE lines
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if line.starts_with("data: ") {
                let data = &line[6..];
                if data == "[DONE]" {
                    let _ = window.emit(
                        "llm-stream",
                        LlmStreamEvent {
                            chunk: String::new(),
                            done: true,
                        },
                    );
                    return Ok(full_response);
                }
                if let Ok(parsed) = serde_json::from_str::<StreamChunk>(data) {
                    if let Some(choice) = parsed.choices.first() {
                        if let Some(content) = &choice.delta.content {
                            full_response.push_str(content);
                            let _ = window.emit(
                                "llm-stream",
                                LlmStreamEvent {
                                    chunk: content.clone(),
                                    done: false,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(full_response)
}

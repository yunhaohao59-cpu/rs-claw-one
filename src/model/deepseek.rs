use super::traits::*;
use futures_util::StreamExt;
use serde_json::Value;
use crate::agent::tool_use::parse_deepseek_tool_calls;

const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com/v1";

pub struct DeepSeekProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: reqwest::Client,
}

impl DeepSeekProvider {
    pub fn new(api_key: String, model: Option<String>, base_url: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| "deepseek-chat".into()),
            base_url: base_url.unwrap_or_else(|| DEEPSEEK_BASE_URL.into()),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl ChatProvider for DeepSeekProvider {
    async fn chat(&self, request: ChatRequest) -> anyhow::Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": request.messages.iter().map(|m| {
                serde_json::to_value(m).unwrap_or_default()
            }).collect::<Vec<_>>(),
            "stream": false,
        });

        if let Some(t) = request.temperature {
            body["temperature"] = Value::from(t);
        }
        if let Some(mt) = request.max_tokens {
            body["max_tokens"] = Value::from(mt);
        }
        if let Some(ref tools) = request.tools {
            body["tools"] = serde_json::json!(tools);
        }

        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("DeepSeek API error: {}", err_text);
        }

        let json: Value = resp.json().await?;

        let choice = &json["choices"][0];
        let content = choice["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let reasoning_content = choice["message"]["reasoning_content"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let tool_calls = parse_deepseek_tool_calls(choice)
            .into_iter()
            .map(|tc| super::traits::ToolCall::new(
                tc.id,
                tc.name,
                tc.arguments.to_string(),
            ))
            .collect();

        let usage = json.get("usage").map(|u| UsageInfo {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
        });

        Ok(ChatResponse {
            content,
            tool_calls,
            usage,
            reasoning_content,
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> anyhow::Result<Box<dyn futures_util::Stream<Item = anyhow::Result<String>> + Unpin + Send>> {
        let url = format!("{}/chat/completions", self.base_url);

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": request.messages.iter().map(|m| {
                serde_json::to_value(m).unwrap_or_default()
            }).collect::<Vec<_>>(),
            "stream": true,
        });

        if let Some(t) = request.temperature {
            body["temperature"] = Value::from(t);
        }
        if let Some(mt) = request.max_tokens {
            body["max_tokens"] = Value::from(mt);
        }

        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("DeepSeek streaming API error: {}", err_text);
        }

        let stream = resp.bytes_stream().map(|chunk| -> anyhow::Result<String> {
            let bytes = chunk?;
            let text = String::from_utf8_lossy(&bytes).to_string();

            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }
                    if let Ok(json) = serde_json::from_str::<Value>(data) {
                        if let Some(delta) = json["choices"][0]["delta"]["content"].as_str() {
                            content.push_str(delta);
                        }
                    }
                }
            }
            Ok(content)
        });

        Ok(Box::new(stream))
    }

    fn provider_name(&self) -> &str {
        "deepseek"
    }

    fn default_model(&self) -> &str {
        &self.model
    }
}

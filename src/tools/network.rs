use async_trait::async_trait;
use serde_json::Value;

use crate::agent::tool_use::{Tool, ToolDefinition};

pub struct HttpGetTool;

#[async_trait]
impl Tool for HttpGetTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "http_get".into(),
            description: "Make an HTTP GET request and return the response body".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The URL to fetch" }
                },
                "required": ["url"]
            }),
        }
    }

    async fn execute(&self, arguments: Value) -> anyhow::Result<String> {
        let url = arguments["url"].as_str().unwrap_or("");
        let client = reqwest::Client::builder()
            .user_agent("RS-Claw/0.1")
            .build()?;
        let resp = client.get(url).send().await?;
        let status = resp.status();
        let text = resp.text().await?;

        if text.len() > 8000 {
            Ok(format!("HTTP {} — {} bytes (truncated)\n{}...", status.as_u16(), text.len(), &text[..8000]))
        } else {
            Ok(format!("HTTP {}\n{}", status.as_u16(), text))
        }
    }
}

pub struct HttpPostTool;

#[async_trait]
impl Tool for HttpPostTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "http_post".into(),
            description: "Make an HTTP POST request with a JSON body and return the response".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The URL to post to" },
                    "body": { "type": "object", "description": "JSON body to send" }
                },
                "required": ["url"]
            }),
        }
    }

    async fn execute(&self, arguments: Value) -> anyhow::Result<String> {
        let url = arguments["url"].as_str().unwrap_or("");
        let body = arguments.get("body").cloned().unwrap_or(Value::Null);
        let client = reqwest::Client::builder()
            .user_agent("RS-Claw/0.1")
            .build()?;
        let resp = client.post(url).json(&body).send().await?;
        let status = resp.status();
        let text = resp.text().await?;

        if text.len() > 8000 {
            Ok(format!("HTTP {} — {} bytes (truncated)\n{}...", status.as_u16(), text.len(), &text[..8000]))
        } else {
            Ok(format!("HTTP {}\n{}", status.as_u16(), text))
        }
    }
}

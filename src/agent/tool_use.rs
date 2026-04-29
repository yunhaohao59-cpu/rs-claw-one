use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, arguments: Value) -> anyhow::Result<String>;
}

pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.push(Box::new(tool));
    }

    pub fn definitions(&self) -> Vec<Value> {
        self.tools
            .iter()
            .map(|t| {
                let def = t.definition();
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": def.name,
                        "description": def.description,
                        "parameters": def.parameters,
                    }
                })
            })
            .collect()
    }

    pub fn definition_text(&self) -> String {
        let mut text = String::from("Available tools:\n");
        for tool in &self.tools {
            let def = tool.definition();
            text.push_str(&format!(
                "- {}: {}\n  Parameters: {}\n",
                def.name, def.description, def.parameters
            ));
        }
        text
    }

    pub async fn dispatch(&self, name: &str, arguments: Value) -> anyhow::Result<String> {
        for tool in &self.tools {
            if tool.definition().name == name {
                return tool.execute(arguments).await;
            }
        }
        anyhow::bail!("Unknown tool: {}", name)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_deepseek_tool_calls(choice: &Value) -> Vec<ToolCall> {
    let mut calls = Vec::new();

    if let Some(tool_calls) = choice.get("message").and_then(|m| m.get("tool_calls")) {
        if let Some(arr) = tool_calls.as_array() {
            for tc in arr {
                let id = tc["id"].as_str().unwrap_or("").to_string();
                let func = &tc["function"];
                let name = func["name"].as_str().unwrap_or("").to_string();
                let args_str = func["arguments"].as_str().unwrap_or("{}");
                let arguments: Value = serde_json::from_str(args_str).unwrap_or(Value::Object(Default::default()));

                calls.push(ToolCall { id, name, arguments });
            }
        }
    }

    calls
}

impl ToolResult {
    pub fn to_message_content(&self) -> String {
        if self.is_error {
            format!("Tool error: {}", self.content)
        } else {
            self.content.clone()
        }
    }
}

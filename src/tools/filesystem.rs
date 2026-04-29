use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

use crate::agent::tool_use::{Tool, ToolDefinition};

pub struct FileSystemTool;

impl FileSystemTool {
    fn resolve(path: &str) -> PathBuf {
        let p = std::path::Path::new(path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")).join(p)
        }
    }
}

#[async_trait]
impl Tool for FileSystemTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "fs_read".into(),
            description: "Read the contents of a file".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the file to read" }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(&self, arguments: Value) -> anyhow::Result<String> {
        let path = arguments["path"].as_str().unwrap_or("");
        let resolved = Self::resolve(path);
        let content = fs::read_to_string(&resolved).await?;
        Ok(content)
    }
}

pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "fs_write".into(),
            description: "Write content to a file, creating parent directories if needed".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the file to write" },
                    "content": { "type": "string", "description": "Content to write" }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn execute(&self, arguments: Value) -> anyhow::Result<String> {
        let path = arguments["path"].as_str().unwrap_or("");
        let content = arguments["content"].as_str().unwrap_or("");
        let resolved = FileSystemTool::resolve(path);
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&resolved, content).await?;
        Ok(format!("Written {} bytes to {}", content.len(), path))
    }
}

pub struct FileListTool;

#[async_trait]
impl Tool for FileListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "fs_list".into(),
            description: "List files and directories in a given path".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path to list" }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(&self, arguments: Value) -> anyhow::Result<String> {
        let path = arguments["path"].as_str().unwrap_or(".");
        let resolved = FileSystemTool::resolve(path);
        let mut entries = Vec::new();
        let mut dir = fs::read_dir(&resolved).await?;
        while let Some(entry) = dir.next_entry().await? {
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
            entries.push(if is_dir { format!("{}/", name) } else { name });
        }
        entries.sort();
        Ok(entries.join("\n"))
    }
}

pub struct FileExistsTool;

#[async_trait]
impl Tool for FileExistsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "fs_exists".into(),
            description: "Check if a file or directory exists".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to check" }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(&self, arguments: Value) -> anyhow::Result<String> {
        let path = arguments["path"].as_str().unwrap_or("");
        let resolved = FileSystemTool::resolve(path);
        Ok(if fs::metadata(&resolved).await.is_ok() { "true" } else { "false" }.into())
    }
}

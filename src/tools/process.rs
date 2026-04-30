use async_trait::async_trait;
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

use crate::agent::tool_use::{Tool, ToolDefinition};

pub struct ShellTool;

#[async_trait]
impl Tool for ShellTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "shell".into(),
            description: "Execute a shell command and return its output. Use this to run terminal commands.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "The shell command to execute" },
                    "cwd": { "type": "string", "description": "Working directory for the command (optional)" }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, arguments: Value) -> anyhow::Result<String> {
        let command = arguments["command"].as_str().unwrap_or("");
        let cwd = arguments["cwd"].as_str();

        #[cfg(windows)]
        let mut cmd = {
            let mut c = Command::new("cmd");
            c.args(["/c", command]);
            c
        };
        #[cfg(not(windows))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.args(["-c", command]);
            c
        };

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        let output = cmd.output().await?;
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        let mut result = String::new();
        if !stdout.is_empty() {
            result.push_str(&format!("STDOUT:\n{}\n", stdout));
        }
        if !stderr.is_empty() {
            result.push_str(&format!("STDERR:\n{}\n", stderr));
        }
        result.push_str(&format!("Exit code: {}", exit_code));

        if result.trim().is_empty() {
            result = format!("Exit code: {}", exit_code);
        }
        Ok(result)
    }
}

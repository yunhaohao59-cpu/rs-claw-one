mod filesystem;
mod process;
mod network;

#[cfg(feature = "tools-desktop")]
mod desktop;

#[cfg(feature = "tools-browser")]
mod browser;

pub use filesystem::{FileSystemTool, FileWriteTool, FileListTool, FileExistsTool};
pub use process::ShellTool;
pub use network::{HttpGetTool, HttpPostTool};

use crate::agent::tool_use::ToolRegistry;

pub fn build_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(FileSystemTool);
    registry.register(FileWriteTool);
    registry.register(FileListTool);
    registry.register(FileExistsTool);
    registry.register(ShellTool);
    registry.register(HttpGetTool);
    registry.register(HttpPostTool);
    registry
}

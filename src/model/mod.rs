mod traits;
mod openai;
mod deepseek;
mod claude;

pub use traits::ChatProvider;
pub use traits::{ChatRequest, ChatResponse, ChatMessage, ToolCall, UsageInfo};
pub use openai::OpenAiProvider;
pub use deepseek::DeepSeekProvider;
pub use claude::ClaudeProvider;

pub fn create_provider(
    provider_type: &str,
    api_key: &str,
    model: Option<&str>,
    base_url: Option<&str>,
) -> anyhow::Result<Box<dyn ChatProvider>> {
    match provider_type {
        "openai" => Ok(Box::new(OpenAiProvider::new(
            api_key.to_string(),
            model.map(|s| s.to_string()),
            base_url.map(|s| s.to_string()),
        ))),
        "deepseek" => Ok(Box::new(DeepSeekProvider::new(
            api_key.to_string(),
            model.map(|s| s.to_string()),
            base_url.map(|s| s.to_string()),
        ))),
        "claude" => Ok(Box::new(ClaudeProvider::new(
            api_key.to_string(),
            model.map(|s| s.to_string()),
        ))),
        _ => anyhow::bail!("Unknown provider: {}", provider_type),
    }
}

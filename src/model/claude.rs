use super::traits::*;

pub struct ClaudeProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| "claude-4-sonnet".into()),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl ChatProvider for ClaudeProvider {
    async fn chat(&self, _request: ChatRequest) -> anyhow::Result<ChatResponse> {
        Ok(ChatResponse {
            content: String::new(),
            tool_calls: Vec::new(),
            usage: None,
            reasoning_content: None,
        })
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> anyhow::Result<Box<dyn futures_util::Stream<Item = anyhow::Result<String>> + Unpin + Send>> {
        use futures_util::stream;
        Ok(Box::new(stream::empty()))
    }

    fn provider_name(&self) -> &str {
        "claude"
    }

    fn default_model(&self) -> &str {
        &self.model
    }
}

#[async_trait::async_trait]
impl VisionProvider for ClaudeProvider {
    async fn analyze_image(&self, _image_data: &[u8], _prompt: &str) -> anyhow::Result<String> {
        Ok(String::new())
    }

    fn provider_name(&self) -> &str {
        "claude-vision"
    }
}

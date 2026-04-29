use super::traits::*;

pub struct OpenAiProvider {
    api_key: String,
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: Option<String>, base_url: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| "gpt-4o".into()),
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".into()),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl ChatProvider for OpenAiProvider {
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
        "openai"
    }

    fn default_model(&self) -> &str {
        &self.model
    }
}

#[async_trait::async_trait]
impl VisionProvider for OpenAiProvider {
    async fn analyze_image(&self, _image_data: &[u8], _prompt: &str) -> anyhow::Result<String> {
        Ok(String::new())
    }

    fn provider_name(&self) -> &str {
        "openai-vision"
    }
}

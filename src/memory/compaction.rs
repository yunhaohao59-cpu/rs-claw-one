use crate::model::{ChatProvider, ChatRequest, ChatMessage};

pub struct Compactor<'a> {
    model: &'a dyn ChatProvider,
}

impl<'a> Compactor<'a> {
    pub fn new(model: &'a dyn ChatProvider) -> Self {
        Self { model }
    }

    pub fn should_compact(&self, token_count: usize, threshold: usize) -> bool {
        token_count > threshold
    }

    pub async fn compact(
        &self,
        history: &[super::session_memory::MessageRecord],
    ) -> anyhow::Result<String> {
        let mut transcript = String::new();
        for m in history {
            transcript.push_str(&format!("[{}]: {}\n", m.role, m.content));
        }

        let messages = vec![
            ChatMessage::system("You are a conversation summarizer. Summarize the following conversation into a concise paragraph that captures key topics, decisions, and context. Keep it under 500 characters."),
            ChatMessage::user(format!("Summarize this conversation:\n{}", transcript)),
        ];

        let request = ChatRequest {
            model: "deepseek-chat".into(),
            messages,
            stream: false,
            temperature: Some(0.3),
            max_tokens: Some(512),
            tools: None,
        };

        let response = self.model.chat(request).await?;
        Ok(response.content)
    }
}

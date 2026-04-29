use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct SessionMemory {
    messages: VecDeque<ChatMessage>,
    max_messages: usize,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl SessionMemory {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: VecDeque::with_capacity(max_messages),
            max_messages,
        }
    }

    pub fn add(&mut self, role: impl Into<String>, content: impl Into<String>) {
        if self.messages.len() >= self.max_messages {
            self.messages.pop_front();
        }
        self.messages.push_back(ChatMessage {
            role: role.into(),
            content: content.into(),
            timestamp: chrono::Utc::now(),
        });
    }

    pub fn messages(&self) -> impl Iterator<Item = &ChatMessage> {
        self.messages.iter()
    }
}

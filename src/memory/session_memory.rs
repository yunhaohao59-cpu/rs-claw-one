use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct SessionMemory {
    messages: VecDeque<MessageRecord>,
    max_messages: usize,
}

#[derive(Debug, Clone)]
pub struct MessageRecord {
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
        self.messages.push_back(MessageRecord {
            role: role.into(),
            content: content.into(),
            timestamp: chrono::Utc::now(),
        });
    }

    pub fn messages(&self) -> impl Iterator<Item = &MessageRecord> {
        self.messages.iter()
    }

    pub fn count(&self) -> usize {
        self.messages.len()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn all_records(&self) -> Vec<MessageRecord> {
        self.messages.iter().cloned().collect()
    }

    pub fn estimated_tokens(&self) -> usize {
        self.messages.iter().map(|m| m.content.len() / 4).sum()
    }
}

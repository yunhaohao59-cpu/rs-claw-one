use crate::model::ChatProvider;

pub struct Compactor<'a> {
    model: &'a dyn ChatProvider,
}

impl<'a> Compactor<'a> {
    pub fn new(model: &'a dyn ChatProvider) -> Self {
        Self { model }
    }

    pub async fn compact(
        &self,
        _history: &[super::session_memory::ChatMessage],
    ) -> anyhow::Result<String> {
        Ok(String::new())
    }

    pub fn should_compact(&self, token_count: usize, threshold: usize) -> bool {
        token_count > threshold
    }
}

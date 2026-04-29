use crate::memory::VectorStore;

pub struct SkillMatcher {
    store: VectorStore,
}

impl SkillMatcher {
    pub fn new(store: VectorStore) -> Self {
        Self { store }
    }

    pub fn match_intent(&self, _intent_embedding: &[f32]) -> anyhow::Result<Vec<String>> {
        Ok(Vec::new())
    }
}

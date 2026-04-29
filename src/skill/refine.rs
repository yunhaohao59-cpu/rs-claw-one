use super::Skill;
use crate::model::ChatProvider;

pub struct SkillRefiner<'a> {
    model: &'a dyn ChatProvider,
}

impl<'a> SkillRefiner<'a> {
    pub fn new(model: &'a dyn ChatProvider) -> Self {
        Self { model }
    }

    pub async fn refine(&self, _task_log: &str) -> anyhow::Result<Skill> {
        Ok(Skill {
            name: String::new(),
            description: String::new(),
            trigger_patterns: Vec::new(),
            steps: Vec::new(),
            preconditions: Vec::new(),
            success_rate: 0.0,
            usage_count: 0,
            embedding: Vec::new(),
        })
    }
}

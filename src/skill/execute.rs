use super::Skill;

pub struct SkillExecutor;

impl SkillExecutor {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, _skill: &Skill) -> anyhow::Result<()> {
        Ok(())
    }
}

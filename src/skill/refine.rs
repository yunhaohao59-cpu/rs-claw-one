use super::store::Skill;
use crate::model::{ChatProvider, ChatRequest, ChatMessage};

pub struct SkillRefiner<'a> {
    model: &'a dyn ChatProvider,
}

impl<'a> SkillRefiner<'a> {
    pub fn new(model: &'a dyn ChatProvider) -> Self {
        Self { model }
    }

    pub async fn refine(&self, task_log: &str) -> anyhow::Result<Skill> {
        let messages = vec![
            ChatMessage::system(
                "You are a skill extraction system. Given a task execution log, extract a reusable skill definition in JSON format.\n\
                Output only valid JSON, no other text:\n\
                {\n  \"name\": \"short_skill_name\",\n  \"description\": \"what this skill does\",\n  \"trigger_patterns\": [\"when to use this\"],\n  \"steps\": [{\"type\": \"command\"|\"http\", \"command\": \"the command\", \"expected\": \"expected output hint\"}],\n  \"preconditions\": [\"things needed before running\"]\n}"
            ),
            ChatMessage::user(format!("Task log:\n{}", task_log)),
        ];

        let request = ChatRequest {
            model: "deepseek-chat".into(),
            messages,
            stream: false,
            temperature: Some(0.3),
            max_tokens: Some(1024),
            tools: None,
        };

        let response = self.model.chat(request).await?;
        let json_str = response.content.trim();

        let skill: Skill = serde_json::from_str(json_str).unwrap_or_else(|_| Skill {
            name: "unnamed_skill".into(),
            description: json_str.chars().take(200).collect(),
            trigger_patterns: vec![],
            steps: vec![],
            preconditions: vec![],
            success_rate: 0.0,
            usage_count: 1,
            embedding: vec![],
        });

        Ok(skill)
    }
}

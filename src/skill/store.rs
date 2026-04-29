use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub trigger_patterns: Vec<String>,
    #[serde(default)]
    pub steps: Vec<SkillStep>,
    #[serde(default)]
    pub preconditions: Vec<String>,
    #[serde(default)]
    pub success_rate: f64,
    #[serde(default)]
    pub usage_count: u64,
    #[serde(default)]
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStep {
    pub r#type: String,
    pub command: String,
    #[serde(default)]
    pub expected: Option<String>,
}

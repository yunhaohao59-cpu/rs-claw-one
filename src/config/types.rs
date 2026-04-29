use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsClawConfig {
    pub gateway: GatewayConfig,
    pub model: ModelConfig,
    pub vision: Option<VisionConfig>,
    pub memory: MemoryConfig,
    pub skill: SkillConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_host")]
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub api_key: String,
    #[serde(default = "default_chat_model")]
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionConfig {
    pub provider: String,
    pub api_key: String,
    #[serde(default = "default_vision_model")]
    pub model: String,
    pub fallback_to_accessibility: bool,
    pub coordinate_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_max_session_messages")]
    pub max_session_messages: usize,
    #[serde(default = "default_compaction_threshold")]
    pub compaction_threshold: usize,
    #[serde(default = "default_top_k_memories")]
    pub top_k_memories: usize,
    #[serde(default = "default_auto_compaction")]
    pub auto_compaction: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    #[serde(default = "default_auto_refine")]
    pub auto_refine: bool,
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f64,
}

fn default_port() -> u16 { 18789 }
fn default_host() -> String { "127.0.0.1".into() }
fn default_chat_model() -> String { "deepseek-chat".into() }
fn default_vision_model() -> String { "gpt-4o".into() }
fn default_max_session_messages() -> usize { 100 }
fn default_compaction_threshold() -> usize { 64000 }
fn default_top_k_memories() -> usize { 3 }
fn default_auto_compaction() -> bool { true }
fn default_auto_refine() -> bool { true }
fn default_similarity_threshold() -> f64 { 0.75 }

impl Default for RsClawConfig {
    fn default() -> Self {
        Self {
            gateway: GatewayConfig {
                port: default_port(),
                host: default_host(),
            },
            model: ModelConfig {
                provider: "deepseek".into(),
                api_key: String::new(),
                model: default_chat_model(),
                base_url: None,
            },
            vision: Some(VisionConfig {
                provider: "openai".into(),
                api_key: String::new(),
                model: default_vision_model(),
                fallback_to_accessibility: true,
                coordinate_mode: "relative".into(),
            }),
            memory: MemoryConfig {
                max_session_messages: default_max_session_messages(),
                compaction_threshold: default_compaction_threshold(),
                top_k_memories: default_top_k_memories(),
                auto_compaction: default_auto_compaction(),
            },
            skill: SkillConfig {
                auto_refine: default_auto_refine(),
                similarity_threshold: default_similarity_threshold(),
            },
        }
    }
}

impl RsClawConfig {
    pub fn config_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".rs-claw")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(Self::config_path(), content)?;
        Ok(())
    }
}

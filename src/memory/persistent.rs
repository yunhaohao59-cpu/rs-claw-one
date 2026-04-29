use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PersistentMemory {
    pub identity: Option<String>,
    pub soul: Option<String>,
    pub agents: Option<String>,
    config_dir: PathBuf,
}

impl PersistentMemory {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            identity: None,
            soul: None,
            agents: None,
            config_dir,
        }
    }

    pub fn load(&mut self) -> anyhow::Result<()> {
        for (name, field) in [
            ("identity.md", &mut self.identity),
            ("soul.md", &mut self.soul),
            ("agents.md", &mut self.agents),
        ] {
            let path = self.config_dir.join(name);
            if path.exists() {
                *field = Some(std::fs::read_to_string(&path)?);
            }
        }
        Ok(())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        for (name, content) in [
            ("identity.md", &self.identity),
            ("soul.md", &self.soul),
            ("agents.md", &self.agents),
        ] {
            if let Some(text) = content {
                std::fs::write(self.config_dir.join(name), text)?;
            }
        }
        Ok(())
    }
}

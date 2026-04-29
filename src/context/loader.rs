use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct ProjectContext {
    pub identity: Option<String>,
    pub soul: Option<String>,
    pub agents: Option<String>,
    pub project_type: Option<String>,
    pub repo_info: Option<String>,
    pub build_commands: Vec<String>,
}

impl ProjectContext {
    pub fn load_from(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let mut ctx = Self::default();

        ctx.load_markdown_files(path)?;
        ctx.infer_project_type(path)?;
        ctx.infer_repo_info(path)?;

        Ok(ctx)
    }

    fn load_markdown_files(&mut self, dir: &Path) -> anyhow::Result<()> {
        for (name, field) in [
            ("IDENTITY.md", &mut self.identity),
            ("SOUL.md", &mut self.soul),
            ("AGENTS.md", &mut self.agents),
        ] {
            let file_path = dir.join(name);
            if file_path.exists() {
                *field = Some(std::fs::read_to_string(&file_path)?);
            }
        }
        Ok(())
    }

    fn infer_project_type(&mut self, dir: &Path) -> anyhow::Result<()> {
        if dir.join("Cargo.toml").exists() {
            self.project_type = Some("rust".into());
        } else if dir.join("package.json").exists() {
            self.project_type = Some("node".into());
        } else if dir.join("go.mod").exists() {
            self.project_type = Some("go".into());
        } else if dir.join("pyproject.toml").exists() || dir.join("setup.py").exists() {
            self.project_type = Some("python".into());
        }

        if let Some(project_type) = &self.project_type {
            match project_type.as_str() {
                "rust" => {
                    if let Ok(content) = std::fs::read_to_string(dir.join("Cargo.toml")) {
                        if let Some(name) = content.lines()
                            .find(|l| l.trim().starts_with("name"))
                            .and_then(|l| l.split('=').nth(1))
                            .map(|s| s.trim().trim_matches('"'))
                        {
                            self.project_type = Some(format!("rust/{}", name));
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn infer_repo_info(&mut self, dir: &Path) -> anyhow::Result<()> {
        let git_config = dir.join(".git").join("config");
        if git_config.exists() {
            self.repo_info = Some(String::from("git repository detected"));
        }
        Ok(())
    }
}

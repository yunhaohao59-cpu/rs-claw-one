use clap::Subcommand;

mod args;
mod repl;
mod style;

pub use args::CliArgs;

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(about = "Start interactive REPL (default)")]
    Repl,

    #[command(about = "Send a single message")]
    Chat {
        #[arg(short, long)]
        message: String,
    },

    #[command(about = "Start the Gateway server")]
    Serve {
        #[arg(long, default_value = "18789")]
        port: u16,
    },

    #[command(about = "Manage skills")]
    Skill {
        #[command(subcommand)]
        action: SkillCommand,
    },

    #[command(about = "Run setup wizard")]
    Setup,
}

#[derive(Subcommand, Debug)]
pub enum SkillCommand {
    #[command(about = "List all skills")]
    List,

    #[command(about = "Run a specific skill")]
    Run { name: String },
}

pub async fn run_command(cmd: Command) -> anyhow::Result<()> {
    match cmd {
        Command::Repl => run_repl().await,
        Command::Chat { message } => {
            let config = crate::config::RsClawConfig::load()?;
            if config.model.api_key.is_empty() {
                anyhow::bail!("API Key not configured. Run 'rs-claw setup' first.");
            }
            let provider = crate::model::create_provider(
                &config.model.provider,
                &config.model.api_key,
                Some(&config.model.model),
                config.model.base_url.as_deref(),
            )?;
            let tools = crate::tools::build_registry();
            let config_dir = crate::config::RsClawConfig::config_dir();
            std::fs::create_dir_all(&config_dir).ok();
            let db = crate::storage::Database::open(config_dir.join("rs-claw.db"))?;
            let vs = crate::memory::VectorStore::new(config_dir.join("vectors"))?;
            let mut agent = crate::agent::AgentRuntime::with_storage(
                provider,
                config.model.model.clone(),
                tools,
                config.memory.max_session_messages,
                config.memory.compaction_threshold,
                config.memory.auto_compaction,
                config.skill.auto_refine,
                config.skill.similarity_threshold,
                config.memory.top_k_memories,
                db,
                vs,
            )?;
            let response = agent.handle_message_sync(&message).await?;
            println!("{}", response);
            Ok(())
        }
        Command::Serve { port } => {
            let config = crate::config::RsClawConfig::load()?;
            let server = crate::gateway::GatewayServer::new(config);
            server.run(port).await
        }
        Command::Skill { action } => {
            match action {
                SkillCommand::List => {
                    tracing::info!("Listing skills...");
                    Ok(())
                }
                SkillCommand::Run { name } => {
                    tracing::info!("Running skill: {}", name);
                    Ok(())
                }
            }
        }
        Command::Setup => {
            println!("RS-Claw Setup Wizard");
            println!("====================");
            println!();
            println!("Create a configuration file at ~/.rs-claw/config.toml");
            println!();

            let mut provider = String::new();
            print!("Provider [deepseek]: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            std::io::stdin().read_line(&mut provider)?;
            let provider = provider.trim();
            let provider = if provider.is_empty() { "deepseek" } else { provider };

            let mut api_key = String::new();
            print!("API Key: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            std::io::stdin().read_line(&mut api_key)?;
            let api_key = api_key.trim();

            if api_key.is_empty() {
                println!("API Key is required. Setup aborted.");
                return Ok(());
            }

            let mut model = String::new();
            print!("Model [deepseek-chat]: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            std::io::stdin().read_line(&mut model)?;
            let model = model.trim();
            let model = if model.is_empty() { "deepseek-chat" } else { model };

            let mut config = crate::config::RsClawConfig::default();
            config.model.provider = provider.to_string();
            config.model.api_key = api_key.to_string();
            config.model.model = model.to_string();
            config.save()?;

            println!();
            println!("Configuration saved to ~/.rs-claw/config.toml");
            println!("Run 'rs-claw' to start chatting!");
            Ok(())
        }
    }
}

pub async fn run_repl() -> anyhow::Result<()> {
    repl::start().await
}

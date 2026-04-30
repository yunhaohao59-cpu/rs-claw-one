use crate::config::RsClawConfig;
use crate::agent::AgentRuntime;
use crate::model;
use crate::tools;
use crate::storage::Database;
use crate::memory::VectorStore;

use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn start() -> anyhow::Result<()> {
    println!("╔══════════════════════════════════════════╗");
    println!("║         RS-Claw  v0.1.0                  ║");
    println!("║   AI-powered computer control agent      ║");
    println!("╚══════════════════════════════════════════╝");

    let config = RsClawConfig::load()?;
    let config_dir = RsClawConfig::config_dir();
    std::fs::create_dir_all(&config_dir).ok();

    if config.model.api_key.is_empty() {
        println!();
        println!("  ⚠  API Key not configured.");
        println!();
        println!("  Set your API key in ~/.rs-claw/config.toml:");
        println!("  [model]");
        println!("  provider = \"deepseek\"");
        println!("  api_key = \"sk-your-key-here\"");
        println!();
        println!("  Or run: rs-claw setup");
        return Ok(());
    }

    let provider = model::create_provider(
        &config.model.provider,
        &config.model.api_key,
        Some(&config.model.model),
        config.model.base_url.as_deref(),
    )?;

    let tools = tools::build_registry();

    let db = Database::open(config_dir.join("rs-claw.db"))?;
    let vector_store = VectorStore::new(config_dir.join("vectors"))?;

    tracing::info!("Storage ready at {}", config_dir.display());

    println!();
    println!("  Provider: {}", config.model.provider);
    println!("  Model:    {}", config.model.model);
    println!("  Tools:    {} available", tools.definitions().len());
    println!("  Storage:  SQLite + Vector (trigram HNSW)");
    println!();
    println!("  Type your message and press Enter to chat.");
    println!("  Commands: /help  /quit  /clear  /config  /sessions");
    println!();

    let agent = Arc::new(Mutex::new(
        AgentRuntime::with_storage(
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
            vector_store,
        )?
    ));

    loop {
        print!("> ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();

        if input.is_empty() {
            continue;
        }

        match input.as_str() {
            "/quit" | "/exit" => {
                println!("  Goodbye! Session saved.");
                break;
            }
            "/help" => {
                println!("  /help     - Show this help");
                println!("  /quit     - Exit (auto-save)");
                println!("  /clear    - Clear session history");
                println!("  /config   - Show current configuration");
                println!("  /tools    - List available tools");
                println!("  /sessions - List saved sessions");
                println!("  Any other text is sent to the AI.");
                continue;
            }
            "/clear" => {
                let new_provider = model::create_provider(
                    &config.model.provider,
                    &config.model.api_key,
                    Some(&config.model.model),
                    config.model.base_url.as_deref(),
                )?;
                let new_tools = tools::build_registry();
                let new_db = Database::open(config_dir.join("rs-claw.db"))?;
                let new_vs = VectorStore::new(config_dir.join("vectors"))?;
                let mut agent = agent.lock().await;
                *agent = AgentRuntime::with_storage(
                    new_provider,
                    config.model.model.clone(),
                    new_tools,
                    config.memory.max_session_messages,
                    config.memory.compaction_threshold,
                    config.memory.auto_compaction,
                    config.skill.auto_refine,
                    config.skill.similarity_threshold,
                    config.memory.top_k_memories,
                    new_db,
                    new_vs,
                )?;
                println!("  Session cleared (new session started).");
                continue;
            }
            "/config" => {
                println!("  Provider: {}", config.model.provider);
                println!("  Model:    {}", config.model.model);
                if let Some(ref url) = config.model.base_url {
                    println!("  Base URL: {}", url);
                }
                println!("  Gateway:  {}:{}", config.gateway.host, config.gateway.port);
                println!("  Auto-refine: {}", config.skill.auto_refine);
                continue;
            }
            "/tools" => {
                let registry = tools::build_registry();
                for def in registry.definitions() {
                    let func = &def["function"];
                    println!("  - {}", func["name"].as_str().unwrap_or("?"));
                }
                continue;
            }
            "/sessions" => {
                let db = Database::open(config_dir.join("rs-claw.db"))?;
                match db.list_sessions() {
                    Ok(sessions) => {
                        if sessions.is_empty() {
                            println!("  No saved sessions.");
                        } else {
                            println!("  Saved sessions:");
                            for (id, name, updated) in &sessions {
                                println!("    {}  {}  {}", &id[..8], name, updated);
                            }
                        }
                    }
                    Err(e) => println!("  Error: {}", e),
                }
                continue;
            }
            _ => {}
        }

        let mut agent = agent.lock().await;
        print!("🤖 ");
        std::io::Write::flush(&mut std::io::stdout())?;

        match agent.handle_message_sync(&input).await {
            Ok(response) => {
                println!("{}", response);
                println!();
            }
            Err(e) => {
                eprintln!("  Error: {}", e);
            }
        }
    }

    Ok(())
}

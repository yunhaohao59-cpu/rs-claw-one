use crate::config::RsClawConfig;
use crate::agent::AgentRuntime;
use crate::model;
use crate::tools;
use crate::storage::Database;
use crate::memory::VectorStore;

use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn start() -> anyhow::Result<()> {
    super::style::welcome();

    let config = RsClawConfig::load()?;
    let config_dir = RsClawConfig::config_dir();
    std::fs::create_dir_all(&config_dir).ok();

    if config.model.api_key.is_empty() {
        println!();
        eprintln!("  ⚠  API Key not configured. Run 'rs-claw setup'");
        return Ok(());
    }

    let provider = model::create_provider(
        &config.model.provider,
        &config.model.api_key,
        Some(&config.model.model),
        config.model.base_url.as_deref(),
    )?;

    let reg = tools::build_registry();
    let db = Database::open(config_dir.join("rs-claw.db"))?;
    let vs = VectorStore::new(config_dir.join("vectors"))?;

    let agent = Arc::new(Mutex::new(AgentRuntime::with_storage(
        provider, config.model.model.clone(), reg,
        config.memory.max_session_messages,
        config.memory.compaction_threshold,
        config.memory.auto_compaction,
        config.skill.auto_refine,
        config.skill.similarity_threshold,
        config.memory.top_k_memories,
        db, vs,
    )?));

    {
        use super::style as s;
        let a = agent.lock().await;
        s::info_line("Provider", &config.model.provider);
        s::info_line("Model", &config.model.model);
        s::info_line("Session", &format!("{} (new)", &a.session_id()[..8]));
        println!();
    }

    loop {
        super::style::prompt();
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();
        if input.is_empty() { continue; }

        if input.starts_with("/switch ") {
            let target = input[8..].trim();
            let mut a = agent.lock().await;
            let sessions = match a.list_sessions() {
                Ok(s) => s, Err(e) => { println!("  Error: {}", e); continue; }
            };
            if let Some((id, _, updated)) = sessions.iter().find(|(id,_,_)| id.starts_with(target)) {
                match a.switch_session(id) {
                    Ok(()) => super::style::system_msg(&format!("Switched to {} (last: {})", &id[..8], updated)),
                    Err(e) => eprintln!("  Error: {}", e),
                }
            } else {
                super::style::system_msg(&format!("No session matching '{}'", target));
            }
            continue;
        }

        if input == "/new" {
            let mut a = agent.lock().await;
            match a.new_session() {
                Ok(id) => super::style::system_msg(&format!("New session {}", &id[..8])),
                Err(e) => eprintln!("  Error: {}", e),
            }
            continue;
        }

        match input.as_str() {
            "/quit" | "/exit" => { super::style::system_msg("Goodbye! Session saved."); break; }
            "/help" => {
                use super::style as s;
                s::info_line("/help", "Show help");
                s::info_line("/quit", "Exit (auto-save)");
                s::info_line("/new", "New session");
                s::info_line("/clear", "New session (alias)");
                s::info_line("/config", "Show configuration");
                s::info_line("/tools", "List tools");
                s::info_line("/sessions", "List sessions");
                s::info_line("/switch <id>", "Switch session");
                continue;
            }
            "/clear" => {
                let mut a = agent.lock().await;
                match a.new_session() {
                    Ok(id) => super::style::system_msg(&format!("New session {}", &id[..8])),
                    Err(e) => eprintln!("  Error: {}", e),
                }
                continue;
            }
            "/config" => {
                let a = agent.lock().await;
                use super::style as s;
                s::info_line("Provider", &config.model.provider);
                s::info_line("Model", &config.model.model);
                if let Some(ref url) = config.model.base_url { s::info_line("Base URL", url); }
                s::info_line("Gateway", &format!("{}:{}", config.gateway.host, config.gateway.port));
                s::info_line("Session", &a.session_id()[..8]);
                continue;
            }
            "/tools" => {
                let reg = tools::build_registry();
                use super::style as s;
                for d in reg.definitions() {
                    let n = d["function"]["name"].as_str().unwrap_or("?");
                    s::info_line("", n);
                }
                continue;
            }
            "/sessions" => {
                let a = agent.lock().await;
                let cur = a.session_id().to_string();
                match a.list_sessions() {
                    Ok(sessions) => {
                        if sessions.is_empty() {
                            super::style::system_msg("No saved sessions");
                        } else {
                            super::style::system_msg("Saved sessions");
                            for (id, _, updated) in &sessions {
                                super::style::session_info(id, updated, id == &cur);
                            }
                        }
                    }
                    Err(e) => eprintln!("  Error: {}", e),
                }
                continue;
            }
            _ => {}
        }

        super::style::user_msg(&input);

        let mut a = agent.lock().await;
        super::style::ai_header();
        std::io::Write::flush(&mut std::io::stdout())?;

        match a.handle_message_sync(&input).await {
            Ok(response) => {
                let mut lines = response.lines();
                let first = lines.next().unwrap_or("");
                println!("{}", first);
                for line in lines {
                    if line.starts_with("🔧") {
                        super::style::tool_start(line.trim_start_matches("🔧 "));
                    } else if line.starts_with("  ✓") {
                        let rest = line.trim_start_matches("  ✓ ").trim();
                        let (name, summary) = rest.split_once(" → ").unwrap_or((rest, ""));
                        super::style::tool_ok(name, summary);
                    } else if line.starts_with("  ✗") {
                        let rest = line.trim_start_matches("  ✗ ").trim();
                        let (name, err) = rest.split_once(" → ").unwrap_or((rest, ""));
                        super::style::tool_err(name, err);
                    } else {
                        super::style::ai_line(line);
                    }
                }
                super::style::ai_footer();
                println!();
            }
            Err(e) => {
                use owo_colors::OwoColorize;
                let msg = format!("Error: {}", e);
                super::style::ai_line(&msg.red().to_string());
            }
        }
    }

    Ok(())
}

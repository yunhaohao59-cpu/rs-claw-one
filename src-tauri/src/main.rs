use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Manager, Emitter};
use serde::Serialize;

struct AppState {
    agent: Arc<Mutex<rs_claw::agent::AgentRuntime>>,
}

#[derive(Clone, Serialize)]
struct ChatEvent {
    #[serde(rename = "type")]
    event_type: String,
    text: Option<String>,
    tool_name: Option<String>,
    tool_result: Option<String>,
    tool_duration_ms: Option<u64>,
    tool_error: Option<String>,
    finish_reason: Option<String>,
    session_id: Option<String>,
}

impl ChatEvent {
    fn text_delta(text: &str) -> Self {
        Self { event_type: "text_delta".into(), text: Some(text.to_string()),
            tool_name: None, tool_result: None, tool_duration_ms: None,
            tool_error: None, finish_reason: None, session_id: None }
    }
    fn tool_call(name: &str) -> Self {
        Self { event_type: "tool_call".into(), text: None, tool_name: Some(name.to_string()),
            tool_result: None, tool_duration_ms: None, tool_error: None,
            finish_reason: None, session_id: None }
    }
    fn tool_result_ok(name: &str, result: &str, ms: u64) -> Self {
        Self { event_type: "tool_result".into(), text: None, tool_name: Some(name.to_string()),
            tool_result: Some(result.to_string()), tool_duration_ms: Some(ms),
            tool_error: None, finish_reason: None, session_id: None }
    }
    fn tool_error(name: &str, err: &str) -> Self {
        Self { event_type: "tool_error".into(), text: None, tool_name: Some(name.to_string()),
            tool_result: None, tool_duration_ms: None,
            tool_error: Some(err.to_string()), finish_reason: None, session_id: None }
    }
    fn done(session_id: &str) -> Self {
        Self { event_type: "done".into(), text: None, tool_name: None,
            tool_result: None, tool_duration_ms: None, tool_error: None,
            finish_reason: Some("stop".into()), session_id: Some(session_id.to_string()) }
    }
    fn error_msg(msg: &str) -> Self {
        Self { event_type: "error".into(), text: Some(msg.to_string()), tool_name: None,
            tool_result: None, tool_duration_ms: None, tool_error: None,
            finish_reason: None, session_id: None }
    }
}

#[tauri::command]
async fn send_message(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    message: String,
) -> Result<(), String> {
    let mut agent = state.agent.lock().await;
    let response = agent.handle_message_sync(&message).await.map_err(|e| e.to_string())?;
    let lines: Vec<&str> = response.lines().collect();
    for line in &lines {
        if line.starts_with("🔧") {
            let name = line.trim_start_matches("🔧 ").to_string();
            let _ = app.emit("chat-event", ChatEvent::tool_call(&name));
        } else if line.starts_with("  ✓") {
            let rest = line.trim_start_matches("  ✓ ").trim();
            let (name, result) = rest.split_once(" → ").unwrap_or((rest, ""));
            let _ = app.emit("chat-event", ChatEvent::tool_result_ok(name, result, 0));
        } else if line.starts_with("  ✗") {
            let rest = line.trim_start_matches("  ✗ ").trim();
            let (name, err) = rest.split_once(" → ").unwrap_or((rest, ""));
            let _ = app.emit("chat-event", ChatEvent::tool_error(name, err));
        } else {
            let _ = app.emit("chat-event", ChatEvent::text_delta(line));
        }
    }
    let _ = app.emit("chat-event", ChatEvent::done(agent.session_id()));
    Ok(())
}

#[tauri::command]
async fn new_session(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let mut agent = state.agent.lock().await;
    agent.new_session().map_err(|e| e.to_string())
}

#[tauri::command]
async fn switch_session(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let mut agent = state.agent.lock().await;
    agent.switch_session(&session_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_sessions(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Vec<String>>, String> {
    let agent = state.agent.lock().await;
    let sessions = agent.list_sessions().map_err(|e| e.to_string())?;
    Ok(sessions.iter().map(|(id, _, updated)| vec![id.clone(), updated.clone()]).collect())
}

#[tauri::command]
async fn current_session(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let agent = state.agent.lock().await;
    Ok(agent.session_id().to_string())
}

#[tauri::command]
async fn get_tools(
    _state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let reg = rs_claw::tools::build_registry();
    Ok(reg.definitions().iter().map(|d| d["function"]["name"].as_str().unwrap_or("?").to_string()).collect())
}

#[tauri::command]
async fn minimize_window(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
async fn maximize_window(window: tauri::Window) -> Result<(), String> {
    window.maximize().map_err(|e| e.to_string())
}

#[tauri::command]
async fn unmaximize_window(window: tauri::Window) -> Result<(), String> {
    window.unmaximize().map_err(|e| e.to_string())
}

#[tauri::command]
async fn close_window(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let config = rs_claw::config::RsClawConfig::load().unwrap_or_default();
            let provider = rs_claw::model::create_provider(
                &config.model.provider,
                &config.model.api_key,
                Some(&config.model.model),
                config.model.base_url.as_deref(),
            ).expect("Failed to create model provider");

            let tools = rs_claw::tools::build_registry();
            let config_dir = rs_claw::config::RsClawConfig::config_dir();
            std::fs::create_dir_all(&config_dir).ok();
            let db = rs_claw::storage::Database::open(config_dir.join("rs-claw.db"))
                .expect("Failed to open database");
            let vs = rs_claw::memory::VectorStore::new(config_dir.join("vectors"))
                .expect("Failed to open vector store");

            let agent = rs_claw::agent::AgentRuntime::with_storage(
                provider, config.model.model, tools,
                config.memory.max_session_messages,
                config.memory.compaction_threshold,
                config.memory.auto_compaction,
                config.skill.auto_refine,
                config.skill.similarity_threshold,
                config.memory.top_k_memories,
                db, vs,
            ).expect("Failed to create agent");

            app.manage(AppState { agent: Arc::new(Mutex::new(agent)) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            send_message, new_session, switch_session,
            list_sessions, current_session, get_tools,
            minimize_window, maximize_window, unmaximize_window, close_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

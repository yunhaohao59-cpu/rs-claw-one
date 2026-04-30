use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Manager, Emitter};
use serde::Serialize;

struct AppState {
    agent: Arc<Mutex<rs_claw::agent::AgentRuntime>>,
    config: Arc<Mutex<rs_claw::config::RsClawConfig>>,
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
}

#[tauri::command]
async fn send_message(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    message: String,
) -> Result<String, String> {
    let mut agent = state.agent.lock().await;

    let app_clone = app.clone();
    let app_tool = app.clone();
    let app_text = app.clone();

    let response = agent.handle_message_stream_sync(
        &message,
        move |text| {
            let _ = app_text.emit("chat-event", ChatEvent::text_delta(text));
        },
        move |tool_name| {
            let _ = app_tool.emit("chat-event", ChatEvent::tool_call(tool_name));
        },
        move |tool_name, result, ok| {
            if ok {
                let _ = app_clone.emit("chat-event", ChatEvent::tool_result_ok(tool_name, result, 0));
            } else {
                let _ = app_clone.emit("chat-event", ChatEvent::tool_error(tool_name, result));
            }
        },
    ).await.map_err(|e| e.to_string())?;

    let sid = agent.session_id().to_string();
    let _ = app.emit("chat-event", ChatEvent::done(&sid));
    Ok(response)
}

#[tauri::command]
async fn new_session(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let mut agent = state.agent.lock().await;
    agent.new_session().map_err(|e| e.to_string())
}

#[tauri::command]
async fn switch_session(state: tauri::State<'_, AppState>, session_id: String) -> Result<(), String> {
    let mut agent = state.agent.lock().await;
    agent.switch_session(&session_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_session(state: tauri::State<'_, AppState>, session_id: String) -> Result<(), String> {
    let agent = state.agent.lock().await;
    agent.delete_session(&session_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn rename_session(state: tauri::State<'_, AppState>, session_id: String, name: String) -> Result<(), String> {
    let agent = state.agent.lock().await;
    agent.rename_session(&session_id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_sessions(state: tauri::State<'_, AppState>) -> Result<Vec<Vec<String>>, String> {
    let agent = state.agent.lock().await;
    let sessions = agent.list_sessions().map_err(|e| e.to_string())?;
    Ok(sessions.iter().map(|(id, name, updated)| vec![id.clone(), name.clone(), updated.clone()]).collect())
}

#[tauri::command]
async fn current_session(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let agent = state.agent.lock().await;
    Ok(agent.session_id().to_string())
}

#[tauri::command]
async fn get_tools() -> Result<Vec<String>, String> {
    let reg = rs_claw::tools::build_registry();
    Ok(reg.definitions().iter().map(|d| d["function"]["name"].as_str().unwrap_or("?").to_string()).collect())
}

#[tauri::command]
async fn get_config(state: tauri::State<'_, AppState>) -> Result<rs_claw::config::RsClawConfig, String> {
    let config = state.config.lock().await;
    Ok(config.clone())
}

#[tauri::command]
async fn save_config(
    state: tauri::State<'_, AppState>,
    provider: String,
    api_key: String,
    model: String,
    base_url: Option<String>,
) -> Result<(), String> {
    let mut config = state.config.lock().await;
    config.model.provider = provider;
    config.model.api_key = api_key;
    config.model.model = model;
    config.model.base_url = base_url;
    config.save().map_err(|e| e.to_string())
}

#[tauri::command]
async fn has_api_key(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let config = state.config.lock().await;
    Ok(!config.model.api_key.is_empty())
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
            let config_dir = rs_claw::config::RsClawConfig::config_dir();
            std::fs::create_dir_all(&config_dir).ok();

            let has_key = !config.model.api_key.is_empty();

            let model_name = config.model.model.clone();
            let max_msgs = config.memory.max_session_messages;
            let compact_thresh = config.memory.compaction_threshold;
            let auto_compact = config.memory.auto_compaction;
            let auto_refine = config.skill.auto_refine;
            let sim_thresh = config.skill.similarity_threshold;
            let top_k = config.memory.top_k_memories;

            let agent = if has_key {
                let provider = rs_claw::model::create_provider(
                    &config.model.provider, &config.model.api_key,
                    Some(&config.model.model), config.model.base_url.as_deref(),
                ).expect("Failed to create provider");

                let tools = rs_claw::tools::build_registry();
                let db = rs_claw::storage::Database::open(config_dir.join("rs-claw.db"))
                    .expect("Failed to open database");
                let vs = rs_claw::memory::VectorStore::new(config_dir.join("vectors"))
                    .expect("Failed to open vector store");

                rs_claw::agent::AgentRuntime::with_storage(
                    provider, model_name.clone(), tools,
                    max_msgs, compact_thresh, auto_compact,
                    auto_refine, sim_thresh, top_k,
                    db, vs,
                ).expect("Failed to create agent")
            } else {
                let provider = rs_claw::model::create_provider(
                    "deepseek", "", Some("deepseek-chat"), None,
                ).expect("Failed to create placeholder provider");
                let tools = rs_claw::tools::build_registry();
                let db = rs_claw::storage::Database::open(config_dir.join("rs-claw.db"))
                    .expect("Failed to open database");
                let vs = rs_claw::memory::VectorStore::new(config_dir.join("vectors"))
                    .expect("Failed to open vector store");
                rs_claw::agent::AgentRuntime::with_storage(
                    provider, model_name, tools,
                    max_msgs, compact_thresh, auto_compact,
                    auto_refine, sim_thresh, top_k,
                    db, vs,
                ).expect("Failed to create agent")
            };

            app.manage(AppState {
                agent: Arc::new(Mutex::new(agent)),
                config: Arc::new(Mutex::new(config)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            send_message, new_session, switch_session, delete_session, rename_session,
            list_sessions, current_session, get_tools,
            get_config, save_config, has_api_key,
            minimize_window, maximize_window, unmaximize_window, close_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use crate::gateway::{Frame, AgentEventPayload, AgentEventData};
use crate::model::{ChatProvider, ChatRequest, ChatMessage, ToolCall as ModelToolCall};
use crate::memory::{SessionMemory, VectorStore, Compactor};
use crate::agent::tool_use::ToolRegistry;
use crate::agent::system_prompt::build_system_prompt;
use crate::context::ProjectContext;
use crate::storage::Database;
use crate::skill::{SkillRefiner, Skill};

const MAX_TOOL_LOOPS: usize = 10;
const VECTOR_STORE_DIR: &str = "vectors";

pub struct AgentRuntime {
    session_memory: SessionMemory,
    provider: Box<dyn ChatProvider>,
    model_name: String,
    tools: ToolRegistry,
    system_prompt: String,
    max_session_messages: usize,
    compaction_threshold: usize,
    auto_compaction: bool,
    auto_refine: bool,
    similarity_threshold: f64,
    db: Option<Database>,
    vector_store: Option<VectorStore>,
    session_id: String,
    top_k_memories: usize,
}

impl AgentRuntime {
    pub fn with_config(
        provider: Box<dyn ChatProvider>,
        model_name: String,
        tools: ToolRegistry,
        max_session_messages: usize,
        compaction_threshold: usize,
        auto_compaction: bool,
    ) -> Self {
        let ctx = ProjectContext::load_from(".").unwrap_or_default();
        let system_prompt = build_system_prompt(&ctx, &tools);

        Self {
            session_memory: SessionMemory::new(max_session_messages),
            provider,
            model_name,
            tools,
            system_prompt,
            max_session_messages,
            compaction_threshold,
            auto_compaction,
            auto_refine: true,
            similarity_threshold: 0.75,
            db: None,
            vector_store: None,
            session_id: String::new(),
            top_k_memories: 3,
        }
    }

    pub fn with_storage(
        provider: Box<dyn ChatProvider>,
        model_name: String,
        tools: ToolRegistry,
        max_session_messages: usize,
        compaction_threshold: usize,
        auto_compaction: bool,
        auto_refine: bool,
        similarity_threshold: f64,
        top_k_memories: usize,
        db: Database,
        vector_store: VectorStore,
    ) -> anyhow::Result<Self> {
        let ctx = ProjectContext::load_from(".").unwrap_or_default();
        let system_prompt = build_system_prompt(&ctx, &tools);

        let session_id = db.create_session()?;

        Ok(Self {
            session_memory: SessionMemory::new(max_session_messages),
            provider,
            model_name,
            tools,
            system_prompt,
            max_session_messages,
            compaction_threshold,
            auto_compaction,
            auto_refine,
            similarity_threshold,
            db: Some(db),
            vector_store: Some(vector_store),
            session_id,
            top_k_memories,
        })
    }

    pub fn with_session(
        provider: Box<dyn ChatProvider>,
        model_name: String,
        tools: ToolRegistry,
        max_session_messages: usize,
        compaction_threshold: usize,
        auto_compaction: bool,
        auto_refine: bool,
        similarity_threshold: f64,
        top_k_memories: usize,
        db: Database,
        vector_store: VectorStore,
        session_id: String,
    ) -> anyhow::Result<Self> {
        let ctx = ProjectContext::load_from(".").unwrap_or_default();
        let system_prompt = build_system_prompt(&ctx, &tools);

        let mut s = Self {
            session_memory: SessionMemory::new(max_session_messages),
            provider,
            model_name,
            tools,
            system_prompt,
            max_session_messages,
            compaction_threshold,
            auto_compaction,
            auto_refine,
            similarity_threshold,
            db: Some(db),
            vector_store: Some(vector_store),
            session_id: session_id.clone(),
            top_k_memories,
        };

        s.restore_from_db()?;
        Ok(s)
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn list_sessions(&self) -> anyhow::Result<Vec<(String, String, String)>> {
        match &self.db {
            Some(db) => db.list_sessions(),
            None => Ok(Vec::new()),
        }
    }

    pub fn switch_session(&mut self, session_id: &str) -> anyhow::Result<()> {
        let db = match &self.db {
            Some(ref db) => db,
            None => anyhow::bail!("No database configured"),
        };
        let count = db.count_chats(session_id)?;
        if count == 0 {
            db.touch_session(session_id)?;
        }
        self.session_id = session_id.to_string();
        self.session_memory.clear();
        let rows = db.load_chats(&self.session_id)?;
        for row in &rows {
            self.session_memory.add(&row.role, &row.content);
        }
        tracing::info!("Switched to session {} ({} messages)", &self.session_id[..8], rows.len());
        Ok(())
    }

    pub fn new_session(&mut self) -> anyhow::Result<String> {
        let db = match &self.db {
            Some(ref db) => db,
            None => anyhow::bail!("No database configured"),
        };
        let id = db.create_session()?;
        self.session_id = id.clone();
        self.session_memory.clear();
        Ok(id)
    }

    fn restore_from_db(&mut self) -> anyhow::Result<()> {
        let db = match &self.db {
            Some(db) => db,
            None => return Ok(()),
        };
        let rows = db.load_chats(&self.session_id)?;
        for row in &rows {
            self.session_memory.add(&row.role, &row.content);
        }
        if !rows.is_empty() {
            tracing::info!("Restored {} messages for session {}", rows.len(), self.session_id);
        }
        Ok(())
    }

    fn save_chat(&self, role: &str, content: &str) {
        if let Some(ref db) = self.db {
            if let Err(e) = db.insert_chat(&self.session_id, role, content) {
                tracing::warn!("Failed to save chat: {}", e);
            }
        }
    }

    fn inject_memories(&self, messages: &mut Vec<ChatMessage>) {
        let db = match &self.db { Some(d) => d, None => return };

        let keywords: Vec<&str> = self.session_memory.messages()
            .last()
            .map(|m| m.content.split_whitespace().take(5).collect::<Vec<_>>())
            .unwrap_or_default();

        let matching = db.search_memories(&keywords, self.top_k_memories).unwrap_or_default();
        if !matching.is_empty() {
            let memory_text: Vec<String> = matching.iter()
                .map(|m| format!("[Memory #{}]: {}", &m.id[..6], m.content))
                .collect();
            messages.insert(0, ChatMessage::system(
                format!("Relevant memories:\n{}", memory_text.join("\n"))
            ));
        }
    }

    pub async fn handle_message(
        &mut self,
        message: &str,
        sender: tokio::sync::broadcast::Sender<String>,
    ) -> anyhow::Result<()> {
        self.session_memory.add("user", message);
        self.save_chat("user", message);

        self.run_loop(sender).await
    }

    pub async fn handle_message_sync(&mut self, message: &str) -> anyhow::Result<String> {
        self.session_memory.add("user", message);
        self.save_chat("user", message);

        self.run_loop_sync().await
    }

    async fn run_loop(
        &mut self,
        sender: tokio::sync::broadcast::Sender<String>,
    ) -> anyhow::Result<()> {
        let mut messages = self.build_messages();
        self.inject_memories(&mut messages);
        let tool_defs = self.tools.definitions();

        let mut loop_count = 0;
        let mut task_log = String::new();

        loop {
            if loop_count >= MAX_TOOL_LOOPS {
                let err = Frame::event("error", serde_json::json!({
                    "message": "Max tool loop iterations reached"
                }));
                let _ = sender.send(err.to_text());
                break;
            }
            loop_count += 1;

            if self.auto_compaction {
                let tokens = self.session_memory.estimated_tokens();
                if tokens > self.compaction_threshold {
                    let event = Frame::event("agent", serde_json::to_value(AgentEventPayload {
                        stream: "system".into(), data: AgentEventData {
                            text: Some("📝 Compacting context...".into()),
                            finish_reason: None, tool_calls: None,
                        },
                    })?);
                    let _ = sender.send(event.to_text());

                    let compactor = Compactor::new(self.provider.as_ref());
                    let records = self.session_memory.all_records();
                    match compactor.compact(&records).await {
                        Ok(summary) => {
                            self.session_memory.clear();
                            self.session_memory.add("system", &summary);
                            messages = self.build_messages();
                            self.inject_memories(&mut messages);
                        }
                        Err(e) => {
                            tracing::warn!("Compaction failed: {}", e);
                        }
                    }
                }
            }

            let request = ChatRequest {
                model: self.model_name.clone(),
                messages: messages.clone(),
                stream: false,
                temperature: Some(0.7),
                max_tokens: Some(4096),
                tools: if loop_count == 1 { Some(tool_defs.clone()) } else { None },
            };

            let response = self.provider.chat(request).await?;
            let model_tool_calls = response.tool_calls;
            let reasoning = response.reasoning_content;

            if !model_tool_calls.is_empty() {
                if reasoning.is_some() {
                    messages.push(ChatMessage::assistant_tool_calls_with_reasoning(
                        model_tool_calls.clone(), reasoning));
                } else {
                    messages.push(ChatMessage::assistant_tool_calls(model_tool_calls.clone()));
                }

                let tool_names: Vec<&str> = model_tool_calls.iter().map(|t| t.name()).collect();
                task_log.push_str(&format!("🔧 {}\n", tool_names.join(", ")));

                let event = Frame::event("agent", serde_json::to_value(AgentEventPayload {
                    stream: "assistant".into(), data: AgentEventData {
                        text: Some(format!("🔧 Calling tools: {}", tool_names.join(", "))),
                        finish_reason: None, tool_calls: None,
                    },
                })?);
                let _ = sender.send(event.to_text());

                for tc in &model_tool_calls {
                    match self.tools.dispatch(tc.name(), tc.arguments_json()).await {
                        Ok(result) => {
                            let summary = if result.len() > 200 {
                                format!("{}...", &result[..200])
                            } else { result.clone() };
                            messages.push(ChatMessage::tool_result(&tc.id, &result));
                            task_log.push_str(&format!("  ✓ {} → {}\n", tc.name(), summary));

                            let event = Frame::event("agent", serde_json::to_value(AgentEventPayload {
                                stream: "tool".into(), data: AgentEventData {
                                    text: Some(format!("\n  ✓ {} → {}", tc.name(), summary)),
                                    finish_reason: None, tool_calls: None,
                                },
                            })?);
                            let _ = sender.send(event.to_text());
                        }
                        Err(e) => {
                            messages.push(ChatMessage::tool_result(&tc.id, format!("Error: {}", e)));
                            task_log.push_str(&format!("  ✗ {} → {}\n", tc.name(), e));

                            let event = Frame::event("agent", serde_json::to_value(AgentEventPayload {
                                stream: "tool".into(), data: AgentEventData {
                                    text: Some(format!("\n  ✗ {} → Error: {}", tc.name(), e)),
                                    finish_reason: None, tool_calls: None,
                                },
                            })?);
                            let _ = sender.send(event.to_text());
                        }
                    }
                }
                continue;
            }

            let final_text = response.content;
            let end_event = Frame::event("agent", serde_json::to_value(AgentEventPayload {
                stream: "assistant".into(), data: AgentEventData {
                    text: Some(final_text.clone()),
                    finish_reason: Some("stop".into()), tool_calls: None,
                },
            })?);
            let _ = sender.send(end_event.to_text());

            self.session_memory.add("assistant", &final_text);
            self.save_chat("assistant", &final_text);

            if self.auto_refine && !task_log.is_empty() {
                self.try_refine_skill(&task_log).await;
            }

            break;
        }

        Ok(())
    }

    async fn run_loop_sync(&mut self) -> anyhow::Result<String> {
        let mut messages = self.build_messages();
        self.inject_memories(&mut messages);
        let tool_defs = self.tools.definitions();
        let mut full_response = String::new();
        let mut task_log = String::new();
        let mut loop_count = 0;

        loop {
            if loop_count >= MAX_TOOL_LOOPS {
                full_response.push_str("\n\n⚠ Max tool iterations reached.");
                break;
            }
            loop_count += 1;

            if self.auto_compaction {
                let tokens = self.session_memory.estimated_tokens();
                if tokens > self.compaction_threshold {
                    full_response.push_str("\n📝 Compacting...");
                    let compactor = Compactor::new(self.provider.as_ref());
                    let records = self.session_memory.all_records();
                    match compactor.compact(&records).await {
                        Ok(summary) => {
                            self.session_memory.clear();
                            self.session_memory.add("system", &summary);
                            messages = self.build_messages();
                            self.inject_memories(&mut messages);
                        }
                        Err(e) => {
                            tracing::warn!("Compaction failed: {}", e);
                        }
                    }
                }
            }

            let request = ChatRequest {
                model: self.model_name.clone(),
                messages: messages.clone(),
                stream: false,
                temperature: Some(0.7),
                max_tokens: Some(4096),
                tools: if loop_count == 1 { Some(tool_defs.clone()) } else { None },
            };

            let response = self.provider.chat(request).await?;
            let model_tool_calls = response.tool_calls;
            let reasoning = response.reasoning_content;

            if !model_tool_calls.is_empty() {
                let tool_names: Vec<&str> = model_tool_calls.iter().map(|t| t.name()).collect();
                full_response.push_str(&format!("\n🔧 {}", tool_names.join(", ")));
                task_log.push_str(&format!("🔧 {}\n", tool_names.join(", ")));

                if reasoning.is_some() {
                    messages.push(ChatMessage::assistant_tool_calls_with_reasoning(
                        model_tool_calls.clone(), reasoning));
                } else {
                    messages.push(ChatMessage::assistant_tool_calls(model_tool_calls.clone()));
                }

                for tc in &model_tool_calls {
                    match self.tools.dispatch(tc.name(), tc.arguments_json()).await {
                        Ok(result) => {
                            let summary = if result.len() > 300 {
                                format!("{}...", &result[..300])
                            } else { result.clone() };
                            messages.push(ChatMessage::tool_result(&tc.id, &result));
                            full_response.push_str(&format!("\n  ✓ {} → {}", tc.name(), summary));
                            task_log.push_str(&format!("  ✓ {} → {}\n", tc.name(), summary));
                        }
                        Err(e) => {
                            let err = format!("Error: {}", e);
                            messages.push(ChatMessage::tool_result(&tc.id, &err));
                            full_response.push_str(&format!("\n  ✗ {} → {}", tc.name(), e));
                            task_log.push_str(&format!("  ✗ {} → {}\n", tc.name(), e));
                        }
                    }
                }
                full_response.push_str("\n");
                continue;
            }

            if !response.content.is_empty() {
                full_response.push_str(&response.content);
            }
            break;
        }

        self.session_memory.add("assistant", &full_response);
        self.save_chat("assistant", &full_response);

        if self.auto_refine && !task_log.is_empty() {
            self.try_refine_skill(&task_log).await;
        }

        Ok(full_response)
    }

    async fn try_refine_skill(&mut self, task_log: &str) {
        let refiner = SkillRefiner::new(self.provider.as_ref());
        match refiner.refine(task_log).await {
            Ok(skill) => {
                if skill.name != "unnamed_skill" {
                    let def = serde_json::to_string(&skill).unwrap_or_default();
                    if let Some(ref db) = self.db {
                        let _ = db.upsert_skill(&skill.name, &def, None);
                    }
                    if let Some(ref vs) = self.vector_store {
                        let _ = vs.insert(&skill.name, &skill.description);
                    }
                    tracing::info!("Skill refined: {}", skill.name);
                }
            }
            Err(e) => {
                tracing::warn!("Skill refine failed: {}", e);
            }
        }
    }

    fn build_messages(&self) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        messages.push(ChatMessage::system(&self.system_prompt));

        for m in self.session_memory.messages() {
            if m.role == "system" && messages.len() == 1 {
                continue;
            }
            messages.push(ChatMessage {
                role: m.role.clone(),
                content: Some(m.content.clone()),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            });
        }

        messages
    }
}

use crate::gateway::{Frame, AgentEventPayload, AgentEventData};
use crate::model::{ChatProvider, ChatRequest, ChatMessage, ToolCall as ModelToolCall};
use crate::memory::SessionMemory;
use crate::agent::tool_use::ToolRegistry;
use crate::agent::system_prompt::build_system_prompt;
use crate::context::ProjectContext;

const MAX_TOOL_LOOPS: usize = 10;

pub struct AgentRuntime {
    session_memory: SessionMemory,
    provider: Box<dyn ChatProvider>,
    model_name: String,
    tools: ToolRegistry,
    system_prompt: String,
    max_session_messages: usize,
    compaction_threshold: usize,
    auto_compaction: bool,
}

impl AgentRuntime {
    pub fn new(
        provider: Box<dyn ChatProvider>,
        model_name: String,
        tools: ToolRegistry,
    ) -> Self {
        let ctx = ProjectContext::load_from(".").unwrap_or_default();
        let system_prompt = build_system_prompt(&ctx, &tools);

        Self {
            session_memory: SessionMemory::new(100),
            provider,
            model_name,
            tools,
            system_prompt,
            max_session_messages: 100,
            compaction_threshold: 64000,
            auto_compaction: true,
        }
    }

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
        }
    }

    pub async fn handle_message(
        &mut self,
        message: &str,
        sender: tokio::sync::broadcast::Sender<String>,
    ) -> anyhow::Result<()> {
        self.session_memory.add("user", message);

        let mut messages = self.build_messages();
        let tool_defs = self.tools.definitions();

        let mut loop_count = 0;
        loop {
            if loop_count >= MAX_TOOL_LOOPS {
                let err = Frame::event("error", serde_json::json!({
                    "message": "Max tool loop iterations reached"
                }));
                let _ = sender.send(err.to_text());
                break;
            }
            loop_count += 1;

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
                    messages.push(ChatMessage::assistant_tool_calls_with_reasoning(model_tool_calls.clone(), reasoning));
                } else {
                    messages.push(ChatMessage::assistant_tool_calls(model_tool_calls.clone()));
                }

                let event = Frame::event("agent", serde_json::to_value(AgentEventPayload {
                    stream: "assistant".into(),
                    data: AgentEventData {
                        text: Some(format!("🔧 Calling tools: {}",
                            model_tool_calls.iter().map(|t| t.name()).collect::<Vec<_>>().join(", "))),
                        finish_reason: None,
                        tool_calls: None,
                    },
                })?);
                let _ = sender.send(event.to_text());

                for tc in &model_tool_calls {
                    match self.tools.dispatch(tc.name(), tc.arguments_json()).await {
                        Ok(result) => {
                            messages.push(ChatMessage::tool_result(&tc.id, &result));

                            let event = Frame::event("agent", serde_json::to_value(AgentEventPayload {
                                stream: "tool".into(),
                                data: AgentEventData {
                                    text: Some(format!("\n  ✓ {} => {}", tc.name(),
                                        if result.len() > 200 { format!("{}...", &result[..200]) } else { result.clone() })),
                                    finish_reason: None,
                                    tool_calls: None,
                                },
                            })?);
                            let _ = sender.send(event.to_text());
                        }
                        Err(e) => {
                            messages.push(ChatMessage::tool_result(&tc.id, format!("Error: {}", e)));

                            let event = Frame::event("agent", serde_json::to_value(AgentEventPayload {
                                stream: "tool".into(),
                                data: AgentEventData {
                                    text: Some(format!("\n  ✗ {} => Error: {}", tc.name(), e)),
                                    finish_reason: None,
                                    tool_calls: None,
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
                stream: "assistant".into(),
                data: AgentEventData {
                    text: Some(final_text.clone()),
                    finish_reason: Some("stop".into()),
                    tool_calls: None,
                },
            })?);
            let _ = sender.send(end_event.to_text());

            self.session_memory.add("assistant", &final_text);
            break;
        }

        Ok(())
    }

    pub async fn handle_message_sync(&mut self, message: &str) -> anyhow::Result<String> {
        self.session_memory.add("user", message);

        let tool_defs = self.tools.definitions();

        let mut full_response = String::new();
        let mut messages = self.build_messages();

        let mut loop_count = 0;
        loop {
            if loop_count >= MAX_TOOL_LOOPS {
                full_response.push_str("\n\n⚠ Max tool iterations reached.");
                break;
            }
            loop_count += 1;

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
                let tool_names: Vec<_> = model_tool_calls.iter().map(|t| t.name().to_string()).collect();
                full_response.push_str(&format!("\n🔧 {}", tool_names.join(", ")));

                if reasoning.is_some() {
                    messages.push(ChatMessage::assistant_tool_calls_with_reasoning(model_tool_calls.clone(), reasoning));
                } else {
                    messages.push(ChatMessage::assistant_tool_calls(model_tool_calls.clone()));
                }

                let mut all_results = Vec::new();
                for tc in &model_tool_calls {
                    match self.tools.dispatch(tc.name(), tc.arguments_json()).await {
                        Ok(result) => {
                            let summary = if result.len() > 300 {
                                format!("{}...", &result[..300])
                            } else {
                                result.clone()
                            };
                            messages.push(ChatMessage::tool_result(&tc.id, &result));
                            full_response.push_str(&format!("\n  ✓ {} → {}", tc.name(), summary));
                            all_results.push(format!("{} result: {}", tc.name(), result));
                        }
                        Err(e) => {
                            let err = format!("Error: {}", e);
                            messages.push(ChatMessage::tool_result(&tc.id, &err));
                            full_response.push_str(&format!("\n  ✗ {} → {}", tc.name(), e));
                            all_results.push(format!("{} error: {}", tc.name(), e));
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

        Ok(full_response)
    }

    fn build_messages(&self) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        messages.push(ChatMessage::system(&self.system_prompt));

        for m in self.session_memory.messages() {
            if m.role == "system" {
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

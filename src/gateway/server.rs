use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tokio::net::TcpListener;
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::tungstenite::Message;

use super::session::SessionManager;
use super::protocol::{Frame, ChatSendParams};
use crate::agent::AgentRuntime;
use crate::config::RsClawConfig;

pub struct GatewayServer {
    config: RsClawConfig,
    session_manager: Arc<SessionManager>,
}

impl GatewayServer {
    pub fn new(config: RsClawConfig) -> Self {
        Self {
            config,
            session_manager: Arc::new(SessionManager::new()),
        }
    }

    pub async fn run(&self, port: u16) -> anyhow::Result<()> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&addr).await?;
        tracing::info!("Gateway listening on ws://{}", addr);

        loop {
            let (stream, peer) = listener.accept().await?;
            tracing::info!("New connection from {}", peer);

            let config = self.config.clone();
            let sessions = self.session_manager.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, config, sessions).await {
                    tracing::error!("Connection error from {}: {}", peer, e);
                }
            });
        }
    }

    async fn handle_connection(
        stream: tokio::net::TcpStream,
        config: RsClawConfig,
        sessions: Arc<SessionManager>,
    ) -> anyhow::Result<()> {
        let ws_stream = tokio_tungstenite::accept_async(stream).await?;
        let (ws_write, mut ws_read) = ws_stream.split();
        let ws_write = Arc::new(Mutex::new(ws_write));

        let session_key = sessions.create().await;
        tracing::info!("Session created: {}", session_key);

        let provider = crate::model::create_provider(
            &config.model.provider,
            &config.model.api_key,
            Some(&config.model.model),
            config.model.base_url.as_deref(),
        )?;

        let agent = Arc::new(Mutex::new(
            AgentRuntime::with_config(
                provider,
                config.model.model.clone(),
                crate::tools::build_registry(),
                config.memory.max_session_messages,
                config.memory.compaction_threshold,
                config.memory.auto_compaction,
            )
        ));

        while let Some(msg) = ws_read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let text_str = text.to_string();
                    if let Ok(frame) = Frame::from_json(&text_str) {
                        let write = ws_write.clone();
                        let agent = agent.clone();
                        let session_key = session_key.clone();
                        let sessions = sessions.clone();

                        tokio::spawn(async move {
                            Self::dispatch_frame(frame, write, agent, session_key, sessions).await;
                        });
                    }
                }
                Ok(Message::Ping(data)) => {
                    let mut write = ws_write.lock().await;
                    let _ = write.send(Message::Pong(data)).await;
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Client disconnected, session: {}", session_key);
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn dispatch_frame(
        frame: Frame,
        write: Arc<Mutex<futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
            Message,
        >>>,
        agent: Arc<Mutex<AgentRuntime>>,
        session_key: String,
        sessions: Arc<SessionManager>,
    ) {
        match &frame {
            Frame::Request { id, method, params } => match method.as_str() {
                "chat.send" => {
                    let chat_params: ChatSendParams = serde_json::from_value(params.clone())
                        .unwrap_or(ChatSendParams {
                            session_key: Some(session_key.clone()),
                            message: String::new(),
                            idempotency_key: None,
                        });

                    if chat_params.message.is_empty() {
                        let resp = Frame::response(id, false, None, Some("message is required".into()));
                        let mut w = write.lock().await;
                        let _ = w.send(Message::Text(resp.to_text().into())).await;
                        return;
                    }

                    let (tx, mut rx) = broadcast::channel::<String>(64);

                    {
                        let mut agent = agent.lock().await;
                        if let Err(e) = agent.handle_message(&chat_params.message, tx).await {
                            let err_frame = Frame::response(
                                id,
                                false,
                                None,
                                Some(format!("Agent error: {}", e)),
                            );
                            let mut w = write.lock().await;
                            let _ = w.send(Message::Text(err_frame.to_text().into())).await;
                            return;
                        }
                    }

                    let ack = Frame::response(id, true, Some(serde_json::json!({
                        "session_key": session_key
                    })), None);
                    {
                        let mut w = write.lock().await;
                        let _ = w.send(Message::Text(ack.to_text().into())).await;
                    }

                    while let Ok(event_text) = rx.recv().await {
                        let mut w = write.lock().await;
                        let _ = w.send(Message::Text(event_text.into())).await;
                    }

                    if let Some(session) = sessions.get(&session_key).await {
                        let mut s = session.write().await;
                        s.message_count += 1;
                    }
                }
                "health" => {
                    let resp = Frame::response(id, true, Some(serde_json::json!({
                        "status": "ok",
                        "session_key": session_key
                    })), None);
                    let mut w = write.lock().await;
                    let _ = w.send(Message::Text(resp.to_text().into())).await;
                }
                _ => {
                    let resp = Frame::response(
                        id,
                        false,
                        None,
                        Some(format!("Unknown method: {}", method)),
                    );
                    let mut w = write.lock().await;
                    let _ = w.send(Message::Text(resp.to_text().into())).await;
                }
            },
            _ => {}
        }
    }
}

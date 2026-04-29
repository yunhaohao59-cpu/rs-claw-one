use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Frame {
    #[serde(rename = "req")]
    Request {
        id: String,
        method: String,
        #[serde(default)]
        params: serde_json::Value,
    },
    #[serde(rename = "res")]
    Response {
        id: String,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "event")]
    Event {
        event: String,
        #[serde(default)]
        payload: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEventPayload {
    pub stream: String,
    pub data: AgentEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEventData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSendParams {
    #[serde(default)]
    pub session_key: Option<String>,
    pub message: String,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

impl Frame {
    pub fn request(id: impl Into<String>, method: impl Into<String>, params: serde_json::Value) -> Self {
        Frame::Request {
            id: id.into(),
            method: method.into(),
            params,
        }
    }

    pub fn response(id: impl Into<String>, ok: bool, payload: Option<serde_json::Value>, error: Option<String>) -> Self {
        Frame::Response {
            id: id.into(),
            ok,
            payload,
            error,
        }
    }

    pub fn event(event: impl Into<String>, payload: serde_json::Value) -> Self {
        Frame::Event {
            event: event.into(),
            payload,
        }
    }

    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    pub fn to_text(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Session {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub message_count: usize,
}

pub struct SessionManager {
    sessions: RwLock<HashMap<String, Arc<RwLock<Session>>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create(&self) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let session = Session {
            id: id.clone(),
            created_at: chrono::Utc::now(),
            message_count: 0,
        };
        self.sessions.write().await.insert(id.clone(), Arc::new(RwLock::new(session)));
        id
    }

    pub async fn get(&self, id: &str) -> Option<Arc<RwLock<Session>>> {
        self.sessions.read().await.get(id).cloned()
    }
}

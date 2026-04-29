use std::sync::Arc;

pub struct MessageRouter {
    session_manager: Arc<super::SessionManager>,
}

impl MessageRouter {
    pub fn new(session_manager: Arc<super::SessionManager>) -> Self {
        Self { session_manager }
    }
}

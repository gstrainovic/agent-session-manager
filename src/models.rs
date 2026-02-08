use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub size: u64,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String, // "user" or "assistant"
    pub content: String,
}

impl Session {
    pub fn new(id: String, project_path: String) -> Self {
        Self {
            id,
            project_path,
            created_at: chrono::Local::now().to_rfc3339(),
            updated_at: chrono::Local::now().to_rfc3339(),
            size: 0,
            messages: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("test-id".to_string(), "/test/path".to_string());
        assert_eq!(session.id, "test-id");
        assert_eq!(session.project_path, "/test/path");
        assert_eq!(session.messages.len(), 0);
    }
}

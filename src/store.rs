// src/store.rs

use crate::models::{Session, Message};
use anyhow::{Result, Context};
use std::fs;
use std::path::{Path, PathBuf};

pub struct SessionStore {
    projects_path: PathBuf,
}

impl SessionStore {
    pub fn new() -> Self {
        let projects_path = dirs::home_dir()
            .expect("home dir")
            .join(".claude/projects");
        
        Self { projects_path }
    }

    pub fn load_sessions(&self) -> Result<Vec<Session>> {
        let mut sessions = Vec::new();
        
        if !self.projects_path.exists() {
            return Ok(sessions);
        }

        for entry in fs::read_dir(&self.projects_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Ok(session) = self.load_session_from_dir(&path) {
                    sessions.push(session);
                }
            }
        }
        
        Ok(sessions)
    }

    fn load_session_from_dir(&self, path: &Path) -> Result<Session> {
        let session_id = path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .context("invalid session dir")?;

        let jsonl_path = path.join(format!("{}.jsonl", session_id));
        
        let mut session = Session::new(
            session_id,
            path.to_string_lossy().to_string(),
        );

        if jsonl_path.exists() {
            let content = fs::read_to_string(&jsonl_path)?;
            for line in content.lines() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                    // Parse user and assistant messages
                    if let Some(content) = json.get("content").and_then(|c| c.as_str()) {
                        let role = json.get("role")
                            .and_then(|r| r.as_str())
                            .unwrap_or("unknown");
                        
                        session.messages.push(Message {
                            role: role.to_string(),
                            content: content.to_string(),
                        });
                    }
                }
            }
        }

        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_sessions_from_projects() {
        let store = SessionStore::new();
        let sessions = store.load_sessions().unwrap();
        // Will load whatever exists
        let _ = sessions;
    }
}

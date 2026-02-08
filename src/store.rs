use crate::models::{parse_jsonl_messages, Session};
use anyhow::{Context, Result};
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

    #[cfg(test)]
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            projects_path: path,
        }
    }

    pub fn load_sessions(&self) -> Result<Vec<Session>> {
        let mut sessions = Vec::new();

        if !self.projects_path.exists() {
            return Ok(sessions);
        }

        for project_entry in fs::read_dir(&self.projects_path)? {
            let project_entry = project_entry?;
            let project_path = project_entry.path();

            if !project_path.is_dir() {
                continue;
            }

            let project_name = project_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            for file_entry in fs::read_dir(&project_path)? {
                let file_entry = file_entry?;
                let file_path = file_entry.path();

                if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }

                if let Ok(session) =
                    self.load_session_from_jsonl(&file_path, &project_name)
                {
                    sessions.push(session);
                }
            }
        }

        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(sessions)
    }

    fn load_session_from_jsonl(&self, path: &Path, project_name: &str) -> Result<Session> {
        let session_id = path
            .file_stem()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .context("invalid jsonl filename")?;

        let metadata = fs::metadata(path)?;
        let file_size = metadata.len();

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| {
                let datetime: chrono::DateTime<chrono::Local> = t.into();
                Some(datetime.to_rfc3339())
            })
            .unwrap_or_default();

        let created = metadata
            .created()
            .ok()
            .and_then(|t| {
                let datetime: chrono::DateTime<chrono::Local> = t.into();
                Some(datetime.to_rfc3339())
            })
            .unwrap_or_default();

        let content = fs::read_to_string(path)?;
        let messages = parse_jsonl_messages(&content);

        Ok(Session {
            id: session_id,
            project_path: path.to_string_lossy().to_string(),
            project_name: project_name.to_string(),
            created_at: created,
            updated_at: modified,
            size: file_size,
            messages,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_store() -> (TempDir, SessionStore) {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::with_path(tmp.path().to_path_buf());
        (tmp, store)
    }

    #[test]
    fn test_empty_dir_returns_empty() {
        let (tmp, store) = create_test_store();
        fs::create_dir_all(tmp.path()).unwrap();
        let sessions = store.load_sessions().unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_nonexistent_dir_returns_empty() {
        let store = SessionStore::with_path(PathBuf::from("/tmp/nonexistent-test-dir-xyz"));
        let sessions = store.load_sessions().unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_loads_sessions_from_project_subdirs() {
        let (tmp, store) = create_test_store();

        let project_dir = tmp.path().join("-home-g-myproject");
        fs::create_dir_all(&project_dir).unwrap();

        let jsonl_content = r#"{"type":"user","message":{"role":"user","content":"hello"},"uuid":"a1"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"hi there"}]},"uuid":"a2"}
{"type":"progress","data":{}}
"#;

        let mut f = fs::File::create(project_dir.join("abc-123.jsonl")).unwrap();
        f.write_all(jsonl_content.as_bytes()).unwrap();

        let sessions = store.load_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "abc-123");
        assert_eq!(sessions[0].project_name, "-home-g-myproject");
        assert_eq!(sessions[0].messages.len(), 2);
        assert_eq!(sessions[0].messages[0].role, "user");
        assert_eq!(sessions[0].messages[0].content, "hello");
        assert_eq!(sessions[0].messages[1].role, "assistant");
        assert_eq!(sessions[0].messages[1].content, "hi there");
    }

    #[test]
    fn test_multiple_sessions_in_one_project() {
        let (tmp, store) = create_test_store();

        let project_dir = tmp.path().join("-home-g-project");
        fs::create_dir_all(&project_dir).unwrap();

        let line = r#"{"type":"user","message":{"role":"user","content":"msg"},"uuid":"x"}"#;

        fs::write(project_dir.join("session-1.jsonl"), line).unwrap();
        fs::write(project_dir.join("session-2.jsonl"), line).unwrap();

        let sessions = store.load_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_skips_non_jsonl_files() {
        let (tmp, store) = create_test_store();

        let project_dir = tmp.path().join("-home-g-project");
        fs::create_dir_all(&project_dir).unwrap();

        fs::write(project_dir.join("not-a-session.txt"), "hello").unwrap();
        fs::create_dir_all(project_dir.join("some-subdir")).unwrap();

        let sessions = store.load_sessions().unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_sessions_sorted_by_updated_at_desc() {
        let (tmp, store) = create_test_store();

        let project_dir = tmp.path().join("-home-g-project");
        fs::create_dir_all(&project_dir).unwrap();

        let line = r#"{"type":"user","message":{"role":"user","content":"msg"},"uuid":"x"}"#;
        fs::write(project_dir.join("old-session.jsonl"), line).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));

        fs::write(project_dir.join("new-session.jsonl"), line).unwrap();

        let sessions = store.load_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, "new-session");
        assert_eq!(sessions[1].id, "old-session");
    }
}

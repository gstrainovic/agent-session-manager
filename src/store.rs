use crate::models::{parse_jsonl_messages, Session};
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

/// Converts a Claude project slug (e.g. "-home-g-agent-session-manager") back to
/// the actual filesystem path (e.g. "/home/g/agent-session-manager").
/// Uses a greedy algorithm trying longest directory segments first.
fn slug_to_path(slug: &str) -> Option<PathBuf> {
    let slug = slug.strip_prefix('-').unwrap_or(slug);
    let parts: Vec<&str> = slug.split('-').collect();

    let mut path = PathBuf::from("/");
    let mut i = 0;

    while i < parts.len() {
        let mut found = false;
        // Try longest segment combination first (greedy)
        for j in (i + 1..=parts.len()).rev() {
            let candidate = parts[i..j].join("-");
            let test_path = path.join(&candidate);
            if test_path.is_dir() {
                path = test_path;
                i = j;
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
    }

    Some(path)
}

pub struct SessionStore {
    projects_path: PathBuf,
    trash_path: PathBuf,
}

impl SessionStore {
    pub fn new() -> Self {
        let base = if let Ok(dir) = std::env::var("CLAUDE_DATA_DIR") {
            PathBuf::from(dir)
        } else {
            dirs::home_dir().expect("home dir").join(".claude")
        };
        let projects_path = base.join("projects");
        let trash_path = base.join("trash.json");
        Self {
            projects_path,
            trash_path,
        }
    }

    #[cfg(test)]
    pub fn with_path(path: PathBuf) -> Self {
        let trash_path = path.join("trash.json");
        Self {
            projects_path: path,
            trash_path,
        }
    }

    #[allow(dead_code)] // Used in integration tests (tests/integration.rs)
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

            let project_slug = project_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let resolved_path = slug_to_path(&project_slug)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| project_slug.clone());

            for file_entry in fs::read_dir(&project_path)? {
                let file_entry = file_entry?;
                let file_path = file_entry.path();

                if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }

                if let Ok(session) =
                    self.load_session_from_jsonl(&file_path, &project_slug, &resolved_path)
                {
                    sessions.push(session);
                }
            }
        }

        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(sessions)
    }

    pub fn count_session_files(&self) -> usize {
        if !self.projects_path.exists() {
            return 0;
        }

        let mut count = 0;
        if let Ok(entries) = fs::read_dir(&self.projects_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(files) = fs::read_dir(&path) {
                        for file in files.flatten() {
                            if file.path().extension().and_then(|e| e.to_str()) == Some("jsonl") {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        count
    }

    pub fn load_sessions_with_progress<F>(&self, mut on_progress: F) -> Result<Vec<Session>>
    where
        F: FnMut(usize, usize),
    {
        if !self.projects_path.exists() {
            return Ok(Vec::new());
        }

        let total = self.count_session_files();

        let file_paths: Vec<(PathBuf, String, String)> = {
            let mut paths = Vec::new();
            for project_entry in fs::read_dir(&self.projects_path)? {
                let project_entry = project_entry?;
                let project_path = project_entry.path();

                if !project_path.is_dir() {
                    continue;
                }

                let project_slug = project_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let resolved_path = slug_to_path(&project_slug)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| project_slug.clone());

                for file_entry in fs::read_dir(&project_path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if file_path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                        paths.push((file_path, project_slug.clone(), resolved_path.clone()));
                    }
                }
            }
            paths
        };

        let sessions: Vec<Session> = file_paths
            .into_par_iter()
            .filter_map(|(file_path, project_slug, resolved_path)| {
                self.load_session_from_jsonl(&file_path, &project_slug, &resolved_path)
                    .ok()
            })
            .collect();

        on_progress(total, total);

        Ok(sessions)
    }

    fn load_session_from_jsonl(
        &self,
        path: &Path,
        project_name: &str,
        project_path: &str,
    ) -> Result<Session> {
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
        let total_entries = crate::models::count_jsonl_entries(&content);
        let messages = parse_jsonl_messages(&content);

        Ok(Session {
            id: session_id,
            project_path: project_path.to_string(),
            project_name: project_name.to_string(),
            created_at: created,
            updated_at: modified,
            size: file_size,
            total_entries,
            messages,
            original_content: Some(content),
        })
    }

    pub fn load_trash(&self) -> Result<Vec<Session>> {
        if !self.trash_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.trash_path)?;
        let sessions: Vec<Session> = serde_json::from_str(&content)?;
        Ok(sessions)
    }

    pub fn save_trash(&self, trash: &[Session]) -> Result<()> {
        let json = serde_json::to_string_pretty(trash)?;
        fs::write(&self.trash_path, json)?;
        Ok(())
    }

    /// Returns the path to a session's JSONL file based on project_name and id
    pub fn get_session_file_path(&self, project_name: &str, session_id: &str) -> PathBuf {
        self.projects_path
            .join(project_name)
            .join(format!("{}.jsonl", session_id))
    }

    /// Deletes a session's JSONL file from the projects directory
    pub fn delete_session_file(&self, project_name: &str, session_id: &str) -> Result<()> {
        let path = self.get_session_file_path(project_name, session_id);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    pub fn restore_session_file(&self, session: &Session) -> Result<()> {
        if let Some(ref content) = session.original_content {
            let path = self.get_session_file_path(&session.project_name, &session.id);
            fs::create_dir_all(path.parent().unwrap())?;
            fs::write(&path, content)?;
        }
        Ok(())
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

    #[test]
    fn test_count_session_files() {
        let (tmp, store) = create_test_store();

        let p1 = tmp.path().join("project-a");
        let p2 = tmp.path().join("project-b");
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&p2).unwrap();

        let line = r#"{"type":"user","message":{"role":"user","content":"msg"},"uuid":"x"}"#;
        fs::write(p1.join("s1.jsonl"), line).unwrap();
        fs::write(p1.join("s2.jsonl"), line).unwrap();
        fs::write(p2.join("s3.jsonl"), line).unwrap();
        fs::write(p2.join("not-a-session.txt"), "ignore").unwrap();

        assert_eq!(store.count_session_files(), 3);
    }

    #[test]
    fn test_load_sessions_with_progress() {
        let (tmp, store) = create_test_store();

        let p1 = tmp.path().join("project-a");
        fs::create_dir_all(&p1).unwrap();

        let line = r#"{"type":"user","message":{"role":"user","content":"msg"},"uuid":"x"}"#;
        fs::write(p1.join("s1.jsonl"), line).unwrap();
        fs::write(p1.join("s2.jsonl"), line).unwrap();

        let mut progress_calls = Vec::new();
        let sessions = store
            .load_sessions_with_progress(|loaded, total| {
                progress_calls.push((loaded, total));
            })
            .unwrap();

        assert_eq!(sessions.len(), 2);
        assert_eq!(progress_calls.len(), 1);
        assert_eq!(progress_calls[0], (2, 2));
    }

    #[test]
    fn test_slug_to_path_resolves_home() {
        // "-home-g" should resolve to /home/g if it exists
        let result = slug_to_path("-home-g");
        if PathBuf::from("/home/g").is_dir() {
            assert_eq!(result, Some(PathBuf::from("/home/g")));
        }
    }

    #[test]
    fn test_slug_to_path_returns_none_for_nonexistent() {
        let result = slug_to_path("-nonexistent-path-xyz123");
        assert_eq!(result, None);
    }

    #[test]
    fn test_slug_to_path_handles_empty() {
        let result = slug_to_path("");
        // Empty slug with no parts should resolve to "/"
        assert_eq!(result, Some(PathBuf::from("/")));
    }

    #[test]
    fn test_delete_session_file_removes_from_load() {
        let (tmp, store) = create_test_store();

        let project_dir = tmp.path().join("-home-g-myproject");
        fs::create_dir_all(&project_dir).unwrap();

        let line = r#"{"type":"user","message":{"role":"user","content":"msg"},"uuid":"x"}"#;
        fs::write(project_dir.join("test-session.jsonl"), line).unwrap();

        let sessions = store.load_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "test-session");

        store
            .delete_session_file("-home-g-myproject", "test-session")
            .unwrap();

        let sessions_after_delete = store.load_sessions().unwrap();
        assert_eq!(sessions_after_delete.len(), 0);
    }

    #[test]
    fn test_delete_session_file_is_idempotent() {
        let (_tmp, store) = create_test_store();

        let result = store.delete_session_file("-home-g-nonexistent", "no-session");
        assert!(result.is_ok());
    }

    #[test]
    fn test_restore_session_file_recreates_jsonl() {
        let (tmp, store) = create_test_store();

        let project_dir = tmp.path().join("-home-g-myproject");
        fs::create_dir_all(&project_dir).unwrap();

        let original_content = r#"{"type":"user","message":{"role":"user","content":"hello"},"uuid":"x"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"hi"}]},"uuid":"y"}"#;
        fs::write(project_dir.join("test-session.jsonl"), original_content).unwrap();

        let sessions = store.load_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        let session = sessions[0].clone();

        store
            .delete_session_file("-home-g-myproject", "test-session")
            .unwrap();
        let sessions_after_delete = store.load_sessions().unwrap();
        assert_eq!(sessions_after_delete.len(), 0);

        store.restore_session_file(&session).unwrap();

        let sessions_after_restore = store.load_sessions().unwrap();
        assert_eq!(sessions_after_restore.len(), 1);
        assert_eq!(sessions_after_restore[0].id, "test-session");
        assert_eq!(sessions_after_restore[0].messages.len(), 2);
    }

    #[test]
    fn test_new_uses_env_var_claude_data_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::env::set_var("CLAUDE_DATA_DIR", tmp.path());
        let store = SessionStore::new();
        assert_eq!(store.projects_path, tmp.path().join("projects"));
        std::env::remove_var("CLAUDE_DATA_DIR");
    }
}

use crate::models::Session;
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Renames a session by appending a custom-title entry to the JSONL file.
/// This matches how Claude Code's /rename command works.
pub fn rename_session(session: &Session, new_name: &str) -> Result<()> {
    let entry = serde_json::json!({
        "type": "custom-title",
        "customTitle": new_name,
        "sessionId": session.id,
    });
    // Ensure newline separator even if file doesn't end with one
    let content = fs::read_to_string(&session.jsonl_path)?;
    let prefix = if content.ends_with('\n') { "" } else { "\n" };
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&session.jsonl_path)?;
    write!(file, "{}{}\n", prefix, entry.to_string())?;
    Ok(())
}

pub fn delete_session(_session: &Session) -> Result<()> {
    let trash_dir = dirs::home_dir().expect("home dir").join(".claude/trash");
    fs::create_dir_all(&trash_dir)?;
    Ok(())
}

pub fn export_session(session: &Session, export_dir: &Path) -> Result<String> {
    fs::create_dir_all(export_dir)?;

    let safe_name = session.project_name.trim_matches('-');
    let filename = format!(
        "{}-{}.md",
        safe_name,
        &session.id[..8.min(session.id.len())]
    );
    let path = export_dir.join(filename);

    let mut file = fs::File::create(&path)?;

    writeln!(file, "# Session: {}", session.display_name())?;
    writeln!(file, "")?;
    writeln!(file, "- **Project:** {}", session.project_name)?;
    writeln!(file, "- **Session ID:** {}", session.id)?;
    writeln!(file, "- **Created:** {}", session.created_at)?;
    writeln!(file, "- **Updated:** {}", session.updated_at)?;
    writeln!(file, "")?;
    writeln!(file, "---")?;
    writeln!(file, "")?;

    for msg in &session.messages {
        let prefix = if msg.role == "user" {
            "## You"
        } else {
            "## Assistant"
        };
        writeln!(file, "{}", prefix)?;
        writeln!(file, "")?;
        writeln!(file, "{}", msg.content)?;
        writeln!(file, "")?;
    }

    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Message;

    fn make_test_session() -> Session {
        Session {
            id: "abc12345-test".to_string(),
            project_path: "/test/path".to_string(),
            project_name: "test-project".to_string(),
            created_at: "2026-01-01T00:00:00".to_string(),
            updated_at: "2026-01-02T00:00:00".to_string(),
            size: 1234,
            total_entries: 2,
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                },
                Message {
                    role: "assistant".to_string(),
                    content: "Hi there".to_string(),
                },
            ],
            jsonl_path: std::path::PathBuf::new(),
            slug: None,
        }
    }

    #[test]
    fn test_delete_session_creates_trash_dir() {
        let session = make_test_session();
        let result = delete_session(&session);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_session_creates_markdown() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let session = make_test_session();
        let result = export_session(&session, tmp.path());
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.contains("test-project-abc12345"));
        assert!(path.ends_with(".md"));

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Session:"));
        assert!(content.contains("Hello"));
        assert!(content.contains("Hi there"));
        assert!(content.contains("## You"));
        assert!(content.contains("## Assistant"));
    }

    #[test]
    fn test_export_filename_strips_leading_dash() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let mut session = make_test_session();
        session.project_name = "-home-g".to_string();
        let path = export_session(&session, tmp.path()).unwrap();
        let filename = std::path::Path::new(&path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        assert!(!filename.starts_with('-'), "filename must not start with '-': {filename}");
        assert!(filename.starts_with("home-g"), "filename should start with 'home-g': {filename}");
    }
}

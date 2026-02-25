#![allow(dead_code)]

use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use tempfile::TempDir;

/// Globaler Mutex verhindert parallele Env-Var-Konflikte zwischen Tests.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Isolierte Testumgebung mit eigenen Verzeichnissen für Sessions, Config und Exports.
/// Hält einen globalen Mutex für die Laufzeit der Testumgebung, damit keine zwei Tests
/// gleichzeitig `CLAUDE_DATA_DIR` / `AGENT_CONFIG_DIR` ändern können.
pub struct TestEnv {
    _lock: MutexGuard<'static, ()>,
    pub _tmp: TempDir,
    pub claude_dir: std::path::PathBuf,
    pub config_dir: std::path::PathBuf,
    pub export_dir: std::path::PathBuf,
}

impl TestEnv {
    pub fn new() -> Self {
        let lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = TempDir::new().expect("TempDir");
        let claude_dir = tmp.path().join("claude");
        let config_dir = tmp.path().join("config");
        let export_dir = tmp.path().join("exports");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::create_dir_all(&export_dir).unwrap();
        Self {
            _lock: lock,
            _tmp: tmp,
            claude_dir,
            config_dir,
            export_dir,
        }
    }

    pub fn activate(&self) {
        std::env::set_var("CLAUDE_DATA_DIR", &self.claude_dir);
        std::env::set_var("AGENT_CONFIG_DIR", &self.config_dir);
    }

    pub fn deactivate() {
        std::env::remove_var("CLAUDE_DATA_DIR");
        std::env::remove_var("AGENT_CONFIG_DIR");
    }
}

/// Erzeugt eine JSONL-Session-Datei in `<claude_dir>/projects/<project_slug>/<session_id>.jsonl`.
pub fn create_fixture_session(
    claude_dir: &Path,
    project_slug: &str,
    session_id: &str,
    messages: &[(&str, &str)],
) {
    let sessions_dir = claude_dir
        .join("projects")
        .join(project_slug);
    std::fs::create_dir_all(&sessions_dir).unwrap();

    let mut lines = Vec::new();
    for (i, (role, content)) in messages.iter().enumerate() {
        let type_ = if *role == "user" { "user" } else { "assistant" };
        let content_json = if *role == "assistant" {
            format!(
                r#"[{{"type":"text","text":"{}"}}]"#,
                content.replace('"', "\\\"")
            )
        } else {
            format!("\"{}\"", content.replace('"', "\\\""))
        };
        lines.push(format!(
            r#"{{"type":"{}","message":{{"role":"{}","content":{}}},"uuid":"test-uuid-{:03}"}}"#,
            type_, role, content_json, i
        ));
    }

    let path = sessions_dir.join(format!("{}.jsonl", session_id));
    std::fs::write(path, lines.join("\n")).unwrap();
}

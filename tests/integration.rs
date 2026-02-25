mod common;

use agent_session_manager::app::App;
use agent_session_manager::commands;
use agent_session_manager::store::SessionStore;
use common::{create_fixture_session, TestEnv};

fn load_sessions(env: &TestEnv) -> Vec<agent_session_manager::models::Session> {
    env.activate();
    let store = SessionStore::new();
    let sessions = store.load_sessions().unwrap_or_default();
    TestEnv::deactivate();
    sessions
}

// ─── READ ────────────────────────────────────────────────────────────────────

#[test]
fn test_read_sessions_from_fixture_dir() {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-test-project",
        "session-uuid-001",
        &[("user", "Hello"), ("assistant", "Hi!")],
    );
    let sessions = load_sessions(&env);

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].messages.len(), 2);
    assert!(sessions[0].project_name.contains("test-project"));
}

#[test]
fn test_search_filters_sessions() {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-alpha-project",
        "uuid-001",
        &[("user", "rust programming")],
    );
    create_fixture_session(
        &env.claude_dir,
        "-beta-project",
        "uuid-002",
        &[("user", "python scripting")],
    );
    let sessions = load_sessions(&env);

    let mut app = App::new(sessions, vec![]);
    app.search_query = "rust".to_string();
    let filtered = app.filtered_sessions();
    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].project_name.contains("alpha"));
}

// ─── UPDATE (Settings) ───────────────────────────────────────────────────────

#[test]
fn test_settings_save_persists_to_config_file() {
    let env = TestEnv::new();
    env.activate();

    let mut app = App::new(vec![], vec![]);
    app.open_settings();
    app.settings_input = env.export_dir.to_string_lossy().to_string();
    app.save_settings();

    TestEnv::deactivate();

    let config_file = env.config_dir.join("config.json");
    assert!(config_file.exists(), "config.json muss erstellt werden");
    let content = std::fs::read_to_string(&config_file).unwrap();
    let cfg: agent_session_manager::config::AppConfig =
        serde_json::from_str(&content).unwrap();
    assert_eq!(cfg.export_path, env.export_dir.to_string_lossy());
}

#[test]
fn test_settings_cancel_does_not_save() {
    let env = TestEnv::new();
    env.activate();

    let mut app = App::new(vec![], vec![]);
    app.config.export_path = "~/original-path".to_string();
    app.open_settings();
    app.settings_input = "~/changed-path".to_string();
    app.cancel_settings();

    TestEnv::deactivate();

    assert_eq!(app.config.export_path, "~/original-path");
    assert!(!env.config_dir.join("config.json").exists());
}

// ─── EXPORT ──────────────────────────────────────────────────────────────────

#[test]
fn test_export_creates_file_in_configured_path() {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-my-project",
        "session-export-001",
        &[("user", "Export this"), ("assistant", "Done!")],
    );
    let sessions = load_sessions(&env);

    let session = &sessions[0];
    let result = commands::export_session(session, &env.export_dir);
    assert!(result.is_ok(), "Export fehlgeschlagen: {:?}", result);

    let files: Vec<_> = env
        .export_dir
        .read_dir()
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(files.len(), 1, "Genau eine Datei muss exportiert werden");
    let content = std::fs::read_to_string(files[0].path()).unwrap();
    assert!(content.contains("Export this"));
    assert!(content.contains("Done!"));
}

#[test]
fn test_export_uses_config_export_path() {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-project-x",
        "uuid-exp",
        &[("user", "test message")],
    );
    let sessions = load_sessions(&env);
    env.activate();

    let mut app = App::new(sessions.clone(), vec![]);
    app.open_settings();
    app.settings_input = env.export_dir.to_string_lossy().to_string();
    app.save_settings();

    let export_path = app.config.resolved_export_path();
    let result = commands::export_session(&sessions[0], &export_path);
    TestEnv::deactivate();

    assert!(result.is_ok());
    assert!(env.export_dir.read_dir().unwrap().count() > 0);
}

// ─── DELETE / TRASH ──────────────────────────────────────────────────────────

#[test]
fn test_delete_moves_session_to_trash() {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-del-project",
        "uuid-del",
        &[("user", "delete me")],
    );
    let sessions = load_sessions(&env);

    let mut app = App::new(sessions, vec![]);
    assert_eq!(app.sessions.len(), 1);
    app.move_selected_to_trash();
    assert_eq!(app.sessions.len(), 0);
    assert_eq!(app.trash.len(), 1);
}

#[test]
fn test_restore_session_from_trash() {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-restore-project",
        "uuid-restore",
        &[("user", "restore me")],
    );
    let sessions = load_sessions(&env);

    let mut app = App::new(sessions, vec![]);
    app.move_selected_to_trash();
    app.switch_tab();
    app.restore_selected_from_trash();
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.trash.len(), 0);
}

#[test]
fn test_empty_trash() {
    let env = TestEnv::new();
    create_fixture_session(&env.claude_dir, "-p1", "uuid-t1", &[("user", "msg1")]);
    create_fixture_session(&env.claude_dir, "-p2", "uuid-t2", &[("user", "msg2")]);
    let sessions = load_sessions(&env);

    let mut app = App::new(sessions, vec![]);
    app.move_selected_to_trash();
    app.select_next();
    app.move_selected_to_trash();
    assert_eq!(app.trash.len(), 2);

    app.switch_tab();
    app.request_empty_trash();
    app.confirm_and_execute();
    assert_eq!(app.trash.len(), 0);
}

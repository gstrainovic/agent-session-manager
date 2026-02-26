mod common;

use agent_session_manager::app::App;
use agent_session_manager::commands;
use agent_session_manager::store::SessionStore;
use common::{create_fixture_session, create_fixture_session_with_title, TestEnv};

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

// ─── CUSTOM TITLE / RENAME ───────────────────────────────────────────────────

#[test]
fn test_custom_title_loaded_from_jsonl() {
    let env = TestEnv::new();
    create_fixture_session_with_title(
        &env.claude_dir,
        "-title-project",
        "uuid-title",
        &[("user", "hello")],
        Some("my-custom-name"),
    );
    let sessions = load_sessions(&env);

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].slug, Some("my-custom-name".to_string()));
}

#[test]
fn test_session_without_custom_title_has_none() {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-no-title",
        "uuid-notitle",
        &[("user", "hello")],
    );
    let sessions = load_sessions(&env);

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].slug, None);
}

#[test]
fn test_search_finds_session_by_custom_title() {
    let env = TestEnv::new();
    create_fixture_session_with_title(
        &env.claude_dir,
        "-proj-a",
        "uuid-a",
        &[("user", "unrelated content")],
        Some("findme-label"),
    );
    create_fixture_session(
        &env.claude_dir,
        "-proj-b",
        "uuid-b",
        &[("user", "other stuff")],
    );
    let sessions = load_sessions(&env);

    let mut app = App::new(sessions, vec![]);
    app.search_query = "findme".to_string();
    let filtered = app.filtered_sessions();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].slug.as_deref(), Some("findme-label"));
}

#[test]
fn test_rename_appends_custom_title_to_jsonl() {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-rename-proj",
        "uuid-rename",
        &[("user", "hello")],
    );
    let sessions = load_sessions(&env);
    let session = &sessions[0];

    assert!(
        session.jsonl_path.exists(),
        "jsonl_path must point to existing file: {:?}",
        session.jsonl_path
    );

    // Rename ausführen
    commands::rename_session(session, "new-name").unwrap();

    // JSONL-Datei prüfen — custom-title Zeile muss angehängt sein
    let content = std::fs::read_to_string(&session.jsonl_path).unwrap();
    assert!(content.contains("custom-title"), "JSONL should contain custom-title entry");
    assert!(content.contains("new-name"), "JSONL should contain the new name");

    // Neu laden — customTitle muss da sein
    let sessions_after = load_sessions(&env);
    assert_eq!(sessions_after[0].slug, Some("new-name".to_string()));
}

#[test]
fn test_rename_prefills_existing_custom_title() {
    let env = TestEnv::new();
    create_fixture_session_with_title(
        &env.claude_dir,
        "-prefill-proj",
        "uuid-prefill",
        &[("user", "hello")],
        Some("existing-title"),
    );
    let sessions = load_sessions(&env);

    let mut app = App::new(sessions, vec![]);
    app.open_rename();
    assert_eq!(app.rename_input, "existing-title");
}

#[test]
fn test_rename_prefills_empty_when_no_title() {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-noprefill",
        "uuid-noprefill",
        &[("user", "hello")],
    );
    let sessions = load_sessions(&env);

    let mut app = App::new(sessions, vec![]);
    app.open_rename();
    assert_eq!(app.rename_input, "");
}

#[test]
fn test_display_name_includes_custom_title() {
    let env = TestEnv::new();
    create_fixture_session_with_title(
        &env.claude_dir,
        "-display-proj",
        "uuid-display",
        &[("user", "hello")],
        Some("my-label"),
    );
    let sessions = load_sessions(&env);

    let name = sessions[0].display_name();
    assert!(name.contains("my-label"), "display_name should contain custom title, got: {}", name);
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

    env.activate();
    let mut app = App::new(sessions, vec![]);
    assert_eq!(app.sessions.len(), 1);
    app.move_selected_to_trash();
    assert_eq!(app.sessions.len(), 0);
    assert_eq!(app.trash.len(), 1);
    TestEnv::deactivate();

    // Dateisystem prüfen — Datei muss im trash-Verzeichnis liegen
    let trash_file = env.claude_dir.join("trash/-del-project/uuid-del.jsonl");
    assert!(trash_file.exists(), "JSONL muss im trash-Verzeichnis liegen: {:?}", trash_file);
    let original = env.claude_dir.join("projects/-del-project/uuid-del.jsonl");
    assert!(!original.exists(), "Original muss verschwunden sein: {:?}", original);
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

    env.activate();
    let mut app = App::new(sessions, vec![]);
    app.move_selected_to_trash();
    app.switch_tab();
    app.restore_selected_from_trash();
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.trash.len(), 0);
    TestEnv::deactivate();

    // Datei ist zurück im projects-Verzeichnis
    let restored_file = env.claude_dir.join("projects/-restore-project/uuid-restore.jsonl");
    assert!(restored_file.exists(), "Datei muss zurück im projects-Verzeichnis sein");
    let trash_file = env.claude_dir.join("trash/-restore-project/uuid-restore.jsonl");
    assert!(!trash_file.exists(), "Datei darf nicht mehr im trash sein");
}

#[test]
fn test_empty_trash() {
    let env = TestEnv::new();
    create_fixture_session(&env.claude_dir, "-p1", "uuid-t1", &[("user", "msg1")]);
    create_fixture_session(&env.claude_dir, "-p2", "uuid-t2", &[("user", "msg2")]);
    let sessions = load_sessions(&env);

    env.activate();
    let mut app = App::new(sessions, vec![]);
    app.move_selected_to_trash();
    app.select_next();
    app.move_selected_to_trash();
    assert_eq!(app.trash.len(), 2);

    app.switch_tab();
    app.request_empty_trash();
    app.confirm_and_execute();
    assert_eq!(app.trash.len(), 0);
    TestEnv::deactivate();

    // Trash-Verzeichnis muss leer oder nicht vorhanden sein
    let trash_dir = env.claude_dir.join("trash");
    assert!(
        !trash_dir.exists() || trash_dir.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true),
        "Trash-Verzeichnis muss leer sein"
    );
}

// ─── MAUS-SIMULIERTE WORKFLOWS (Layer 2) ─────────────────────────────────────
// Layer 2 hat keinen Zugriff auf handle_mouse_event (main.rs).
// Stattdessen rufen wir die App-Methoden auf, die dispatch_click_action aufruft —
// identisch mit dem Maus-Pfad, nur ohne Terminal-Encoding.

#[test]
fn test_mouse_export_click_creates_file() {
    // Simuliert: Mausklick auf "e export" in Command-Bar → Datei wird erstellt
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-mouse-export",
        "uuid-mouse-exp",
        &[("user", "exported via mouse click")],
    );
    let sessions = load_sessions(&env);
    env.activate();

    let mut app = App::new(sessions, vec![]);
    app.config.export_path = env.export_dir.to_string_lossy().to_string();

    // Maus-Click-Aktion: ExportSession (entspricht dispatch_click_action → ExportSession)
    let export_path = app.config.resolved_export_path();
    if let Some(session) = app.get_selected_session() {
        let session_clone = session.clone();
        let result = agent_session_manager::commands::export_session(&session_clone, &export_path);
        assert!(result.is_ok());
    }

    TestEnv::deactivate();

    let files: Vec<_> = env
        .export_dir
        .read_dir()
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(files.len(), 1, "Genau eine Datei muss via Maus-Export erstellt werden");
    let content = std::fs::read_to_string(files[0].path()).unwrap();
    assert!(content.contains("exported via mouse click"));
}

#[test]
fn test_mouse_settings_save_persists_config() {
    // Simuliert: Settings-Modal öffnen, Pfad eingeben, Save-Button klicken
    let env = TestEnv::new();
    env.activate();

    let mut app = App::new(vec![], vec![]);
    // Maus-Click: OpenSettings
    app.open_settings();
    assert!(app.show_settings);
    // Benutzer tippt neuen Pfad
    app.settings_input = env.export_dir.to_string_lossy().to_string();
    // Maus-Click: SaveSettings
    app.save_settings();
    assert!(!app.show_settings, "Modal muss nach Save geschlossen sein");

    TestEnv::deactivate();

    let config_file = env.config_dir.join("config.json");
    assert!(config_file.exists(), "config.json muss nach Maus-Save existieren");
    let content = std::fs::read_to_string(&config_file).unwrap();
    assert!(content.contains(&env.export_dir.to_string_lossy().replace('\\', "\\\\")));
}

#[test]
fn test_mouse_delete_confirm_yes_moves_to_trash() {
    // Simuliert: Session auswählen, d drücken, [y] Maus-Click → Session im Trash
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-mouse-del",
        "uuid-mouse-del",
        &[("user", "delete via mouse")],
    );
    let sessions = load_sessions(&env);
    env.activate();

    let mut app = App::new(sessions, vec![]);
    assert_eq!(app.sessions.len(), 1);

    // Maus-Click: DeleteSession (öffnet Confirmation)
    app.request_delete_confirmation();
    assert!(app.is_confirmation_pending());

    // Maus-Click: ConfirmYes → move_selected_to_trash
    let session = app.get_selected_session().unwrap().clone();
    agent_session_manager::commands::delete_session(&session).unwrap();
    app.move_selected_to_trash();
    app.confirm_action = None;

    assert_eq!(app.sessions.len(), 0);
    assert_eq!(app.trash.len(), 1);
    TestEnv::deactivate();

    let trash_file = env.claude_dir.join("trash/-mouse-del/uuid-mouse-del.jsonl");
    assert!(trash_file.exists(), "Session muss nach Maus-Confirm-Yes im Trash sein");
}

#[test]
fn test_mouse_confirm_no_keeps_session() {
    // Simuliert: [d] delete, dann [n] Maus-Click → Session bleibt
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-mouse-keep",
        "uuid-mouse-keep",
        &[("user", "keep me")],
    );
    let sessions = load_sessions(&env);
    env.activate();

    let mut app = App::new(sessions, vec![]);
    app.request_delete_confirmation();
    assert!(app.is_confirmation_pending());

    // Maus-Click: ConfirmNo
    app.cancel_confirmation();
    assert!(!app.is_confirmation_pending());
    assert_eq!(app.sessions.len(), 1, "Session muss nach Maus-Confirm-No erhalten bleiben");

    TestEnv::deactivate();
}

// Layer 3 E2E PTY-Tests: Starten die echte Binary in einem PTY und interagieren
// per Tastatureingabe. Funktioniert auf Linux/macOS; auf Windows blockiert
// portable-pty's ConPTY-Reader (keine WouldBlock-Semantik), daher deaktiviert.
#![cfg(not(windows))]

mod common;

use common::{create_fixture_session, TestEnv};
use portable_pty::CommandBuilder;
use ratatui_testlib::{KeyCode, Modifiers, TuiTestHarness};
use std::time::Duration;

fn launch(env: &TestEnv) -> ratatui_testlib::Result<TuiTestHarness> {
    let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_agent-session-manager"));
    cmd.env("CLAUDE_DATA_DIR", env.claude_dir.to_str().unwrap());
    cmd.env("AGENT_CONFIG_DIR", env.config_dir.to_str().unwrap());
    let mut harness = TuiTestHarness::builder()
        .with_size(120, 40)
        .with_timeout(Duration::from_secs(10))
        .build()?;
    harness.spawn(cmd)?;
    Ok(harness)
}

// ─── READ ────────────────────────────────────────────────────────────────────

#[test]
fn e2e_shows_sessions_on_start() -> ratatui_testlib::Result<()> {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-my-project",
        "uuid-001",
        &[("user", "Hello World")],
    );

    let mut h = launch(&env)?;
    h.wait_for_text("Sessions")?;
    h.wait_for_text("my-project")?;
    Ok(())
}

#[test]
fn e2e_search_filters_list() -> ratatui_testlib::Result<()> {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-alpha-project",
        "uuid-a",
        &[("user", "rust code")],
    );
    create_fixture_session(
        &env.claude_dir,
        "-beta-project",
        "uuid-b",
        &[("user", "python code")],
    );

    let mut h = launch(&env)?;
    h.wait_for_text("alpha-project")?;

    // Ctrl+F öffnet Suche
    h.send_key_with_modifiers(KeyCode::Char('f'), Modifiers::CTRL)?;
    h.wait_for_text("Search")?;
    h.send_keys("alpha")?;
    h.wait_for_text("alpha-project")?;

    let screen = h.screen_contents();
    assert!(
        !screen.contains("beta-project"),
        "beta-project sollte herausgefiltert sein"
    );
    Ok(())
}

// ─── UPDATE (Settings) ───────────────────────────────────────────────────────

#[test]
fn e2e_settings_change_export_path() -> ratatui_testlib::Result<()> {
    let env = TestEnv::new();

    let mut h = launch(&env)?;
    h.wait_for_text("Sessions")?;

    // Settings öffnen mit 'g'
    h.send_key(KeyCode::Char('g'))?;
    h.wait_for_text("Export Path")?;

    // Default-Pfad löschen (~/claude-exports = 16 Zeichen + Sicherheitspuffer)
    for _ in 0..30 {
        h.send_key(KeyCode::Backspace)?;
    }

    // Neuen Pfad eingeben (forward-slashes für Portabilität)
    let new_path = env
        .export_dir
        .to_string_lossy()
        .replace('\\', "/");
    h.send_keys(&new_path)?;
    h.send_key(KeyCode::Enter)?;
    h.wait_for_text("Settings saved")?;

    let cfg_path = env.config_dir.join("config.json");
    assert!(cfg_path.exists(), "config.json muss nach Save existieren");
    Ok(())
}

// ─── EXPORT ──────────────────────────────────────────────────────────────────

#[test]
fn e2e_export_creates_file() -> ratatui_testlib::Result<()> {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-export-project",
        "uuid-e",
        &[("user", "export this message")],
    );

    // Export-Pfad vorab setzen (über AppConfig direkt)
    {
        std::env::set_var("AGENT_CONFIG_DIR", &env.config_dir);
        let cfg = agent_session_manager::config::AppConfig {
            export_path: env.export_dir.to_string_lossy().to_string(),
        };
        cfg.save().unwrap();
        std::env::remove_var("AGENT_CONFIG_DIR");
    }

    let mut h = launch(&env)?;
    h.wait_for_text("export-project")?;

    h.send_key(KeyCode::Char('e'))?;
    h.wait_for_text("Exported")?;

    let count = env.export_dir.read_dir().unwrap().count();
    assert_eq!(count, 1, "Genau eine Export-Datei erwartet");
    Ok(())
}

// ─── DELETE / TRASH ──────────────────────────────────────────────────────────

#[test]
fn e2e_delete_moves_to_trash_tab() -> ratatui_testlib::Result<()> {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-delete-me",
        "uuid-d",
        &[("user", "to be deleted")],
    );

    let mut h = launch(&env)?;
    h.wait_for_text("delete-me")?;

    // Delete-Bestätigung
    h.send_key(KeyCode::Char('d'))?;
    h.wait_for(|s| s.contains("trash") || s.contains("Trash") || s.contains("Delete"))?;
    h.send_key(KeyCode::Char('y'))?;

    // Tab zu Trash wechseln und prüfen
    h.send_key(KeyCode::Tab)?;
    h.wait_for(|s| s.contains("Trash"))?;
    let screen = h.screen_contents();
    assert!(screen.contains("delete-me"), "Session sollte im Trash-Tab sichtbar sein");
    Ok(())
}

#[test]
fn e2e_restore_from_trash() -> ratatui_testlib::Result<()> {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-restore-me",
        "uuid-r",
        &[("user", "restore me")],
    );

    let mut h = launch(&env)?;
    h.wait_for_text("restore-me")?;

    // In Trash verschieben
    h.send_key(KeyCode::Char('d'))?;
    h.wait_for(|s| s.contains("trash") || s.contains("Trash") || s.contains("Delete"))?;
    h.send_key(KeyCode::Char('y'))?;

    // Trash-Tab öffnen, Session wiederherstellen
    h.send_key(KeyCode::Tab)?;
    h.wait_for(|s| s.contains("Trash"))?;
    h.wait_for_text("restore-me")?;
    h.send_key(KeyCode::Char('r'))?;
    h.wait_for_text("Restored")?;

    // Zurück zu Sessions-Tab
    h.send_key(KeyCode::Tab)?;
    h.wait_for_text("restore-me")?;
    Ok(())
}

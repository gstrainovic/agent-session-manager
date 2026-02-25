# Agent Session Manager - Claude Guidelines

## Projektübersicht

Terminal-UI Session-Manager für Claude Code Sessions, geschrieben in Rust mit ratatui.

## Architektur

### Module

- **`models.rs`**: Core-Datenstrukturen
  - `Session`: Repräsentiert eine Claude Code Session
  - `Message`: Einzelne Nachricht (user/assistant)
  - `parse_jsonl_messages()`: Parst Claude's JSONL-Format

- **`store.rs`**: Session-Verwaltung
  - Lädt Sessions aus `~/.claude/projects/*/sessions/`
  - Verwaltet Trash-System
  - Filtert und sortiert Sessions

- **`commands.rs`**: Session-Operationen
  - `delete_session()`: Verschiebt zu Trash
  - `export_session()`: Exportiert als Markdown in konfigurierten Pfad

- **`config.rs`**: Persistente Konfiguration
  - `AppConfig`: Serialisierbare Einstellungen (aktuell: `export_path`)
  - Speicherort: `%APPDATA%\agent-session-manager\config.json` (Windows) / `~/.config/agent-session-manager/config.json` (Linux/macOS)
  - Default: `export_path = "~/claude-exports"`

- **`app.rs`**: Application State
  - Verwaltet aktuelle Auswahl, Tab-State, Search-Mode, Settings-Modal
  - Enthält Hauptlogik für UI-Events

- **`ui.rs`**: ratatui Rendering
  - Split-Screen: Session-Liste links, Message-Preview rechts
  - Tabs für Sessions/Trash
  - Search-Modal, Delete-Confirmation Dialog, Settings-Modal, Help-Modal

### Datenfluss

1. `main.rs` initialisiert Terminal und App
2. Event-Loop in `run_app()` behandelt Keyboard-Input
3. `App` aktualisiert State basierend auf Events
4. `ui::draw()` rendert aktuellen State

### JSONL-Format

Claude Code Sessions sind JSONL-Dateien mit Einträgen wie:

```json
{"type":"user","message":{"role":"user","content":"hello"},"uuid":"abc"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hi!"}]},"uuid":"def"}
```

Parser filtert nur `type: "user"` und `type: "assistant"`, ignoriert `file-history-snapshot`, `progress` etc.

## Testing

Alle Module haben umfangreiche Unit-Tests:
- `models::tests`: Session-Creation, Display-Names, JSONL-Parsing
- `store::tests`: Session-Loading, Filtering
- `commands::tests`: Export-Funktionalität
- `config::tests`: Save/Load Roundtrip, Pfad-Auflösung (Tilde-Expansion)
- `app::tests`: State-Transitions, Settings-Modal (open/save/cancel)
- `ui::tests`: Render-Snapshots mit `ratatui::TestBackend` + `insta`

Tests verwenden `tempfile` für isolierte Testdaten.

## Code Style

- **Explizite Error-Handling**: Verwende `Result<T, E>` und `?`
- **Clone sparsam**: Nur wenn Move-Semantics Probleme verursacht
- **Tests MÜSSEN vorhanden sein**: Siehe TDD-Guidelines in globaler CLAUDE.md
- **Keine unwrap() in Production-Code**: Verwende `expect()` mit Messages

## Keyboard-Handling

Event-Loop in `main.rs` delegiert zu App-Methoden:
- Nur `KeyEventKind::Press` wird verarbeitet (Windows feuert auch KeyRelease — würde sonst doppelte Eingaben verursachen)
- Modale Dialoge (Search, Settings, Help, Delete-Confirm) haben Priorität
- `Esc` schließt modale Dialoge, beendet sonst die App
- State-Flags: `app.show_search`, `app.show_settings`, `app.show_help`, `app.confirm_action`

| Shortcut | Funktion |
|----------|----------|
| `g` | Settings-Modal öffnen |
| `e` | Export in `config.export_path` |
| `h` | Help-Modal |
| `Ctrl+F` | Suche |

## E2E UI-Test Konzept

### Ansatz: Integration Tests via `handle_key_event` + `TestBackend`

Keine echte PTY nötig — stattdessen Flows direkt in Rust testen:

```rust
// tests/integration_test.rs (Beispiel-Struktur)
fn simulate_keys(app: &mut App, keys: &[KeyCode]) {
    for code in keys {
        let event = KeyEvent::new(*code, KeyModifiers::NONE);
        handle_key_event(app, event);
    }
}

#[test]
fn test_settings_flow_saves_config() {
    let mut app = App::with_sessions(vec![]);
    // g öffnet Settings
    simulate_keys(&mut app, &[KeyCode::Char('g')]);
    assert!(app.show_settings);
    // Backspace x15 + neuen Pfad tippen
    for _ in 0..15 { app.settings_pop_char(); }
    for c in "/tmp/exports".chars() { app.settings_add_char(c); }
    // Enter speichert
    simulate_keys(&mut app, &[KeyCode::Enter]);
    assert!(!app.show_settings);
    assert_eq!(app.config.export_path, "/tmp/exports");
}

#[test]
fn test_settings_modal_renders() {
    let mut app = App::with_sessions(vec![]);
    app.open_settings();
    let output = render_to_string(&app, 100, 20);
    assert!(output.contains("Settings"));
    assert!(output.contains("Export Path"));
    assert!(output.contains("[Enter] save"));
}
```

### Was getestet werden sollte

| Flow | Typ |
|------|-----|
| Settings öffnen → Pfad ändern → speichern → Config-Datei prüfen | Integration |
| Settings öffnen → Esc → Config unverändert | Integration |
| Export → Datei landet in `config.export_path` | Integration |
| Settings-Modal rendert korrekt (`insta` Snapshot) | UI Snapshot |
| Help-Modal rendert korrekt (`insta` Snapshot) | UI Snapshot |
| Delete-Confirm rendert korrekt | UI Snapshot |

### Fehlende Tests (TODO)

- `ui::tests`: Snapshot für Settings-Modal-State
- `ui::tests`: Snapshot für Help-Modal-State
- `commands::tests`: Export mit benutzerdefiniertem Pfad (nicht default)
- Integration: vollständiger Settings-Änderungs-Flow inkl. Datei-Prüfung

## Bekannte Einschränkungen

- Keine Echtzeit-Updates (Session-Liste wird bei Start geladen)
- Keine direkte Session-Bearbeitung (nur Switch, Delete, Export)
- Messages werden komplett geladen (kein Lazy-Loading)

## Future Ideas

- Session-Suche in Message-Content
- Session-Merge
- Favoriten/Tags
- Auto-Cleanup alter Sessions
- Export nach andere Formate (JSON, HTML)

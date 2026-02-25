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

### Layer 1 — Unit- & Snapshot-Tests (in `src/`)

Alle Module haben umfangreiche Unit-Tests:
- `models::tests`: Session-Creation, Display-Names, JSONL-Parsing
- `store::tests`: Session-Loading, Filtering, Env-Var-Isolation
- `commands::tests`: Export-Funktionalität
- `config::tests`: Save/Load Roundtrip, Pfad-Auflösung (Tilde-Expansion), Env-Var-Isolation
- `app::tests`: State-Transitions, Settings-Modal (open/save/cancel)
- `ui::tests`: Render-Snapshots mit `ratatui::TestBackend` + `insta`

```bash
cargo test  # alle Unit-Tests
```

### Layer 2 — Integration Tests (`tests/integration.rs`)

9 Tests die App-Methoden und Store direkt aufrufen (ohne Binary zu starten):
- Read: Sessions aus Fixture laden, Suche filtert korrekt
- Settings: Speichern → config.json, Cancel → keine Änderung
- Export: Datei landet im konfigurierten Pfad
- Trash/Restore/EmptyTrash: CRUD-Flows

**Isolation:** Zwei Env-Vars entkoppeln von Produktivdaten:
- `CLAUDE_DATA_DIR` → überschreibt `~/.claude` in `SessionStore::new()`
- `AGENT_CONFIG_DIR` → überschreibt Platform-Config-Dir in `AppConfig::config_path()`

`tests/common/mod.rs` enthält `TestEnv`-Struct mit globalem `Mutex<()>` für serielle Ausführung (verhindert Race-Conditions bei Env-Var-Zugriff).

```bash
cargo test --test integration
```

### Layer 3 — E2E PTY-Tests (`tests/e2e.rs`)

6 Tests starten die echte Binary in einem PTY (via `ratatui-testlib` + `portable-pty`) und senden Tastatureingaben:
- E2E-Flows: Session-Anzeige, Suche, Settings, Export, Delete/Trash, Restore

**Plattform:** Nur Linux/macOS (`#![cfg(not(windows))]`).
Windows-Einschränkung: `portable-pty`'s ConPTY-Reader blockiert dauerhaft (keine `WouldBlock`-Semantik), daher hängen die Tests auf Windows.

```bash
cargo build && cargo test --test e2e  # nur Linux/macOS
```

### Test-Ausführung

```bash
cargo test          # alle Tests (Layer 1 + 2; Layer 3 auf Linux/macOS)
cargo test --test integration   # nur Layer 2
cargo test --test e2e           # nur Layer 3 (Linux/macOS)
```

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

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
  - `export_session()`: Exportiert als Markdown nach `~/claude-exports/`

- **`app.rs`**: Application State
  - Verwaltet aktuelle Auswahl, Tab-State, Search-Mode
  - Enthält Hauptlogik für UI-Events

- **`ui.rs`**: ratatui Rendering
  - Split-Screen: Session-Liste links, Message-Preview rechts
  - Tabs für Sessions/Trash
  - Search-Modal
  - Delete-Confirmation Dialog

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

Tests verwenden `tempfile` für isolierte Testdaten.

## Code Style

- **Explizite Error-Handling**: Verwende `Result<T, E>` und `?`
- **Clone sparsam**: Nur wenn Move-Semantics Probleme verursacht
- **Tests MÜSSEN vorhanden sein**: Siehe TDD-Guidelines in globaler CLAUDE.md
- **Keine unwrap() in Production-Code**: Verwende `expect()` mit Messages

## Keyboard-Handling

Event-Loop in `main.rs` delegiert zu App-Methoden:
- Modale Dialoge (Search, Delete-Confirm) haben Priorität
- `Esc` schließt modale Dialoge, beendet sonst die App
- State-Flags: `app.show_search`, `app.confirm_delete`

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

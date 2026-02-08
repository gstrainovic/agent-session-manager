# Agent Session Manager

Ein Terminal-basierter Session-Manager für Claude Code Sessions.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

## Features

- **Session-Übersicht**: Zeigt alle Claude Code Sessions mit Projekt-Informationen
- **Schnelle Navigation**: Wechsle einfach zwischen Sessions mit Pfeiltasten und Enter
- **Suche**: Finde Sessions schnell mit `Ctrl+F`
- **Message-Preview**: Zeigt Konversationsinhalt mit scrollbarer Vorschau
- **Session-Export**: Exportiere Sessions als Markdown-Datei
- **Trash-System**: Lösche und restore Sessions sicher
- **TUI-Interface**: Intuitive Terminal-Oberfläche mit [ratatui](https://github.com/ratatui/ratatui)

## Screenshots

Das Tool bietet zwei Tabs:
- **Sessions Tab**: Übersicht aller aktiven Sessions
- **Trash Tab**: Gelöschte Sessions zum Wiederherstellen

## Installation

### Voraussetzungen

- Rust 1.70+ und Cargo
- Claude Code (CLI)

### Von Source bauen

```bash
git clone https://github.com/DEIN_USERNAME/agent-session-manager.git
cd agent-session-manager
cargo build --release
```

Die Binary findest du dann in `target/release/agent-session-manager`.

Optional in PATH installieren:

```bash
cargo install --path .
```

## Usage

Starte einfach:

```bash
agent-session-manager
```

### Keyboard Shortcuts

| Taste | Funktion |
|-------|----------|
| `↑` / `↓` | Session auswählen |
| `Enter` | Zu ausgewählter Session wechseln |
| `Tab` | Zwischen Sessions/Trash wechseln |
| `Ctrl+F` | Suche öffnen |
| `d` | Session löschen (mit Bestätigung) |
| `y` | Löschen bestätigen |
| `n` / `Esc` | Löschen abbrechen |
| `r` | Session aus Trash wiederherstellen |
| `e` | Session als Markdown exportieren |
| `Page Up` / `Page Down` | Message-Preview scrollen |
| `q` / `Esc` | Beenden |

## Architektur

Das Projekt verwendet eine saubere Modul-Struktur:

- **`models.rs`**: Datenmodelle für Sessions und Messages
- **`store.rs`**: Session-Verwaltung und Datei-I/O
- **`commands.rs`**: Session-Operationen (delete, export, restore)
- **`ui.rs`**: TUI-Rendering mit ratatui
- **`app.rs`**: Anwendungslogik und State-Management
- **`main.rs`**: Event-Loop und Terminal-Setup

## Entwicklung

### Tests ausführen

```bash
cargo test
```

### Development Build

```bash
cargo run
```

### Release Build

```bash
cargo build --release
```

## Session-Daten

Sessions werden aus dem Claude Code Session-Verzeichnis gelesen:

```
~/.claude/projects/<project-hash>/sessions/
```

Exportierte Sessions landen in:

```
~/claude-exports/
```

Trash-Verzeichnis:

```
~/.claude/trash/
```

## Beitragen

Contributions sind willkommen! Öffne gerne ein Issue oder Pull Request.

## License

MIT License - siehe LICENSE-Datei für Details.

## Acknowledgments

- Gebaut mit [ratatui](https://github.com/ratatui/ratatui) für das TUI
- [crossterm](https://github.com/crossterm-rs/crossterm) für Terminal-Handling
- Entwickelt für [Claude Code](https://claude.com/claude-code)

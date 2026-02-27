# Agent Session Manager

A terminal-based session manager for Claude Code sessions.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

## Features

- **Session Overview**: Display all Claude Code sessions with project info, date, and message count
- **Quick Navigation**: Switch between sessions easily with arrow keys and Enter
- **Sortable Columns**: Sort by project name, message count, or date
- **Search**: Find sessions quickly with `f`
- **Message Preview**: Show conversation content with scrollable preview
- **Session Export**: Export sessions as Markdown files to a configurable path
- **Settings**: Configure export path via `g`, persisted across sessions
- **Trash System**: Safely delete and restore sessions
- **Parallel Loading**: Fast loading of large session sets with multi-threading
- **TUI Interface**: Intuitive terminal interface with [ratatui](https://github.com/ratatui/ratatui)

## Screenshots

The tool provides two tabs:
- **Sessions Tab**: Overview of all active sessions
- **Trash Tab**: Deleted sessions for recovery

## Installation

### Requirements

- Rust 1.70+ and Cargo
- Claude Code (CLI)

### Build from Source

```bash
git clone https://github.com/DEIN_USERNAME/agent-session-manager.git
cd agent-session-manager
cargo build --release
```

The binary will be in `target/release/agent-session-manager`.

Optionally install to PATH:

```bash
cargo install --path .
```

## Usage

> **Note:** agent-session-manager is currently designed to work exclusively with [Claude Code](https://claude.com/claude-code). It reads session data from Claude Code's internal directories and manages sessions created through Claude Code only.

Simply run:

```bash
agent-session-manager
```

### Keyboard Shortcuts

| Key | Function |
|-----|----------|
| `↑` / `↓` | Select session (list) / scroll preview (line by line) |
| `←` / `→` | Switch focus between list and preview |
| `Enter` | Switch to selected session |
| `Tab` | Switch between Sessions/Trash |
| `Ctrl+F` | Open search |
| `s` | Toggle sort (Project → Msgs → Date) |
| `S` | Toggle sort direction (▲/▼) |
| `d` | Delete session (with confirmation) |
| `y` | Confirm delete |
| `n` / `Esc` | Cancel delete |
| `r` | Restore session from Trash |
| `t` | Empty trash (in Trash tab) |
| `e` | Export session as Markdown |
| `g` | Open settings (configure export path) |
| `0` | Move all sessions with 0 messages to trash |
| `PgUp` / `PgDn` | Page scroll (depending on focus) |
| `h` | Show help (README) |
| `q` / `Esc` | Quit |

## Architecture

The project uses a clean module structure:

- **`models.rs`**: Data models for sessions and messages
- **`store.rs`**: Session management and file I/O (with parallel loading via rayon)
- **`commands.rs`**: Session operations (delete, export, restore)
- **`config.rs`**: Persistent configuration (export path, config file management)
- **`ui.rs`**: TUI rendering with ratatui
- **`app.rs`**: Application logic and state management
- **`main.rs`**: Event loop and terminal setup

## Development

### Development Build

```bash
cargo run
```

### Release Build

```bash
cargo build --release
```

### Testing

Das Projekt verwendet eine 3-Layer Test-Architektur:

**Layer 1 — Unit-Tests** (92 Tests in `src/`):
```bash
cargo test
```

**Layer 2 — Integration Tests** (9 Tests in `tests/integration.rs`):
```bash
cargo test --test integration
```

**Layer 3 — E2E TUI-Tests** (6 Tests in `tests/e2e/`):

Starten die echte Binary und interagieren per Tastatureingabe mit
[@microsoft/tui-test](https://github.com/microsoft/tui-test) (xterm.js-basiert, plattformübergreifend).

```bash
cargo build                     # Binary muss vorher gebaut sein
cd tests/e2e && npm test        # E2E-Tests ausführen
```

Tests erzeugen bei jedem Lauf **Snapshots** in `tests/e2e/__snapshots__/sessions.test.ts.snap` —
ASCII-Abbilder des Terminal-Zustands an jedem Prüfpunkt zur visuellen Inspektion.

## Session Data

Sessions are read from the Claude Code session directory:

```
~/.claude/projects/<project-hash>/sessions/
```

Exported sessions go to the configured export path (default):

```
~/claude-exports/
```

The export path can be changed via `g` → Settings modal. The configuration is saved to:

- **Linux/macOS**: `~/.config/agent-session-manager/config.json`
- **Windows**: `%APPDATA%\agent-session-manager\config.json`

Trash directory:

```
~/.claude/trash.json
```

## Performance

The tool uses **rayon** for parallel loading of sessions:
- All session files are processed simultaneously by multiple threads
- Significantly faster with large session sets (100+)
- Progress bar shows loading status

## Contributing

Contributions welcome! Please open an issue or pull request.

## License

MIT License - see LICENSE file for details.

## Acknowledgments

- Built with [ratatui](https://github.com/ratatui/ratatui) for the TUI
- [crossterm](https://github.com/crossterm-rs/crossterm) for terminal handling
- [rayon](https://github.com/rayon-rs/rayon) for parallel data processing
- Developed for [Claude Code](https://claude.com/claude-code)


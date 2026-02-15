# Agent Session Manager

A terminal-based session manager for Claude Code sessions.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

## Features

- **Session Overview**: Display all Claude Code sessions with project info, date, and message count
- **Quick Navigation**: Switch between sessions easily with arrow keys and Enter
- **Sortable Columns**: Sort by project name, message count, or date
- **Search**: Find sessions quickly with `Ctrl+F`
- **Message Preview**: Show conversation content with scrollable preview
- **Session Export**: Export sessions as Markdown files
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

Simply run:

```bash
agent-session-manager
```

### Keyboard Shortcuts

| Key | Function |
|-----|----------|
| `↑` / `↓` | Select session (list) / scroll preview (line by line) |
| `←` | Switch / `→` focus between list and preview |
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
| `0` | Move all sessions with 0 messages to trash |
| `PgUp` / `PgDn` | Page scroll (depending on focus) |
| `h` | Show help (README) |
| `q` / `Esc` | Quit |

## Architecture

The project uses a clean module structure:

- **`models.rs`**: Data models for sessions and messages
- **`store.rs`**: Session management and file I/O (with parallel loading via rayon)
- **`commands.rs`**: Session operations (delete, export, restore)
- **`ui.rs`**: TUI rendering with ratatui
- **`app.rs`**: Application logic and state management
- **`main.rs`**: Event loop and terminal setup

## Development

### Run Tests

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

## Session Data

Sessions are read from the Claude Code session directory:

```
~/.claude/projects/<project-hash>/sessions/
```

Exported sessions go to:

```
~/claude-exports/
```

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


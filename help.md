# Agent Session Manager - Help

A terminal-based session manager for Claude Code sessions.

## Keyboard Shortcuts

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
| `h` | Show this help |
| `q` / `Esc` | Quit |

## Features

- **Session Overview**: Display all Claude Code sessions with project info, date, and message count
- **Quick Navigation**: Switch between sessions easily with arrow keys and Enter
- **Sortable Columns**: Sort by project name, message count, or date
- **Search**: Find sessions quickly with `Ctrl+F`
- **Message Preview**: Show conversation content with scrollable preview
- **Session Export**: Export sessions as Markdown files to a configurable path
- **Settings**: Configure export path via `g`, saved to platform config directory
- **Trash System**: Safely delete and restore sessions
- **Parallel Loading**: Fast loading of large session sets with multi-threading
- **TUI Interface**: Intuitive terminal interface with [ratatui](https://github.com/ratatui/ratatui)

## Settings

Press `g` to open the settings modal. Currently configurable:

- **Export Path**: Directory where exported Markdown files are saved (default: `~/claude-exports`)

Settings are persisted to:
- **Linux/macOS**: `~/.config/agent-session-manager/config.json`
- **Windows**: `%APPDATA%\agent-session-manager\config.json`

Created 2026 by Goran Strainovic

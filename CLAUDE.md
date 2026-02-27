# Agent Session Manager - Claude Guidelines

## Project Overview

Terminal-UI session manager for Claude Code sessions, written in Rust with ratatui.

> **Important:** This project is currently **exclusively designed for Claude Code** and only works with it. It reads session data from Claude Code's internal directories and manages only sessions created through Claude Code.

## Architecture

### Modules

- **`models.rs`**: Core data structures
  - `Session`: Represents a Claude Code session
  - `Message`: Individual message (user/assistant)
  - `parse_jsonl_messages()`: Parses Claude's JSONL format

- **`store.rs`**: Session management
  - Loads sessions from `~/.claude/projects/*/sessions/`
  - Manages trash system
  - Filters and sorts sessions

- **`commands.rs`**: Session operations
  - `delete_session()`: Moves to trash
  - `export_session()`: Exports as Markdown to configured path

- **`config.rs`**: Persistent configuration
  - `AppConfig`: Serializable settings (currently: `export_path`)
  - Location: `%APPDATA%\agent-session-manager\config.json` (Windows) / `~/.config/agent-session-manager/config.json` (Linux/macOS)
  - Default: `export_path = "~/claude-exports"`

- **`app.rs`**: Application state
  - Manages current selection, tab state, search mode, settings modal
  - Contains main logic for UI events

- **`ui.rs`**: ratatui rendering
  - Split-screen: session list on left, message preview on right
  - Tabs for Sessions/Trash
  - Search modal, delete confirmation dialog, settings modal, help modal

### Data Flow

1. `main.rs` initializes terminal and app
2. Event loop in `run_app()` handles keyboard input
3. `App` updates state based on events
4. `ui::draw()` renders current state

### JSONL Format

Claude Code sessions are JSONL files with entries like:

```json
{"type":"user","message":{"role":"user","content":"hello"},"uuid":"abc"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hi!"}]},"uuid":"def"}
```

Parser filters only `type: "user"` and `type: "assistant"`, ignores `file-history-snapshot`, `progress` etc.

## Testing

### Layer 1 — Unit & Snapshot Tests (in `src/`)

All modules have extensive unit tests:
- `models::tests`: Session creation, display names, JSONL parsing
- `store::tests`: Session loading, filtering, env var isolation
- `commands::tests`: Export functionality
- `config::tests`: Save/load roundtrip, path resolution (tilde expansion), env var isolation
- `app::tests`: State transitions, settings modal (open/save/cancel)
- `ui::tests`: Render snapshots with `ratatui::TestBackend` + `insta`

```bash
cargo test  # all unit tests
```

### Layer 2 — Integration Tests (`tests/integration.rs`)

9 tests that call app methods and store directly (without starting binary):
- Read: Load sessions from fixtures, search filters correctly
- Settings: Save → config.json, cancel → no changes
- Export: File lands in configured path
- Trash/Restore/EmptyTrash: CRUD flows

**Isolation:** Two env vars decouple from production data:
- `CLAUDE_DATA_DIR` → overrides `~/.claude` in `SessionStore::new()`
- `AGENT_CONFIG_DIR` → overrides platform config dir in `AppConfig::config_path()`

`tests/common/mod.rs` contains `TestEnv` struct with global `Mutex<()>` for serial execution (prevents race conditions in env var access).

```bash
cargo test --test integration
```

### Layer 3 — E2E TUI Tests (`tests/e2e/`)

6 TypeScript tests with `@microsoft/tui-test` (Microsoft). Starts the real binary
and interacts via keyboard input. Uses xterm.js as terminal emulator instead
of ConPTY pipes — works cross-platform (Windows/Linux/macOS).

**Prerequisite:** `cargo build` must run first (binary in `target/debug/`).

```bash
cd tests/e2e && npm test    # Run E2E tests
```

**Snapshots:** Tests contain `toMatchSnapshot()` calls that write terminal state on each run
to `tests/e2e/__snapshots__/sessions.test.ts.snap` (always `--updateSnapshot`).
Snapshots serve for **visual inspection** by the developer, not regression testing
(timestamps and temp paths change on each run). Snapshot file is in `.gitignore`.

### Test Execution

```bash
cargo test                      # all tests (Layer 1 + 2)
cargo test --test integration   # only Layer 2
cd tests/e2e && npm test        # only Layer 3 (E2E)
```

Tests use `tempfile` for isolated test data.

## Code Style

- **Explicit error handling**: Use `Result<T, E>` and `?`
- **Minimize cloning**: Only when move semantics causes issues
- **Tests MUST exist**: See TDD guidelines in global CLAUDE.md
- **No unwrap() in production code**: Use `expect()` with messages

## Keyboard Handling

Event loop in `main.rs` delegates to app methods:
- Only `KeyEventKind::Press` is processed (Windows also fires KeyRelease — would cause double inputs otherwise)
- Modal dialogs (search, settings, help, delete confirm) have priority
- `Esc` closes modal dialogs, exits app otherwise
- State flags: `app.show_search`, `app.show_settings`, `app.show_help`, `app.confirm_action`

| Shortcut | Function |
|----------|----------|
| `g` | Open settings modal |
| `e` | Export to `config.export_path` |
| `h` | Help modal |
| `Ctrl+F` | Search |


## Known Limitations

- **Claude Code dependency:** Currently only works with Claude Code — reads from `~/.claude/projects/*/sessions/`
- No real-time updates (session list loaded at startup)
- No direct session editing (only switch, delete, export)
- Messages fully loaded (no lazy loading)

## Future Ideas

- Search in message content
- Session merge
- Favorites/tags
- Auto-cleanup of old sessions
- Export to other formats (JSON, HTML)

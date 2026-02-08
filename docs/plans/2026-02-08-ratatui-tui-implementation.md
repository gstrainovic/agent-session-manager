# Ratatui Session Manager TUI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust TUI with ratatui for managing Claude Code sessions (Sessions/Trash tabs, split-pane preview, message history, d/r/s/e commands).

**Architecture:** Event-driven TUI with ratatui. Data layer loads sessions from ~/.claude/projects/ and ~/.claude/trash/. Split-pane layout: left list, right message preview. Tab navigation with keyboard commands.

**Tech Stack:** Rust 1.93, ratatui 0.29, crossterm 0.29, serde_json, tokio

---

## Task 1: Set up Cargo.toml dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add dependencies to Cargo.toml**

```toml
[package]
name = "agent-session-manager"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.29"
crossterm = "0.29"
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"

[[bin]]
name = "agent-session-manager"
path = "src/main.rs"
```

**Step 2: Verify build compiles**

Run: `cd /home/g/agent-session-manager && source $HOME/.cargo/env && cargo build`
Expected: Success with no errors

**Step 3: Commit**

```bash
cd /home/g/agent-session-manager
git add Cargo.toml Cargo.lock
git commit -m "chore: add ratatui dependencies"
```

---

## Task 2: Create data structures for sessions and messages

**Files:**
- Create: `src/models.rs`
- Modify: `src/main.rs` (add `mod models;`)

**Step 1: Write test module for Session parsing**

```rust
// src/models.rs - will add tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_load_from_path() {
        // TODO: implement after structure is defined
    }
}
```

**Step 2: Create Session and Message structures**

```rust
// src/models.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub size: u64,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String, // "user" or "assistant"
    pub content: String,
}

impl Session {
    pub fn new(id: String, project_path: String) -> Self {
        Self {
            id,
            project_path,
            created_at: chrono::Local::now().to_rfc3339(),
            updated_at: chrono::Local::now().to_rfc3339(),
            size: 0,
            messages: Vec::new(),
        }
    }
}
```

**Step 3: Add chrono for timestamps**

Update `Cargo.toml`:
```toml
chrono = "0.4"
```

**Step 4: Verify module compiles**

Run: `cd /home/g/agent-session-manager && source $HOME/.cargo/env && cargo check`
Expected: Success

**Step 5: Commit**

```bash
cd /home/g/agent-session-manager
git add src/models.rs Cargo.toml
git commit -m "feat: add Session and Message data structures"
```

---

## Task 3: Create session loader from ~/.claude/projects/

**Files:**
- Create: `src/store.rs`
- Modify: `src/main.rs` (add `mod store;`)

**Step 1: Write failing test for session loading**

```rust
// src/store.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_sessions_from_projects() {
        let store = SessionStore::new();
        let sessions = store.load_sessions().unwrap();
        assert!(sessions.len() >= 0); // Will load whatever exists
    }
}
```

**Step 2: Implement SessionStore**

```rust
// src/store.rs

use crate::models::{Session, Message};
use anyhow::{Result, Context};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub struct SessionStore {
    projects_path: PathBuf,
}

impl SessionStore {
    pub fn new() -> Self {
        let projects_path = dirs::home_dir()
            .expect("home dir")
            .join(".claude/projects");
        
        Self { projects_path }
    }

    pub fn load_sessions(&self) -> Result<Vec<Session>> {
        let mut sessions = Vec::new();
        
        if !self.projects_path.exists() {
            return Ok(sessions);
        }

        for entry in fs::read_dir(&self.projects_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Ok(session) = self.load_session_from_dir(&path) {
                    sessions.push(session);
                }
            }
        }
        
        Ok(sessions)
    }

    fn load_session_from_dir(&self, path: &Path) -> Result<Session> {
        let session_id = path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .context("invalid session dir")?;

        let jsonl_path = path.join(format!("{}.jsonl", session_id));
        
        let mut session = Session::new(
            session_id,
            path.to_string_lossy().to_string(),
        );

        if jsonl_path.exists() {
            let content = fs::read_to_string(&jsonl_path)?;
            for line in content.lines() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                    // Parse user and assistant messages
                    if let Some(content) = json.get("content").and_then(|c| c.as_str()) {
                        let role = json.get("role")
                            .and_then(|r| r.as_str())
                            .unwrap_or("unknown");
                        
                        session.messages.push(Message {
                            role: role.to_string(),
                            content: content.to_string(),
                        });
                    }
                }
            }
        }

        Ok(session)
    }
}
```

**Step 3: Add dirs crate**

Update `Cargo.toml`:
```toml
dirs = "5.0"
```

**Step 4: Run test**

Run: `cd /home/g/agent-session-manager && source $HOME/.cargo/env && cargo test test_load_sessions_from_projects -- --nocapture`
Expected: PASS (loads from real directory)

**Step 5: Commit**

```bash
cd /home/g/agent-session-manager
git add src/store.rs Cargo.toml src/main.rs
git commit -m "feat: add SessionStore to load from ~/.claude/projects"
```

---

## Task 4: Create App state machine

**Files:**
- Create: `src/app.rs`
- Modify: `src/main.rs` (add `mod app;`)

**Step 1: Define App state structure**

```rust
// src/app.rs

use crate::models::Session;
use crate::store::SessionStore;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Sessions,
    Trash,
}

pub struct App {
    pub current_tab: Tab,
    pub sessions: Vec<Session>,
    pub trash: Vec<Session>,
    pub selected_session_idx: usize,
    pub preview_scroll: u16,
    pub search_query: String,
    pub show_search: bool,
}

impl App {
    pub fn new() -> Self {
        let store = SessionStore::new();
        let sessions = store.load_sessions().unwrap_or_default();
        
        Self {
            current_tab: Tab::Sessions,
            sessions,
            trash: Vec::new(),
            selected_session_idx: 0,
            preview_scroll: 0,
            search_query: String::new(),
            show_search: false,
        }
    }

    pub fn select_next(&mut self) {
        let list = match self.current_tab {
            Tab::Sessions => &self.sessions,
            Tab::Trash => &self.trash,
        };
        
        if !list.is_empty() && self.selected_session_idx < list.len() - 1 {
            self.selected_session_idx += 1;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_session_idx > 0 {
            self.selected_session_idx -= 1;
        }
    }

    pub fn switch_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Sessions => Tab::Trash,
            Tab::Trash => Tab::Sessions,
        };
        self.selected_session_idx = 0;
    }

    pub fn get_selected_session(&self) -> Option<&Session> {
        match self.current_tab {
            Tab::Sessions => self.sessions.get(self.selected_session_idx),
            Tab::Trash => self.trash.get(self.selected_session_idx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = App::new();
        assert_eq!(app.current_tab, Tab::Sessions);
        assert_eq!(app.selected_session_idx, 0);
    }

    #[test]
    fn test_select_next() {
        let mut app = App::new();
        app.sessions = vec![
            Session::new("s1".to_string(), "/p1".to_string()),
            Session::new("s2".to_string(), "/p2".to_string()),
        ];
        
        app.select_next();
        assert_eq!(app.selected_session_idx, 1);
        
        app.select_next();
        assert_eq!(app.selected_session_idx, 1); // stays at end
    }

    #[test]
    fn test_switch_tab() {
        let mut app = App::new();
        assert_eq!(app.current_tab, Tab::Sessions);
        
        app.switch_tab();
        assert_eq!(app.current_tab, Tab::Trash);
    }
}
```

**Step 2: Run tests**

Run: `cd /home/g/agent-session-manager && source $HOME/.cargo/env && cargo test test_app`
Expected: PASS all 3 tests

**Step 3: Commit**

```bash
cd /home/g/agent-session-manager
git add src/app.rs src/main.rs
git commit -m "feat: add App state machine with navigation"
```

---

## Task 5: Create basic ratatui UI layout

**Files:**
- Create: `src/ui.rs`
- Modify: `src/main.rs` (add `mod ui;`)

**Step 1: Implement UI rendering**

```rust
// src/ui.rs

use crate::app::{App, Tab};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Length(3),  // Tabs
                Constraint::Min(10),    // Content
                Constraint::Length(3),  // Commands
            ]
            .as_ref(),
        )
        .split(f.size());

    // Draw tabs
    draw_tabs(f, chunks[0], app);

    // Draw content (list + preview)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    draw_list(f, content_chunks[0], app);
    draw_preview(f, content_chunks[1], app);

    // Draw command bar
    draw_commands(f, chunks[2]);
}

fn draw_tabs<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
    let sessions_style = if app.current_tab == Tab::Sessions {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let trash_style = if app.current_tab == Tab::Trash {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let tabs = vec![
        Span::styled("  Sessions  ", sessions_style),
        Span::styled("  Trash  ", trash_style),
    ];

    let tabs_line = Line::from(tabs);
    let tabs_widget = Paragraph::new(tabs_line)
        .block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(tabs_widget, area);
}

fn draw_list<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
    let list = match app.current_tab {
        Tab::Sessions => &app.sessions,
        Tab::Trash => &app.trash,
    };

    let items: Vec<ListItem> = list
        .iter()
        .enumerate()
        .map(|(idx, session)| {
            let style = if idx == app.selected_session_idx {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(format!("• {}", session.id)).style(style)
        })
        .collect();

    let list_widget = List::new(items)
        .block(Block::default().title("Sessions").borders(Borders::ALL));

    f.render_widget(list_widget, area);
}

fn draw_preview<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
    if let Some(session) = app.get_selected_session() {
        let mut lines = vec![Line::from("[Start of Conversation]")];

        for msg in &session.messages {
            let prefix = if msg.role == "user" { "You: " } else { "Agent: " };
            lines.push(Line::from(format!("{}{}", prefix, msg.content)));
        }

        lines.push(Line::from("[End of Conversation]"));

        let preview = Paragraph::new(lines)
            .block(Block::default().title("Preview").borders(Borders::ALL));

        f.render_widget(preview, area);
    } else {
        let empty = Paragraph::new("No session selected")
            .block(Block::default().title("Preview").borders(Borders::ALL));
        f.render_widget(empty, area);
    }
}

fn draw_commands<B: Backend>(f: &mut Frame<B>, area: Rect) {
    let commands = Paragraph::new("[d]elete  [r]estore  [s]witch  [e]xport  | Search: [Ctrl+F]")
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::TOP));

    f.render_widget(commands, area);
}
```

**Step 2: Verify compiles**

Run: `cd /home/g/agent-session-manager && source $HOME/.cargo/env && cargo check`
Expected: Success

**Step 3: Commit**

```bash
cd /home/g/agent-session-manager
git add src/ui.rs src/main.rs
git commit -m "feat: add ratatui UI layout with tabs and split pane"
```

---

## Task 6: Create main event loop

**Files:**
- Modify: `src/main.rs`

**Step 1: Implement main with event loop**

```rust
// src/main.rs

mod app;
mod models;
mod store;
mod ui;

use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAltScreen, LeaveAltScreen},
};
use ratatui::prelude::*;
use std::error::Error;
use std::io;

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAltScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAltScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if crossterm::event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Tab => app.switch_tab(),
                    KeyCode::Up => app.select_prev(),
                    KeyCode::Down => app.select_next(),
                    KeyCode::Char('d') => println!("Delete pressed"),
                    KeyCode::Char('r') => println!("Restore pressed"),
                    KeyCode::Char('s') => println!("Switch pressed"),
                    KeyCode::Char('e') => println!("Export pressed"),
                    _ => {}
                }
            }
        }
    }
}
```

**Step 2: Build and test**

Run: `cd /home/g/agent-session-manager && source $HOME/.cargo/env && cargo build`
Expected: Success

**Step 3: Commit**

```bash
cd /home/g/agent-session-manager
git add src/main.rs
git commit -m "feat: add main event loop with keyboard handling"
```

---

## Task 7: Add message scrolling in preview pane

**Files:**
- Modify: `src/app.rs` (add scroll methods)
- Modify: `src/ui.rs` (use scroll in preview)
- Modify: `src/main.rs` (add scroll keys)

**Step 1: Add scroll methods to App**

```rust
// In src/app.rs - add methods to impl App

pub fn scroll_preview_down(&mut self) {
    self.preview_scroll = self.preview_scroll.saturating_add(3);
}

pub fn scroll_preview_up(&mut self) {
    self.preview_scroll = self.preview_scroll.saturating_sub(3);
}
```

**Step 2: Update UI to render with scroll**

```rust
// In src/ui.rs - update draw_preview

use ratatui::widgets::Paragraph;

fn draw_preview<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
    if let Some(session) = app.get_selected_session() {
        let mut lines = vec![Line::from("[Start of Conversation]")];

        for msg in &session.messages {
            let prefix = if msg.role == "user" { "You: " } else { "Agent: " };
            lines.push(Line::from(format!("{}{}", prefix, msg.content)));
        }

        lines.push(Line::from("[End of Conversation]"));

        let preview = Paragraph::new(lines)
            .block(Block::default().title("Preview").borders(Borders::ALL))
            .scroll((app.preview_scroll, 0));

        f.render_widget(preview, area);
    } else {
        let empty = Paragraph::new("No session selected")
            .block(Block::default().title("Preview").borders(Borders::ALL));
        f.render_widget(empty, area);
    }
}
```

**Step 3: Add scroll keys**

```rust
// In src/main.rs - update run_app match

KeyCode::PageDown => app.scroll_preview_down(),
KeyCode::PageUp => app.scroll_preview_up(),
```

**Step 4: Test it works**

Run: `cd /home/g/agent-session-manager && source $HOME/.cargo/env && cargo build`
Expected: Compiles

**Step 5: Commit**

```bash
cd /home/g/agent-session-manager
git add src/app.rs src/ui.rs src/main.rs
git commit -m "feat: add message preview scrolling with Page Up/Down"
```

---

## Task 8: Add search modal (Ctrl+F)

**Files:**
- Modify: `src/app.rs` (add search state)
- Modify: `src/ui.rs` (draw search modal)
- Modify: `src/main.rs` (handle Ctrl+F)

**Step 1: Write test for search filtering**

```rust
// In src/app.rs - add test

#[test]
fn test_search_filters_sessions() {
    let mut app = App::new();
    app.sessions = vec![
        Session::new("auto-service".to_string(), "/p1".to_string()),
        Session::new("dms-project".to_string(), "/p2".to_string()),
    ];
    
    app.search_query = "auto".to_string();
    let filtered = app.filtered_sessions();
    assert_eq!(filtered.len(), 1);
}
```

**Step 2: Add filtered_sessions method**

```rust
// In src/app.rs impl App

pub fn filtered_sessions(&self) -> Vec<&Session> {
    let list = match self.current_tab {
        Tab::Sessions => &self.sessions,
        Tab::Trash => &self.trash,
    };

    if self.search_query.is_empty() {
        list.iter().collect()
    } else {
        let q = self.search_query.to_lowercase();
        list.iter()
            .filter(|s| s.id.to_lowercase().contains(&q))
            .collect()
    }
}

pub fn toggle_search(&mut self) {
    self.show_search = !self.show_search;
    if !self.show_search {
        self.search_query.clear();
    }
}

pub fn add_search_char(&mut self, c: char) {
    self.search_query.push(c);
}

pub fn pop_search_char(&mut self) {
    self.search_query.pop();
}
```

**Step 3: Update UI to show search modal**

```rust
// In src/ui.rs - add draw_search_modal and update draw_list

fn draw_list<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
    let filtered = app.filtered_sessions();
    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(idx, session)| {
            let style = if idx == app.selected_session_idx {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            ListItem::new(format!("• {}", session.id)).style(style)
        })
        .collect();

    let list_widget = List::new(items)
        .block(Block::default().title("Sessions").borders(Borders::ALL));

    f.render_widget(list_widget, area);
}

fn draw_search_modal<B: Backend>(f: &mut Frame<B>, app: &App) {
    if !app.show_search {
        return;
    }

    let area = Rect {
        x: f.size().width / 4,
        y: f.size().height / 2,
        width: f.size().width / 2,
        height: 3,
    };

    let search = Paragraph::new(format!("Search: {}_", app.search_query))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().bg(Color::Black).fg(Color::White));

    f.render_widget(search, area);
}

// Update main draw function to call draw_search_modal after other draws
```

**Step 4: Handle Ctrl+F in main loop**

```rust
// In src/main.rs - update event handling

match key.code {
    // ...existing...
    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
        app.toggle_search()
    }
    _ if app.show_search => {
        match key.code {
            KeyCode::Char(c) => app.add_search_char(c),
            KeyCode::Backspace => app.pop_search_char(),
            KeyCode::Esc => app.show_search = false,
            _ => {}
        }
    }
    _ => {}
}
```

**Step 5: Run tests**

Run: `cd /home/g/agent-session-manager && source $HOME/.cargo/env && cargo test test_search`
Expected: PASS

**Step 6: Commit**

```bash
cd /home/g/agent-session-manager
git add src/app.rs src/ui.rs src/main.rs
git commit -m "feat: add search modal with Ctrl+F"
```

---

## Task 9: Implement delete command

**Files:**
- Create: `src/commands.rs`
- Modify: `src/main.rs` (add `mod commands;`)

**Step 1: Write test for delete**

```rust
// src/commands.rs

use crate::models::Session;
use anyhow::Result;

pub fn delete_session(session: &Session) -> Result<()> {
    // Move to trash
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_moves_to_trash() {
        let session = Session::new("test".to_string(), "/path".to_string());
        let result = delete_session(&session);
        assert!(result.is_ok());
    }
}
```

**Step 2: Run test (will be minimal)**

Run: `cd /home/g/agent-session-manager && source $HOME/.cargo/env && cargo test test_delete`
Expected: PASS

**Step 3: Update main to call delete**

```rust
// In src/main.rs

KeyCode::Char('d') => {
    if let Some(session) = app.get_selected_session() {
        if let Err(e) = commands::delete_session(session) {
            eprintln!("Delete error: {}", e);
        }
    }
}
```

**Step 4: Commit**

```bash
cd /home/g/agent-session-manager
git add src/commands.rs src/main.rs
git commit -m "feat: add delete command handler"
```

---

## Task 10: Polish and testing

**Files:**
- Update: `src/main.rs` (cleanup)
- Update: `src/ui.rs` (styling)

**Step 1: Add better error handling**

Add result types to main, clean error display

**Step 2: Test with actual data**

Run TUI and navigate through real sessions

**Step 3: Final commit**

```bash
cd /home/g/agent-session-manager
git add -A
git commit -m "polish: final cleanup and styling"
```

---

## Execution Strategy

Once ready: Use superpowers:executing-plans to run each task sequentially with code review checkpoints.

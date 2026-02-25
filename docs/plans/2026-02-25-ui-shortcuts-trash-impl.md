# UI/Shortcuts/Trash Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Trash als Verzeichnis (fs::rename statt JSON), Command Bar 2-zeilig mit allen Shortcuts, Tab Bar mit ●/○, Preview Scroll-Indikator.

**Architecture:** Trash-Dateien werden per `fs::rename` nach `~/.claude/trash/<project>/<id>.jsonl` verschoben. `Session.original_content` entfällt. UI-Änderungen nur in `ui.rs` mit angepassten Layout-Constraints.

**Tech Stack:** Rust, ratatui 0.29, insta (Snapshots), tempfile (Tests)

---

## Task 1: Session.original_content entfernen

**Files:**
- Modify: `src/models.rs`
- Modify: `src/store.rs` (Verwendungsstellen)
- Modify: `src/app.rs` (Verwendungsstellen)

**Step 1: Failing test schreiben**

In `src/models.rs` → `#[cfg(test)] mod tests`:

```rust
#[test]
fn test_session_has_no_original_content_field() {
    let s = Session {
        id: "x".to_string(),
        project_path: "/p".to_string(),
        project_name: "p".to_string(),
        created_at: "".to_string(),
        updated_at: "".to_string(),
        size: 0,
        total_entries: 0,
        messages: vec![],
        // Wenn original_content noch vorhanden: compile error erwartet
    };
    assert_eq!(s.id, "x");
}
```

Dieser Test kompiliert nicht solange `original_content` noch existiert und nicht angegeben ist.

**Step 2: Test fehlschlagen lassen**

```bash
cargo test test_session_has_no_original_content_field 2>&1 | head -20
```
Erwartung: Compile-Fehler "missing field `original_content`"

**Step 3: original_content entfernen**

In `src/models.rs`:
```rust
// VORHER:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_path: String,
    pub project_name: String,
    pub created_at: String,
    pub updated_at: String,
    pub size: u64,
    pub total_entries: usize,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_content: Option<String>,
}

// NACHHER:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_path: String,
    pub project_name: String,
    pub created_at: String,
    pub updated_at: String,
    pub size: u64,
    pub total_entries: usize,
    pub messages: Vec<Message>,
}
```

In `src/models.rs` → `Session::new()`:
```rust
// original_content: None,  ← diese Zeile löschen
```

In `src/store.rs` → `load_session_from_jsonl()`:
```rust
// VORHER:
Ok(Session {
    ...
    original_content: Some(content),
})

// NACHHER:
Ok(Session {
    ...
    // original_content entfernt
})
```

In `src/app.rs` → alle `original_content: None` Vorkommen entfernen (in `make_session` Test-Helper).

**Step 4: Tests laufen lassen**

```bash
cargo test 2>&1 | tail -20
```
Erwartung: alle Tests grün

**Step 5: Commit**

```bash
git add src/models.rs src/store.rs src/app.rs
git commit -m "refactor: remove Session.original_content field"
```

---

## Task 2: Store — Trash als Verzeichnis

**Files:**
- Modify: `src/store.rs`

**Step 1: Neue Tests schreiben**

Am Ende von `src/store.rs` → `#[cfg(test)] mod tests`, neue Tests hinzufügen:

```rust
#[test]
fn test_move_to_trash_moves_file() {
    let (tmp, store) = create_test_store();
    let project_dir = tmp.path().join("projects/-home-g-project");
    fs::create_dir_all(&project_dir).unwrap();
    let line = r#"{"type":"user","message":{"role":"user","content":"hi"},"uuid":"x"}"#;
    fs::write(project_dir.join("s1.jsonl"), line).unwrap();

    store.move_to_trash("-home-g-project", "s1").unwrap();

    assert!(!project_dir.join("s1.jsonl").exists(), "Datei muss aus projects verschwunden sein");
    assert!(
        tmp.path().join("trash/-home-g-project/s1.jsonl").exists(),
        "Datei muss im trash-Verzeichnis sein"
    );
}

#[test]
fn test_load_trash_reads_moved_files() {
    let (tmp, store) = create_test_store();
    let trash_dir = tmp.path().join("trash/-home-g-project");
    fs::create_dir_all(&trash_dir).unwrap();
    let line = r#"{"type":"user","message":{"role":"user","content":"hi"},"uuid":"x"}"#;
    fs::write(trash_dir.join("s1.jsonl"), line).unwrap();

    let trash = store.load_trash().unwrap();
    assert_eq!(trash.len(), 1);
    assert_eq!(trash[0].id, "s1");
    assert_eq!(trash[0].project_name, "-home-g-project");
}

#[test]
fn test_restore_moves_file_back() {
    let (tmp, store) = create_test_store();
    let project_dir = tmp.path().join("projects/-home-g-project");
    fs::create_dir_all(&project_dir).unwrap();
    let line = r#"{"type":"user","message":{"role":"user","content":"hi"},"uuid":"x"}"#;
    fs::write(project_dir.join("s1.jsonl"), line).unwrap();

    store.move_to_trash("-home-g-project", "s1").unwrap();
    assert!(!project_dir.join("s1.jsonl").exists());

    let trash = store.load_trash().unwrap();
    store.restore_session_file(&trash[0]).unwrap();

    assert!(project_dir.join("s1.jsonl").exists(), "Datei muss zurück in projects sein");
    assert!(!tmp.path().join("trash/-home-g-project/s1.jsonl").exists());
}

#[test]
fn test_empty_trash_removes_all_files() {
    let (tmp, store) = create_test_store();
    let project_dir = tmp.path().join("projects/-home-g-project");
    fs::create_dir_all(&project_dir).unwrap();
    let line = r#"{"type":"user","message":{"role":"user","content":"hi"},"uuid":"x"}"#;
    fs::write(project_dir.join("s1.jsonl"), line).unwrap();
    fs::write(project_dir.join("s2.jsonl"), line).unwrap();

    store.move_to_trash("-home-g-project", "s1").unwrap();
    store.move_to_trash("-home-g-project", "s2").unwrap();

    store.empty_trash().unwrap();

    assert!(!tmp.path().join("trash").exists() ||
            tmp.path().join("trash").read_dir().map(|mut d| d.next().is_none()).unwrap_or(true));
}

#[test]
fn test_load_sessions_ignores_trash_dir() {
    let (tmp, store) = create_test_store();
    // Session in projects
    let project_dir = tmp.path().join("projects/-home-g-project");
    fs::create_dir_all(&project_dir).unwrap();
    let line = r#"{"type":"user","message":{"role":"user","content":"hi"},"uuid":"x"}"#;
    fs::write(project_dir.join("s1.jsonl"), line).unwrap();
    // Session in trash — darf NICHT in load_sessions auftauchen
    let trash_dir = tmp.path().join("trash/-home-g-project");
    fs::create_dir_all(&trash_dir).unwrap();
    fs::write(trash_dir.join("s2.jsonl"), line).unwrap();

    let sessions = store.load_sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, "s1");
}
```

**Step 2: Tests fehlschlagen lassen**

```bash
cargo test test_move_to_trash_moves_file 2>&1 | tail -10
```
Erwartung: FAIL — `move_to_trash` existiert nicht

**Step 3: Store refactoren**

**3a: `with_path` → `with_base` umbenennen** (bricht Store-Unit-Tests, werden in Step 3c gefixt):

```rust
// src/store.rs

pub struct SessionStore {
    projects_path: PathBuf,
    trash_path: PathBuf,  // jetzt Verzeichnis, nicht Datei
}

impl SessionStore {
    pub fn new() -> Self {
        let base = if let Ok(dir) = std::env::var("CLAUDE_DATA_DIR") {
            PathBuf::from(dir)
        } else {
            dirs::home_dir().expect("home dir").join(".claude")
        };
        Self {
            projects_path: base.join("projects"),
            trash_path: base.join("trash"),
        }
    }

    #[cfg(test)]
    pub fn with_base(base: PathBuf) -> Self {
        Self {
            projects_path: base.join("projects"),
            trash_path: base.join("trash"),
        }
    }
```

**3b: Neue Methoden hinzufügen, alte entfernen:**

```rust
    /// Verschiebt Session-JSONL per rename in trash-Verzeichnis
    pub fn move_to_trash(&self, project_name: &str, session_id: &str) -> Result<()> {
        let src = self.get_session_file_path(project_name, session_id);
        let dst_dir = self.trash_path.join(project_name);
        fs::create_dir_all(&dst_dir)?;
        let dst = dst_dir.join(format!("{}.jsonl", session_id));
        fs::rename(&src, &dst)?;
        Ok(())
    }

    /// Verschiebt Session-JSONL aus trash zurück nach projects
    pub fn restore_session_file(&self, session: &Session) -> Result<()> {
        let src = self.trash_path
            .join(&session.project_name)
            .join(format!("{}.jsonl", session.id));
        let dst_dir = self.projects_path.join(&session.project_name);
        fs::create_dir_all(&dst_dir)?;
        let dst = dst_dir.join(format!("{}.jsonl", session.id));
        fs::rename(&src, &dst)?;
        Ok(())
    }

    /// Löscht das komplette trash-Verzeichnis
    pub fn empty_trash(&self) -> Result<()> {
        if self.trash_path.exists() {
            fs::remove_dir_all(&self.trash_path)?;
        }
        Ok(())
    }

    /// Liest alle Sessions aus dem trash-Verzeichnis
    pub fn load_trash(&self) -> Result<Vec<Session>> {
        if !self.trash_path.exists() {
            return Ok(Vec::new());
        }
        let mut sessions = Vec::new();
        for project_entry in fs::read_dir(&self.trash_path)? {
            let project_entry = project_entry?;
            let project_path = project_entry.path();
            if !project_path.is_dir() {
                continue;
            }
            let project_slug = project_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let resolved_path = slug_to_path(&project_slug)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| project_slug.clone());
            for file_entry in fs::read_dir(&project_path)? {
                let file_entry = file_entry?;
                let file_path = file_entry.path();
                if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }
                if let Ok(session) =
                    self.load_session_from_jsonl(&file_path, &project_slug, &resolved_path)
                {
                    sessions.push(session);
                }
            }
        }
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }
```

**`save_trash` und `delete_session_file` entfernen** (werden nicht mehr gebraucht).

Achtung: `delete_session_file` wird noch in `empty_trash()` (alt) in `app.rs` verwendet. Erst in Task 3 entfernen.

**3c: Store-Unit-Tests fixen**

`create_test_store` anpassen:
```rust
fn create_test_store() -> (TempDir, SessionStore) {
    let tmp = TempDir::new().unwrap();
    let store = SessionStore::with_base(tmp.path().to_path_buf());
    (tmp, store)
}
```

Alle Tests die `tmp.path().join("-home-g-project")` verwenden:
```rust
// VORHER:
let project_dir = tmp.path().join("-home-g-myproject");

// NACHHER:
let project_dir = tmp.path().join("projects/-home-g-myproject");
```

Betroffene Tests (alle in `store::tests`):
- `test_loads_sessions_from_project_subdirs`
- `test_multiple_sessions_in_one_project`
- `test_skips_non_jsonl_files`
- `test_sessions_sorted_by_updated_at_desc`
- `test_count_session_files`
- `test_load_sessions_with_progress`
- `test_delete_session_file_removes_from_load` → umbenennen zu `test_move_to_trash_removes_from_projects`, umschreiben
- `test_delete_session_file_is_idempotent` → entfernen (Methode existiert nicht mehr)
- `test_restore_session_file_recreates_jsonl` → umschreiben für neue Logik
- `test_load_trash_*` → für neue Logik umschreiben (kein JSON mehr)

**Step 4: Tests laufen lassen**

```bash
cargo test --lib 2>&1 | tail -20
```
Erwartung: alle Store-Tests grün

**Step 5: Commit**

```bash
git add src/store.rs src/models.rs
git commit -m "refactor: trash as directory with fs::rename, remove save_trash"
```

---

## Task 3: App.rs — neue Store-Methoden verwenden

**Files:**
- Modify: `src/app.rs`

**Step 1: Failing test schreiben**

In `src/app.rs` → tests: bestehende Tests prüfen ob sie noch kompilieren. Kein neuer Test nötig — die bestehenden `test_move_to_trash`, `test_restore_from_trash` testen den richtigen State.

**Step 2: app.rs anpassen**

`move_selected_to_trash()`:
```rust
// VORHER:
let store = SessionStore::new();
let _ = store.delete_session_file(&removed.project_name, &removed.id);
self.trash.push(removed);
let _ = store.save_trash(&self.trash);

// NACHHER:
let store = SessionStore::new();
let _ = store.move_to_trash(&removed.project_name, &removed.id);
self.trash.push(removed);
// kein save_trash mehr
```

`restore_selected_from_trash()`:
```rust
// save_trash Zeile entfernen:
// let _ = store.save_trash(&self.trash);  ← löschen
```

`delete_permanently()`:
```rust
// save_trash Zeile entfernen
// let _ = store.save_trash(&self.trash);  ← löschen
```

`empty_trash()`:
```rust
// VORHER:
let store = SessionStore::new();
for session in &self.trash {
    let _ = store.delete_session_file(&session.project_name, &session.id);
}
self.trash.clear();
let _ = store.save_trash(&self.trash);

// NACHHER:
let store = SessionStore::new();
let _ = store.empty_trash();
self.trash.clear();
```

`trash_zero_messages()`:
```rust
// save_trash Zeile entfernen
// let _ = store.save_trash(&self.trash);  ← löschen
```

**Step 3: Tests laufen lassen**

```bash
cargo test 2>&1 | tail -20
```
Erwartung: alle Tests grün

**Step 4: Commit**

```bash
git add src/app.rs
git commit -m "refactor: app uses move_to_trash and empty_trash, no more save_trash"
```

---

## Task 4: Tab Bar mit ●/○

**Files:**
- Modify: `src/ui.rs` → `draw_tabs()`

**Step 1: Failing Snapshot-Test schreiben**

Der bestehende `test_snapshot_initial_render` wird nach der Änderung einen anderen Output haben. Kein neuer Test nötig — Snapshot-Update in Step 4.

Für TDD: prüfe dass aktueller Test noch läuft:
```bash
cargo test test_snapshot_initial_render 2>&1 | tail -5
```

**Step 2: draw_tabs() anpassen**

```rust
fn draw_tabs(f: &mut Frame, area: Rect, app: &App) {
    let session_count = app.sessions.len();
    let trash_count = app.trash.len();

    let (sessions_marker, sessions_style) = if app.current_tab == Tab::Sessions {
        ("●", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    } else {
        ("○", Style::default().fg(Color::DarkGray))
    };

    let (trash_marker, trash_style) = if app.current_tab == Tab::Trash {
        ("●", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    } else {
        ("○", Style::default().fg(Color::DarkGray))
    };

    let tabs = vec![
        Span::styled(
            format!("  {} Sessions ({})  ", sessions_marker, session_count),
            sessions_style,
        ),
        Span::styled(
            format!("  {} Trash ({})  ", trash_marker, trash_count),
            trash_style,
        ),
        Span::styled(
            "│  Tab: switch  h: help  ",
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let tabs_line = Line::from(tabs);
    let tabs_widget = Paragraph::new(tabs_line).block(
        Block::default()
            .title(" Agent Session Manager ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(tabs_widget, area);
}
```

**Step 3: Snapshots updaten**

```bash
cargo test -- --test-output immediate 2>&1 | grep -E "FAIL|snapshot"
INSTA_UPDATE=always cargo test 2>&1 | tail -10
```

**Step 4: Tests grün bestätigen**

```bash
cargo test 2>&1 | tail -10
```

**Step 5: Commit**

```bash
git add src/ui.rs src/snapshots/
git commit -m "feat: tab bar with ●/○ active indicator and help hint"
```

---

## Task 5: Command Bar — 2 Zeilen, alle Shortcuts

**Files:**
- Modify: `src/ui.rs` → `draw()` Layout + `draw_commands()`

**Step 1: Layout anpassen**

In `draw()` das untere Constraint von `Length(3)` auf `Length(4)` ändern:

```rust
// src/ui.rs → draw()
.constraints(
    [
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(4),  // war: Length(3)
    ]
    .as_ref(),
)
```

**Step 2: draw_commands() ersetzen**

```rust
fn draw_commands(f: &mut Frame, area: Rect, app: &App) {
    let sep = Span::styled("│", Style::default().fg(Color::DarkGray));

    // Status-Nachricht: volle Breite, 1 Zeile
    if let Some(ref msg) = app.status_message {
        let bar = Paragraph::new(Line::from(vec![
            Span::styled(msg.as_str(), Style::default().fg(Color::Green)),
        ]))
        .block(Block::default().borders(Borders::TOP).border_style(
            Style::default().fg(Color::DarkGray),
        ));
        f.render_widget(bar, area);
        return;
    }

    let c = |s: &'static str, color: Color| Span::styled(s, Style::default().fg(color));
    let w = |s: &'static str| Span::raw(s);

    let (line1, line2) = match app.current_tab {
        Tab::Sessions => (
            Line::from(vec![
                c("↑↓", Color::Cyan), w(" nav  "),
                c("←→", Color::Cyan), w(" focus  "),
                sep.clone(), w("  "),
                c("[Enter]", Color::Cyan), w(" resume  "),
                c("[d]", Color::Red), w("elete  "),
                c("[e]", Color::Yellow), w("xport  "),
                c("[0]", Color::Red), w(" clean  "),
            ]),
            Line::from(vec![
                w("                   "),
                sep.clone(), w("  "),
                c("[s]", Color::Magenta), w(" sort  "),
                c("[S]", Color::Magenta), w(" dir  "),
                c("[Ctrl+F]", Color::Cyan), w(" search  "),
                c("[g]", Color::Magenta), w(" settings  "),
                c("[h]", Color::DarkGray), w("elp  "),
                c("[q]", Color::DarkGray), w("uit"),
            ]),
        ),
        Tab::Trash => (
            Line::from(vec![
                c("↑↓", Color::Cyan), w(" nav  "),
                c("←→", Color::Cyan), w(" focus  "),
                sep.clone(), w("  "),
                c("[r]", Color::Green), w("estore  "),
                c("[d]", Color::Red), w("elete  "),
                c("[t]", Color::Red), w(" empty trash  "),
            ]),
            Line::from(vec![
                w("                   "),
                sep.clone(), w("  "),
                c("[s]", Color::Magenta), w(" sort  "),
                c("[S]", Color::Magenta), w(" dir  "),
                c("[g]", Color::Magenta), w(" settings  "),
                c("[h]", Color::DarkGray), w("elp  "),
                c("[q]", Color::DarkGray), w("uit"),
            ]),
        ),
    };

    let text = ratatui::text::Text::from(vec![line1, line2]);
    let bar = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(bar, area);
}
```

**Step 3: Snapshots updaten**

```bash
INSTA_UPDATE=always cargo test 2>&1 | tail -10
```

**Step 4: Tests grün**

```bash
cargo test 2>&1 | tail -10
```

**Step 5: Commit**

```bash
git add src/ui.rs src/snapshots/
git commit -m "feat: command bar 2 lines with all shortcuts categorized"
```

---

## Task 6: Preview Scroll-Indikator

**Files:**
- Modify: `src/ui.rs` → `draw_preview()`

**Step 1: Logik hinzufügen**

Am Ende von `draw_preview()`, nach dem Paragraph-Aufbau, Scroll-Info in den Block-Titel einbauen:

```rust
// Zeile zählen für Scroll-Indikator
let total_lines = lines.len();
let visible_height = area.height.saturating_sub(2) as usize; // minus Borders

let title = if total_lines > visible_height {
    let current_page = (app.preview_scroll as usize / visible_height.max(1)) + 1;
    let total_pages = (total_lines + visible_height - 1) / visible_height.max(1);
    format!(" Preview  ↓ {}/{} ", current_page, total_pages)
} else {
    " Preview ".to_string()
};

let preview = Paragraph::new(lines)
    .block(
        Block::default()
            .title(title)  // war: .title(" Preview ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if app.focus == FocusPanel::Preview {
                Color::Yellow
            } else {
                Color::DarkGray
            })),
    )
    .style(Style::default().bg(Color::Black))
    .wrap(Wrap { trim: false })
    .scroll((app.preview_scroll, 0));
```

**Step 2: Snapshots updaten**

```bash
INSTA_UPDATE=always cargo test 2>&1 | tail -10
```

**Step 3: Tests grün**

```bash
cargo test 2>&1 | tail -10
```

**Step 4: Commit**

```bash
git add src/ui.rs src/snapshots/
git commit -m "feat: preview scroll indicator in title bar"
```

---

## Task 7: Integration Tests anpassen

**Files:**
- Modify: `tests/integration.rs`
- Modify: `tests/common/mod.rs`

**Step 1: Integration Tests laufen**

```bash
cargo test --test integration 2>&1 | tail -20
```
Prüfen ob alle 9 Tests noch grün sind.

**Step 2: Trash-Tests erweitern (Dateisystem prüfen)**

`test_delete_moves_session_to_trash` erweitern:
```rust
#[test]
fn test_delete_moves_session_to_trash() {
    let env = TestEnv::new();
    create_fixture_session(
        &env.claude_dir,
        "-del-project",
        "uuid-del",
        &[("user", "delete me")],
    );
    let sessions = load_sessions(&env);

    env.activate();
    let mut app = App::new(sessions, vec![]);
    assert_eq!(app.sessions.len(), 1);
    app.move_selected_to_trash();
    TestEnv::deactivate();

    assert_eq!(app.sessions.len(), 0);
    assert_eq!(app.trash.len(), 1);
    // Dateisystem prüfen
    let trash_file = env.claude_dir.join("trash/-del-project/uuid-del.jsonl");
    assert!(trash_file.exists(), "JSONL muss im trash-Verzeichnis liegen");
    let original = env.claude_dir.join("projects/-del-project/uuid-del.jsonl");
    assert!(!original.exists(), "Original muss verschwunden sein");
}
```

Hinweis: `env.activate()` muss um die App-Operationen herum gesetzt sein damit `SessionStore::new()` den richtigen Pfad kennt.

Gleiches Muster für `test_restore_session_from_trash` und `test_empty_trash`.

**Step 3: Tests laufen**

```bash
cargo test --test integration 2>&1 | tail -20
```
Erwartung: alle Tests grün

**Step 4: Alle Tests final**

```bash
cargo test 2>&1 | tail -10
```

**Step 5: Abschluss-Commit**

```bash
git add tests/
git commit -m "test: update integration tests to verify trash filesystem state"
```

---

## Abschluss

```bash
cargo test 2>&1 | tail -5
# expected: test result: ok. X passed; 0 failed
```

todo.md kann geleert werden.

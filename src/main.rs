mod app;
mod commands;
mod config;
mod models;
mod store;
mod ui;

use app::App;
use crossterm::{
    event::{self, Event, KeyCode, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::error::Error;
use std::io;

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Load sessions with progress bar
    let store = store::SessionStore::new();
    let sessions = store
        .load_sessions_with_progress(|loaded, total| {
            let _ = terminal.draw(|f| {
                ui::draw_loading(f, loaded, total);
            });
        })
        .unwrap_or_default();

    let trash = store.load_trash().unwrap_or_default();
    let app = App::new(sessions, trash);
    let res = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::event::DisableMouseCapture)?;
    terminal.show_cursor()?;

    match res {
        Ok(Some((command, path))) => {
            let session_id = command.strip_prefix("claude --resume ").unwrap_or_default();
            launch_claude_resume(session_id, path);
        }
        Err(err) => {
            eprintln!("{:?}", err);
        }
        Ok(None) => {}
    }

    Ok(())
}

/// Launches `claude --resume <id>` after the TUI has exited.
/// Uses spawn+wait on all platforms (terminal is already restored at this point).
fn launch_claude_resume(session_id: &str, path: Option<String>) {
    #[cfg(target_family = "unix")]
    let (prog, args) = {
        let cmd = match path {
            Some(ref p) => format!("clear && cd {} && claude --resume {}", shell_escape(p), session_id),
            None => format!("clear && claude --resume {}", session_id),
        };
        ("sh", vec!["-c".to_string(), cmd])
    };

    #[cfg(target_os = "windows")]
    let (prog, args) = {
        let cmd = match path {
            Some(ref p) => format!("cls && cd /d {} && claude --resume {}", cmd_quote(p), session_id),
            None => format!("cls && claude --resume {}", session_id),
        };
        ("cmd", vec!["/c".to_string(), cmd])
    };

    match std::process::Command::new(prog).args(&args).spawn() {
        Ok(mut child) => { let _ = child.wait(); }
        Err(e) => { eprintln!("Failed to launch claude: {}", e); std::process::exit(1); }
    }
}

#[cfg(target_family = "unix")]
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(target_os = "windows")]
fn cmd_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<Option<(String, Option<String>)>> {
    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;
        app.clear_expired_status();

        // Check if we should resume a session
        if let Some(command) = app.get_resume_command() {
            let path = app.get_resume_session_path();
            return Ok(Some((command, path)));
        }

        // Wait for at least one event (with timeout for status message expiry)
        if crossterm::event::poll(std::time::Duration::from_millis(250))? {
            // Drain all pending events before next draw to avoid rendering artifacts
            loop {
                match event::read()? {
                    Event::Key(key) if key.kind == event::KeyEventKind::Press => {
                        if let Some(result) = handle_key_event(&mut app, key) {
                            return result;
                        }
                    }
                    Event::Mouse(mouse) => {
                        if handle_mouse_event(&mut app, mouse) {
                            return Ok(None);
                        }
                    }
                    _ => {}
                }
                // Check if more events are immediately available
                if !crossterm::event::poll(std::time::Duration::from_millis(0))? {
                    break;
                }
            }
        }
    }
}

/// Handles a single key event, returning `Some(result)` if the app should exit,
/// or `None` to continue the event loop.
fn handle_key_event(
    app: &mut App,
    key: event::KeyEvent,
) -> Option<io::Result<Option<(String, Option<String>)>>> {
    if app.show_settings {
        match key.code {
            KeyCode::Enter => app.save_settings(),
            KeyCode::Esc => app.cancel_settings(),
            KeyCode::Char(c) => app.settings_add_char(c),
            KeyCode::Backspace => app.settings_pop_char(),
            _ => {}
        }
        return None;
    }

    if app.show_help {
        match key.code {
            KeyCode::Char('h') | KeyCode::Esc => app.toggle_help(),
            KeyCode::Up => app.help_scroll_up(1),
            KeyCode::Down => app.help_scroll_down(1),
            KeyCode::PageUp => app.help_scroll_up(10),
            KeyCode::PageDown => app.help_scroll_down(10),
            _ => {}
        }
        return None;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            if app.is_confirmation_pending() {
                app.cancel_confirmation();
            } else if app.show_search {
                app.show_search = false;
            } else {
                return Some(Ok(None));
            }
        }
        KeyCode::Char('f') if !app.show_search => {
            app.toggle_search();
        }
        KeyCode::Char('1') if !app.show_search => app.switch_to_tab(crate::app::Tab::Sessions),
        KeyCode::Char('2') if !app.show_search => app.switch_to_tab(crate::app::Tab::Trash),
        KeyCode::Tab if !app.show_search => app.switch_tab(),
        KeyCode::Up if !app.show_search => match app.focus {
            crate::app::FocusPanel::List => app.select_prev(),
            crate::app::FocusPanel::Preview => app.preview_scroll_up(1),
        },
        KeyCode::Down if !app.show_search => match app.focus {
            crate::app::FocusPanel::List => app.select_next(),
            crate::app::FocusPanel::Preview => app.preview_scroll_down(1),
        },
        KeyCode::Left if !app.show_search => app.focus_left(),
        KeyCode::Right if !app.show_search => app.focus_right(),
        KeyCode::PageDown if !app.show_search => app.page_down(10),
        KeyCode::PageUp if !app.show_search => app.page_up(10),
        KeyCode::Enter if !app.show_search => {
            app.switch_to_selected_session();
        }
        KeyCode::Char('d') if !app.show_search => {
            if app.is_confirmation_pending() {
                use crate::app::ConfirmAction;
                if let Some(action) = &app.confirm_action {
                    match action {
                        ConfirmAction::DeleteToTrash(_) => {
                            if let Some(session) = app.get_selected_session() {
                                let session_clone = session.clone();
                                match commands::delete_session(&session_clone) {
                                    Ok(_) => {
                                        app.move_selected_to_trash();
                                        app.confirm_action = None;
                                    }
                                    Err(_e) => {
                                        app.set_status("Delete failed".to_string());
                                        app.confirm_action = None;
                                    }
                                }
                            }
                        }
                        ConfirmAction::DeletePermanently(_) => {
                            app.confirm_and_execute();
                        }
                        ConfirmAction::EmptyTrash | ConfirmAction::TrashZeroMessages => {
                            app.confirm_and_execute();
                        }
                    }
                }
            } else {
                app.request_delete_confirmation();
            }
        }
        KeyCode::Char('y') if !app.show_search && app.is_confirmation_pending() => {
            use crate::app::ConfirmAction;
            if let Some(action) = &app.confirm_action {
                match action {
                    ConfirmAction::DeleteToTrash(_) => {
                        if let Some(session) = app.get_selected_session() {
                            let session_clone = session.clone();
                            match commands::delete_session(&session_clone) {
                                Ok(_) => {
                                    app.move_selected_to_trash();
                                    app.confirm_action = None;
                                }
                                Err(_e) => {
                                    app.set_status("Delete failed".to_string());
                                    app.confirm_action = None;
                                }
                            }
                        }
                    }
                    ConfirmAction::DeletePermanently(_)
                    | ConfirmAction::EmptyTrash
                    | ConfirmAction::TrashZeroMessages => {
                        app.confirm_and_execute();
                    }
                }
            }
        }
        KeyCode::Char('n') if !app.show_search && app.is_confirmation_pending() => {
            app.cancel_confirmation();
        }
        KeyCode::Char('r') if !app.show_search => {
            app.restore_selected_from_trash();
        }
        KeyCode::Char('0') if !app.show_search => {
            if app.is_confirmation_pending() {
                app.confirm_and_execute();
            } else {
                app.request_trash_zero_messages();
            }
        }
        KeyCode::Char('t') if !app.show_search => {
            if app.is_confirmation_pending() {
                app.confirm_and_execute();
            } else {
                app.request_empty_trash();
            }
        }
        KeyCode::Char('e') if !app.show_search => {
            if let Some(session) = app.get_selected_session() {
                let export_dir = app.config.resolved_export_path();
                let session_clone = session.clone();
                match commands::export_session(&session_clone, &export_dir) {
                    Ok(path) => {
                        app.set_status(format!("Exported to {}", path));
                    }
                    Err(_e) => {
                        app.set_status("Export failed".to_string());
                    }
                }
            }
        }
        KeyCode::Char('g') if !app.show_search => {
            app.open_settings();
        }
        KeyCode::Char('s') if !app.show_search => {
            app.toggle_sort();
            let sort_name = match app.sort_field {
                crate::app::SortField::Project => "project",
                crate::app::SortField::Messages => "messages",
                crate::app::SortField::Date => "date",
            };
            app.set_status(format!("Sorted by: {}", sort_name));
        }
        KeyCode::Char('S') if !app.show_search => {
            app.toggle_sort_direction();
            let dir_name = match app.sort_direction {
                crate::app::SortDirection::Ascending => "ascending",
                crate::app::SortDirection::Descending => "descending",
            };
            app.set_status(format!("Sort direction: {}", dir_name));
        }
        KeyCode::Char('h') if !app.show_search => {
            app.toggle_help();
        }
        _ if app.show_search => match key.code {
            KeyCode::Char(c) => app.add_search_char(c),
            KeyCode::Backspace => app.pop_search_char(),
            KeyCode::Esc => app.show_search = false,
            KeyCode::Enter => app.show_search = false,
            _ => {}
        },
        _ => {}
    }
    None
}

/// Gibt `true` zurück wenn die App beendet werden soll.
fn handle_mouse_event(app: &mut App, mouse: event::MouseEvent) -> bool {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            if app.show_help {
                app.help_scroll_up(3);
            } else if !app.show_settings {
                // Scroll folgt Mausposition statt Fokus-Panel
                let list_width = app.terminal_size.0 * 30 / 100;
                if mouse.column < list_width {
                    app.select_prev();
                } else {
                    app.preview_scroll_up(3);
                }
            }
        }
        MouseEventKind::ScrollDown => {
            if app.show_help {
                app.help_scroll_down(3);
            } else if !app.show_settings {
                let list_width = app.terminal_size.0 * 30 / 100;
                if mouse.column < list_width {
                    app.select_next();
                } else {
                    app.preview_scroll_down(3);
                }
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            let (col, row) = (mouse.column, mouse.row);
            if let Some(action) = app.get_click_action(col, row) {
                return dispatch_click_action(app, action);
            }
            // Click outside registrierter Region: Modal schließen
            if app.show_help {
                app.toggle_help();
                return false;
            }
            if app.show_settings {
                app.cancel_settings();
                return false;
            }
            if app.show_search {
                app.show_search = false;
                return false;
            }
            if app.is_confirmation_pending() {
                app.cancel_confirmation();
                return false;
            }
            // Normal-Modus: Listen-/Preview-Klick
            app.handle_list_click(col, row);
        }
        _ => {}
    }
    false
}

/// Führt eine ClickAction aus. Gibt `true` zurück wenn die App beendet werden soll.
fn dispatch_click_action(app: &mut App, action: crate::app::ClickAction) -> bool {
    use crate::app::ClickAction;
    match action {
        // Tab-Wechsel funktioniert immer
        ClickAction::SwitchTab(tab) => app.switch_to_tab(tab),
        // Modal-Aktionen: Settings Save/Cancel, Confirm Yes/No
        ClickAction::SaveSettings => app.save_settings(),
        ClickAction::CancelSettings => app.cancel_settings(),
        ClickAction::ConfirmYes => {
            // Gleiche Logik wie 'y'-Taste
            use crate::app::ConfirmAction;
            if let Some(action) = &app.confirm_action {
                match action {
                    ConfirmAction::DeleteToTrash(_) => {
                        if let Some(session) = app.get_selected_session() {
                            let session_clone = session.clone();
                            match commands::delete_session(&session_clone) {
                                Ok(_) => {
                                    app.move_selected_to_trash();
                                    app.confirm_action = None;
                                }
                                Err(_) => {
                                    app.set_status("Delete failed".to_string());
                                    app.confirm_action = None;
                                }
                            }
                        }
                    }
                    ConfirmAction::DeletePermanently(_)
                    | ConfirmAction::EmptyTrash
                    | ConfirmAction::TrashZeroMessages => {
                        app.confirm_and_execute();
                    }
                }
            }
        }
        ClickAction::ConfirmNo => app.cancel_confirmation(),
        // Normal-Modus-Aktionen: nur wenn kein Modal offen
        _ => {
            if app.show_settings || app.show_help || app.is_confirmation_pending() {
                return false;
            }
            match action {
                ClickAction::ResumeSession => app.switch_to_selected_session(),
                ClickAction::DeleteSession => app.request_delete_confirmation(),
                ClickAction::ExportSession => {
                    if let Some(session) = app.get_selected_session() {
                        let export_dir = app.config.resolved_export_path();
                        let session_clone = session.clone();
                        match commands::export_session(&session_clone, &export_dir) {
                            Ok(path) => app.set_status(format!("Exported to {}", path)),
                            Err(_) => app.set_status("Export failed".to_string()),
                        }
                    }
                }
                ClickAction::CleanZeroMessages => app.request_trash_zero_messages(),
                ClickAction::ToggleSearch => app.toggle_search(),
                ClickAction::ToggleSort => {
                    app.toggle_sort();
                    let sort_name = match app.sort_field {
                        crate::app::SortField::Project => "project",
                        crate::app::SortField::Messages => "messages",
                        crate::app::SortField::Date => "date",
                    };
                    app.set_status(format!("Sorted by: {}", sort_name));
                }
                ClickAction::OpenSettings => app.open_settings(),
                ClickAction::ToggleHelp => app.toggle_help(),
                ClickAction::Quit => return true,
                ClickAction::RestoreFromTrash => app.restore_selected_from_trash(),
                ClickAction::EmptyTrash => app.request_empty_trash(),
                _ => {} // Modal-Aktionen bereits oben behandelt
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use crate::app::{FocusPanel, Tab};
    use crate::models::{Message, Session};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Führt einen draw-Zyklus durch, um Click-Regionen zu füllen (Single Source of Truth).
    fn render_frame(app: &mut App, width: u16, height: u16) {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::draw(f, app)).unwrap();
    }

    fn make_session(id: &str, project: &str) -> Session {
        Session {
            id: id.to_string(),
            project_path: format!("/home/g/{}", project),
            project_name: project.to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            size: 0,
            total_entries: 1,
            messages: vec![Message {
                role: "user".to_string(),
                content: "msg".to_string(),
            }],
        }
    }

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }


    // --- cmd_quote / shell_escape ---

    #[cfg(target_os = "windows")]
    #[test]
    fn test_cmd_quote_simple() {
        assert_eq!(cmd_quote("C:\\Users\\test"), "\"C:\\Users\\test\"");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_cmd_quote_with_quotes() {
        assert_eq!(
            cmd_quote("path with \"quotes\""),
            "\"path with \"\"quotes\"\"\""
        );
    }

    #[cfg(target_family = "unix")]
    #[test]
    fn test_shell_escape_simple() {
        assert_eq!(shell_escape("/home/user/project"), "'/home/user/project'");
    }

    #[cfg(target_family = "unix")]
    #[test]
    fn test_shell_escape_with_quotes() {
        assert_eq!(shell_escape("it's a path"), "'it'\\''s a path'");
    }

    // --- handle_key_event ---

    #[test]
    fn test_handle_q_quits() {
        let mut app = App::with_sessions(vec![]);
        let result = handle_key_event(&mut app, press(KeyCode::Char('q')));
        assert!(result.is_some());
    }

    #[test]
    fn test_handle_tab_switches_tab() {
        let mut app = App::with_sessions(vec![]);
        handle_key_event(&mut app, press(KeyCode::Tab));
        assert_eq!(app.current_tab, Tab::Trash);
    }

    #[test]
    fn test_handle_s_toggles_sort() {
        let mut app = App::with_sessions(vec![]);
        let initial = app.sort_field;
        handle_key_event(&mut app, press(KeyCode::Char('s')));
        assert_ne!(app.sort_field, initial);
    }

    #[test]
    fn test_handle_h_toggles_help() {
        let mut app = App::with_sessions(vec![]);
        handle_key_event(&mut app, press(KeyCode::Char('h')));
        assert!(app.show_help);
    }

    #[test]
    fn test_handle_g_opens_settings() {
        let mut app = App::with_sessions(vec![]);
        handle_key_event(&mut app, press(KeyCode::Char('g')));
        assert!(app.show_settings);
    }

    #[test]
    fn test_handle_f_toggles_search() {
        let mut app = App::with_sessions(vec![]);
        handle_key_event(&mut app, press(KeyCode::Char('f')));
        assert!(app.show_search);
    }

    #[test]
    fn test_handle_1_switches_to_sessions() {
        let mut app = App::with_sessions(vec![]);
        app.current_tab = crate::app::Tab::Trash;
        handle_key_event(&mut app, press(KeyCode::Char('1')));
        assert_eq!(app.current_tab, crate::app::Tab::Sessions);
    }

    #[test]
    fn test_handle_2_switches_to_trash() {
        let mut app = App::with_sessions(vec![]);
        handle_key_event(&mut app, press(KeyCode::Char('2')));
        assert_eq!(app.current_tab, crate::app::Tab::Trash);
    }

    fn mouse(kind: MouseEventKind, col: u16, row: u16) -> event::MouseEvent {
        event::MouseEvent {
            kind,
            column: col,
            row,
            modifiers: event::KeyModifiers::NONE,
        }
    }

    #[test]
    fn test_mouse_scroll_down_selects_next() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
        ]);
        app.terminal_size = (160, 40);
        // col=10 liegt im Listen-Bereich (< 160*30/100=48) → scrollt Liste
        handle_mouse_event(&mut app, mouse(MouseEventKind::ScrollDown, 10, 10));
        assert_eq!(app.selected_session_idx, 1);
    }

    #[test]
    fn test_mouse_scroll_up_selects_prev() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
        ]);
        app.terminal_size = (160, 40);
        app.selected_session_idx = 1;
        handle_mouse_event(&mut app, mouse(MouseEventKind::ScrollUp, 10, 10));
        assert_eq!(app.selected_session_idx, 0);
    }

    #[test]
    fn test_mouse_click_selects_session_row() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
            make_session("s3", "p3"),
        ]);
        app.terminal_size = (160, 40);
        render_frame(&mut app, 160, 40);
        // row=6 → list_data_row=1 → sessions[offset+1]
        handle_mouse_event(&mut app, mouse(MouseEventKind::Down(MouseButton::Left), 10, 6));
        assert_eq!(app.selected_session_idx, 1);
    }

    #[test]
    fn test_mouse_click_tab_bar_switches_to_trash() {
        let mut app = App::with_sessions(vec![]);
        // Sessions-Tab: "  ● 1 Sessions (0)  " = 20 Zeichen → cols 0-20
        // Trash-Tab: "  ○ 2 Trash (0)  " = 18 Zeichen → cols 21-38
        render_frame(&mut app, 160, 40);
        handle_mouse_event(&mut app, mouse(MouseEventKind::Down(MouseButton::Left), 25, 1));
        assert_eq!(app.current_tab, crate::app::Tab::Trash);
    }

    #[test]
    fn test_mouse_click_tab_bar_help_opens_help() {
        let mut app = App::with_sessions(vec![]);
        // Help-Hint: "│  h help  " = 11 Zeichen → cols 39-49
        render_frame(&mut app, 160, 40);
        assert!(!app.show_help);
        handle_mouse_event(&mut app, mouse(MouseEventKind::Down(MouseButton::Left), 42, 1));
        assert!(app.show_help);
    }

    #[test]
    fn test_mouse_click_tab_bar_switches_to_sessions() {
        let mut app = App::with_sessions(vec![]);
        app.current_tab = crate::app::Tab::Trash;
        // col=10 liegt im Sessions-Tab (cols 0-20)
        render_frame(&mut app, 160, 40);
        handle_mouse_event(&mut app, mouse(MouseEventKind::Down(MouseButton::Left), 10, 1));
        assert_eq!(app.current_tab, crate::app::Tab::Sessions);
    }

    #[test]
    fn test_mouse_click_command_bar_opens_settings() {
        let mut app = App::with_sessions(vec![]);
        render_frame(&mut app, 160, 40);
        // "g settings" startet bei x=82 (nav=16 + sep=5 + Enter+resume=14 + d+delete=10
        // + e+export=10 + 0+clean=9 + f+search=10 + s+sort=8 = 82), Breite=12
        // cmd_y = 40-3 = 37 → row=38 liegt in height:3
        handle_mouse_event(&mut app, mouse(MouseEventKind::Down(MouseButton::Left), 85, 38));
        assert!(app.show_settings);
    }

    #[test]
    fn test_mouse_scroll_in_preview_scrolls_content() {
        let mut app = App::with_sessions(vec![]);
        app.terminal_size = (160, 40);
        app.preview_scroll = 10;
        // col=100 liegt im Preview-Bereich (>= 160*30/100=48) → scrollt Preview
        handle_mouse_event(&mut app, mouse(MouseEventKind::ScrollUp, 100, 10));
        assert!(app.preview_scroll < 10);
    }

    // --- 100% Maus-Support Tests ---

    #[test]
    fn test_click_outside_closes_help() {
        let mut app = App::with_sessions(vec![]);
        app.terminal_size = (160, 40);
        app.toggle_help();
        assert!(app.show_help);
        render_frame(&mut app, 160, 40);
        // Klick auf (0, 0) — keine Regionen registriert bei show_help → click-outside
        handle_mouse_event(&mut app, mouse(MouseEventKind::Down(MouseButton::Left), 0, 0));
        assert!(!app.show_help);
    }

    #[test]
    fn test_click_outside_closes_settings() {
        let mut app = App::with_sessions(vec![]);
        app.terminal_size = (160, 40);
        app.open_settings();
        assert!(app.show_settings);
        render_frame(&mut app, 160, 40);
        // Klick auf (0, 0) — außerhalb des Settings-Modals
        handle_mouse_event(&mut app, mouse(MouseEventKind::Down(MouseButton::Left), 0, 0));
        assert!(!app.show_settings);
    }

    #[test]
    fn test_click_outside_closes_search() {
        let mut app = App::with_sessions(vec![]);
        app.terminal_size = (160, 40);
        app.toggle_search();
        assert!(app.show_search);
        render_frame(&mut app, 160, 40);
        // Klick auf (0, 0) — keine Regionen bei show_search → click-outside
        handle_mouse_event(&mut app, mouse(MouseEventKind::Down(MouseButton::Left), 0, 0));
        assert!(!app.show_search);
    }

    #[test]
    fn test_click_outside_cancels_confirmation() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.terminal_size = (160, 40);
        app.request_delete_confirmation();
        assert!(app.is_confirmation_pending());
        render_frame(&mut app, 160, 40);
        // Klick auf (0, 0) — außerhalb der [y]/[n] Buttons
        handle_mouse_event(&mut app, mouse(MouseEventKind::Down(MouseButton::Left), 0, 0));
        assert!(!app.is_confirmation_pending());
    }

    #[test]
    fn test_mouse_scroll_in_help_modal() {
        let mut app = App::with_sessions(vec![]);
        app.terminal_size = (160, 40);
        app.toggle_help();
        app.help_scroll = 5;
        // Scroll down → help_scroll steigt
        handle_mouse_event(&mut app, mouse(MouseEventKind::ScrollDown, 80, 20));
        assert_eq!(app.help_scroll, 8); // 5 + 3
        // Scroll up → help_scroll sinkt
        handle_mouse_event(&mut app, mouse(MouseEventKind::ScrollUp, 80, 20));
        assert_eq!(app.help_scroll, 5); // 8 - 3
    }

    #[test]
    fn test_scroll_follows_mouse_position_not_focus() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
        ]);
        app.terminal_size = (160, 40);
        // Fokus auf Preview, aber Maus über Liste (col=10 < 48)
        app.focus = crate::app::FocusPanel::Preview;
        handle_mouse_event(&mut app, mouse(MouseEventKind::ScrollDown, 10, 10));
        // Liste wurde gescrollt, nicht Preview
        assert_eq!(app.selected_session_idx, 1);
        assert_eq!(app.preview_scroll, 0);
    }

    #[test]
    fn test_settings_save_click() {
        let mut app = App::with_sessions(vec![]);
        app.terminal_size = (160, 40);
        app.open_settings();
        app.settings_input = "/new/path".to_string();
        render_frame(&mut app, 160, 40);
        // Settings-Modal: popup_width = 160*0.6 = 96, popup_x = 32, inner_x = 33
        // btn_y = (40-7)/2 + 1 + 4 = 16 + 1 + 4 = 21
        // Save-Button: x=33, width=16, y=21
        handle_mouse_event(&mut app, mouse(MouseEventKind::Down(MouseButton::Left), 40, 21));
        assert!(!app.show_settings);
        assert_eq!(app.config.export_path, "/new/path");
    }

    #[test]
    fn test_settings_cancel_click() {
        let mut app = App::with_sessions(vec![]);
        app.terminal_size = (160, 40);
        app.open_settings();
        let original = app.config.export_path.clone();
        app.settings_input = "/changed".to_string();
        render_frame(&mut app, 160, 40);
        // Cancel-Button: x=33+16=49, width=12, y=21
        handle_mouse_event(&mut app, mouse(MouseEventKind::Down(MouseButton::Left), 52, 21));
        assert!(!app.show_settings);
        assert_eq!(app.config.export_path, original);
    }

    #[test]
    fn test_confirm_yes_click() {
        let mut app = App::with_sessions(vec![]);
        app.trash = vec![make_session("t1", "p1")];
        app.current_tab = crate::app::Tab::Trash;
        app.terminal_size = (160, 40);
        app.confirm_action = Some(crate::app::ConfirmAction::DeletePermanently("t1".to_string()));
        app.set_status("PERMANENTLY delete 'p1'? Press 'd' or 'y' to confirm, 'n' or Esc to cancel".to_string());
        render_frame(&mut app, 160, 40);
        // [y] Button: nach Question-Text + 2 Zeichen Abstand
        // "PERMANENTLY delete 'p1'?" = 24 Zeichen + 2 = 26 → x=26
        let action = app.get_click_action(28, 38);
        assert_eq!(action, Some(crate::app::ClickAction::ConfirmYes));
        handle_mouse_event(&mut app, mouse(MouseEventKind::Down(MouseButton::Left), 28, 38));
        assert!(app.trash.is_empty());
    }

    #[test]
    fn test_confirm_no_click() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.terminal_size = (160, 40);
        app.request_delete_confirmation();
        assert!(app.is_confirmation_pending());
        render_frame(&mut app, 160, 40);
        // [n] Button: nach [y] Button + 2 Zeichen
        // Für diesen Test: prüfen dass ConfirmNo Region existiert
        let has_confirm_no = app.click_regions.iter().any(|(_, a)| *a == crate::app::ClickAction::ConfirmNo);
        assert!(has_confirm_no);
        // Klick auf die [n] Region
        let (rx, ry) = {
            let (rect, _) = app.click_regions.iter().find(|(_, a)| *a == crate::app::ClickAction::ConfirmNo).unwrap();
            (rect.x + 1, rect.y)
        };
        handle_mouse_event(&mut app, mouse(MouseEventKind::Down(MouseButton::Left), rx, ry));
        assert!(!app.is_confirmation_pending());
    }

    #[test]
    fn test_no_normal_regions_during_modals() {
        let mut app = App::with_sessions(vec![]);
        // Help-Modal: keine Regionen registriert
        app.toggle_help();
        render_frame(&mut app, 160, 40);
        assert!(app.click_regions.is_empty());

        // Search-Modal: keine Regionen
        app.toggle_help();
        app.toggle_search();
        render_frame(&mut app, 160, 40);
        assert!(app.click_regions.is_empty());

        // Settings-Modal: nur Save/Cancel Regionen
        app.show_search = false;
        app.open_settings();
        render_frame(&mut app, 160, 40);
        assert_eq!(app.click_regions.len(), 2);
        assert!(app.click_regions.iter().any(|(_, a)| *a == crate::app::ClickAction::SaveSettings));
        assert!(app.click_regions.iter().any(|(_, a)| *a == crate::app::ClickAction::CancelSettings));

        // Confirmation: nur [y]/[n] Regionen
        app.cancel_settings();
        app.sessions = vec![make_session("s1", "p1")];
        app.request_delete_confirmation();
        render_frame(&mut app, 160, 40);
        assert_eq!(app.click_regions.len(), 2);
        assert!(app.click_regions.iter().any(|(_, a)| *a == crate::app::ClickAction::ConfirmYes));
        assert!(app.click_regions.iter().any(|(_, a)| *a == crate::app::ClickAction::ConfirmNo));
    }

    #[test]
    fn test_scroll_blocked_during_settings() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
        ]);
        app.terminal_size = (160, 40);
        app.open_settings();
        handle_mouse_event(&mut app, mouse(MouseEventKind::ScrollDown, 10, 10));
        // Scroll wird blockiert — Liste unverändert
        assert_eq!(app.selected_session_idx, 0);
    }

    #[test]
    fn test_handle_arrows_in_list() {
        let mut app =
            App::with_sessions(vec![make_session("s1", "p1"), make_session("s2", "p2")]);
        handle_key_event(&mut app, press(KeyCode::Down));
        assert_eq!(app.selected_session_idx, 1);
        handle_key_event(&mut app, press(KeyCode::Up));
        assert_eq!(app.selected_session_idx, 0);
    }

    #[test]
    fn test_handle_arrows_in_preview() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.focus = FocusPanel::Preview;
        handle_key_event(&mut app, press(KeyCode::Down));
        assert_eq!(app.preview_scroll, 1);
        handle_key_event(&mut app, press(KeyCode::Up));
        assert_eq!(app.preview_scroll, 0);
    }

    #[test]
    fn test_handle_esc_cancels_confirmation() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.request_delete_confirmation();
        handle_key_event(&mut app, press(KeyCode::Esc));
        assert!(!app.is_confirmation_pending());
    }

    #[test]
    fn test_handle_search_mode_chars() {
        let mut app = App::with_sessions(vec![]);
        app.show_search = true;
        handle_key_event(&mut app, press(KeyCode::Char('a')));
        handle_key_event(&mut app, press(KeyCode::Char('b')));
        assert_eq!(app.search_query, "ab");
        handle_key_event(&mut app, press(KeyCode::Backspace));
        assert_eq!(app.search_query, "a");
    }

    #[test]
    fn test_handle_settings_mode() {
        let mut app = App::with_sessions(vec![]);
        app.open_settings();
        let initial_len = app.settings_input.len();
        handle_key_event(&mut app, press(KeyCode::Char('x')));
        assert_eq!(app.settings_input.len(), initial_len + 1);
        handle_key_event(&mut app, press(KeyCode::Esc));
        assert!(!app.show_settings);
    }

    #[test]
    fn test_handle_help_mode_scroll() {
        let mut app = App::with_sessions(vec![]);
        app.toggle_help();
        handle_key_event(&mut app, press(KeyCode::Down));
        assert_eq!(app.help_scroll, 1);
        handle_key_event(&mut app, press(KeyCode::PageDown));
        assert_eq!(app.help_scroll, 11);
    }

    #[test]
    fn test_handle_n_cancels_confirmation() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.request_delete_confirmation();
        handle_key_event(&mut app, press(KeyCode::Char('n')));
        assert!(!app.is_confirmation_pending());
    }

    // --- Settings mode: Enter saves, Backspace deletes ---

    #[test]
    fn test_handle_settings_enter_saves() {
        let mut app = App::with_sessions(vec![]);
        app.open_settings();
        app.settings_input = "/new/path".to_string();
        handle_key_event(&mut app, press(KeyCode::Enter));
        assert!(!app.show_settings);
        assert_eq!(app.config.export_path, "/new/path");
    }

    #[test]
    fn test_handle_settings_backspace() {
        let mut app = App::with_sessions(vec![]);
        app.open_settings();
        let initial_len = app.settings_input.len();
        handle_key_event(&mut app, press(KeyCode::Backspace));
        assert_eq!(app.settings_input.len(), initial_len - 1);
    }

    // --- Help mode: PageUp ---

    #[test]
    fn test_handle_help_page_up() {
        let mut app = App::with_sessions(vec![]);
        app.toggle_help();
        app.help_scroll = 15;
        handle_key_event(&mut app, press(KeyCode::PageUp));
        assert_eq!(app.help_scroll, 5);
    }

    #[test]
    fn test_handle_help_up_scrolls() {
        let mut app = App::with_sessions(vec![]);
        app.toggle_help();
        app.help_scroll = 5;
        handle_key_event(&mut app, press(KeyCode::Up));
        assert_eq!(app.help_scroll, 4);
    }

    #[test]
    fn test_handle_help_esc_closes() {
        let mut app = App::with_sessions(vec![]);
        app.toggle_help();
        handle_key_event(&mut app, press(KeyCode::Esc));
        assert!(!app.show_help);
    }

    // --- Sort direction 'S' ---

    #[test]
    fn test_handle_shift_s_toggles_sort_direction() {
        let mut app = App::with_sessions(vec![]);
        assert_eq!(app.sort_direction, crate::app::SortDirection::Descending);
        handle_key_event(&mut app, press(KeyCode::Char('S')));
        assert_eq!(app.sort_direction, crate::app::SortDirection::Ascending);
    }

    // --- Left/Right focus ---

    #[test]
    fn test_handle_right_switches_to_preview() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        handle_key_event(&mut app, press(KeyCode::Right));
        assert_eq!(app.focus, FocusPanel::Preview);
    }

    #[test]
    fn test_handle_left_switches_to_list() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.focus = FocusPanel::Preview;
        handle_key_event(&mut app, press(KeyCode::Left));
        assert_eq!(app.focus, FocusPanel::List);
    }

    // --- PageDown/PageUp in list ---

    #[test]
    fn test_handle_pagedown_in_list() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
            make_session("s3", "p3"),
        ]);
        handle_key_event(&mut app, press(KeyCode::PageDown));
        assert_eq!(app.selected_session_idx, 2);
    }

    #[test]
    fn test_handle_pageup_in_list() {
        let mut app = App::with_sessions(vec![
            make_session("s1", "p1"),
            make_session("s2", "p2"),
            make_session("s3", "p3"),
        ]);
        app.selected_session_idx = 2;
        handle_key_event(&mut app, press(KeyCode::PageUp));
        assert_eq!(app.selected_session_idx, 0);
    }

    // --- 'r' restore ---

    #[test]
    fn test_handle_r_restore_in_sessions_tab() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        handle_key_event(&mut app, press(KeyCode::Char('r')));
        // restore in sessions tab shows error status
        assert!(app.status_message.unwrap().contains("Trash tab"));
    }

    // --- 't' empty trash ---

    #[test]
    fn test_handle_t_requests_empty_trash() {
        let mut app = App::with_sessions(vec![]);
        app.trash = vec![make_session("t1", "p1")];
        app.current_tab = Tab::Trash;
        handle_key_event(&mut app, press(KeyCode::Char('t')));
        assert_eq!(
            app.confirm_action,
            Some(crate::app::ConfirmAction::EmptyTrash)
        );
    }

    #[test]
    fn test_handle_t_confirms_when_pending() {
        let mut app = App::with_sessions(vec![]);
        app.trash = vec![make_session("t1", "p1")];
        app.current_tab = Tab::Trash;
        app.confirm_action = Some(crate::app::ConfirmAction::EmptyTrash);
        handle_key_event(&mut app, press(KeyCode::Char('t')));
        assert!(app.trash.is_empty());
    }

    // --- '0' trash zero messages ---

    #[test]
    fn test_handle_0_requests_trash_zero() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.sessions[0].messages.clear();
        handle_key_event(&mut app, press(KeyCode::Char('0')));
        assert_eq!(
            app.confirm_action,
            Some(crate::app::ConfirmAction::TrashZeroMessages)
        );
    }

    // --- 'd' in confirm mode (DeleteToTrash) ---

    #[test]
    fn test_handle_d_first_press_requests_confirmation() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        handle_key_event(&mut app, press(KeyCode::Char('d')));
        assert!(app.is_confirmation_pending());
    }

    // --- 'y' confirms DeletePermanently ---

    #[test]
    fn test_handle_y_confirms_delete_permanently() {
        let mut app = App::with_sessions(vec![]);
        app.trash = vec![make_session("t1", "p1")];
        app.current_tab = Tab::Trash;
        app.confirm_action = Some(crate::app::ConfirmAction::DeletePermanently("t1".to_string()));
        handle_key_event(&mut app, press(KeyCode::Char('y')));
        assert!(app.trash.is_empty());
        assert!(!app.is_confirmation_pending());
    }

    // --- Esc/Enter in search mode ---

    #[test]
    fn test_handle_esc_closes_search() {
        let mut app = App::with_sessions(vec![]);
        app.show_search = true;
        app.search_query = "test".to_string();
        handle_key_event(&mut app, press(KeyCode::Esc));
        assert!(!app.show_search);
    }

    #[test]
    fn test_handle_enter_closes_search() {
        let mut app = App::with_sessions(vec![]);
        app.show_search = true;
        app.search_query = "test".to_string();
        handle_key_event(&mut app, press(KeyCode::Enter));
        assert!(!app.show_search);
    }

    // --- 'q' in search mode closes search instead of quitting ---

    #[test]
    fn test_handle_q_in_search_closes_search() {
        let mut app = App::with_sessions(vec![]);
        app.show_search = true;
        let result = handle_key_event(&mut app, press(KeyCode::Char('q')));
        assert!(result.is_none()); // should NOT quit
        assert!(!app.show_search);
    }

    // --- Enter switches to session ---

    #[test]
    fn test_handle_enter_switches_session() {
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        handle_key_event(&mut app, press(KeyCode::Enter));
        assert!(app.resume_session_id.is_some());
    }

    // --- 'e' export ---

    #[test]
    fn test_handle_e_export() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut app = App::with_sessions(vec![make_session("s1", "p1")]);
        app.config.export_path = tmp.path().to_string_lossy().to_string();
        handle_key_event(&mut app, press(KeyCode::Char('e')));
        assert!(app.status_message.unwrap().contains("Exported"));
    }
}

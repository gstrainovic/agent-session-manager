mod app;
mod commands;
mod models;
mod store;
mod ui;

use app::App;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::error::Error;
use std::io;
use std::os::unix::process::CommandExt;

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
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
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    match res {
        Ok(Some((command, path))) => {
            // Build shell command: clear && cd <project> && claude --resume <id>
            let session_id = command.strip_prefix("claude --resume ").unwrap_or_default();

            let shell_cmd = match path {
                Some(project_path) => format!(
                    "clear && cd {} && claude --resume {}",
                    shell_escape(&project_path),
                    session_id
                ),
                None => format!("clear && claude --resume {}", session_id),
            };

            // exec() replaces the current process on success, only returns on error
            let exec_error = std::process::Command::new("sh")
                .args(["-c", &shell_cmd])
                .exec();
            eprintln!("Failed to execute 'claude --resume': {}", exec_error);
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("{:?}", err);
        }
        Ok(None) => {}
    }

    Ok(())
}

/// Escapes a string for safe use in shell commands
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<Option<(String, Option<String>)>> {
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;
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
                if let Event::Key(key) = event::read()? {
                    if let Some(result) = handle_key_event(&mut app, key) {
                        return result;
                    }
                }
                // Check if more events are immediately available
                if !crossterm::event::poll(std::time::Duration::from_millis(0))? {
                    break;
                }
            }
        }
    }
}

fn handle_key_event(
    app: &mut App,
    key: event::KeyEvent,
) -> Option<io::Result<Option<(String, Option<String>)>>> {
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
        KeyCode::Char('f') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.toggle_search();
        }
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
        KeyCode::Char('E') if !app.show_search => {
            if app.is_confirmation_pending() {
                app.confirm_and_execute();
            } else {
                app.request_empty_trash();
            }
        }
        KeyCode::Char('e') if !app.show_search => {
            if let Some(session) = app.get_selected_session() {
                match commands::export_session(session) {
                    Ok(path) => {
                        app.set_status(format!("Exported to {}", path));
                    }
                    Err(_e) => {
                        app.set_status("Export failed".to_string());
                    }
                }
            }
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

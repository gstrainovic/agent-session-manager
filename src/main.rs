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

    let app = App::new();
    let res = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    match res {
        Ok(Some((command, path))) => {
            // Execute the resume command
            let mut cmd = std::process::Command::new("claude");
            cmd.args(vec!["--resume"]);
            
            // Parse session ID from command (format: "claude --resume <id>")
            if let Some(session_id) = command.strip_prefix("claude --resume ") {
                cmd.arg(session_id);
            }
            
            // Change to project directory if available
            if let Some(project_path) = path {
                cmd.current_dir(project_path);
            }
            
            let _ = cmd.exec();
        }
        Err(err) => {
            eprintln!("{:?}", err);
        }
        Ok(None) => {}
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<Option<(String, Option<String>)>> {
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;
        app.clear_expired_status();

        // Check if we should resume a session
        if let Some(command) = app.get_resume_command() {
            let path = app.get_resume_session_path();
            return Ok(Some((command, path)));
        }

        if crossterm::event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        if app.is_confirmation_pending() {
                            app.cancel_confirmation();
                        } else if app.show_search {
                            app.show_search = false;
                        } else {
                            return Ok(None);
                        }
                    }
                    KeyCode::Char('f')
                        if key.modifiers.contains(event::KeyModifiers::CONTROL) =>
                    {
                        app.toggle_search();
                    }
                    KeyCode::Tab if !app.show_search => app.switch_tab(),
                    KeyCode::Up if !app.show_search => app.select_prev(),
                    KeyCode::Down if !app.show_search => app.select_next(),
                    KeyCode::PageDown if !app.show_search => app.scroll_preview_down(),
                    KeyCode::PageUp if !app.show_search => app.scroll_preview_up(),
                    KeyCode::Enter if !app.show_search => {
                        app.switch_to_selected_session();
                    }
                    KeyCode::Char('d') if !app.show_search => {
                        if app.is_confirmation_pending() {
                            // Confirmation pending - check action type
                            use crate::app::ConfirmAction;
                            if let Some(action) = &app.confirm_action {
                                match action {
                                    ConfirmAction::DeleteToTrash(_) => {
                                        // Delete from sessions and move to trash
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
                                    ConfirmAction::EmptyTrash => {
                                        app.confirm_and_execute();
                                    }
                                }
                            }
                        } else {
                            // No confirmation pending - request confirmation
                            app.request_delete_confirmation();
                        }
                    }
                    KeyCode::Char('y') if !app.show_search && app.is_confirmation_pending() => {
                        // Confirm with 'y' - same logic as 'd'
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
                                ConfirmAction::DeletePermanently(_) | ConfirmAction::EmptyTrash => {
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
                    KeyCode::Char('E') if !app.show_search => {
                        if app.is_confirmation_pending() {
                            // Confirm empty trash with 'E'
                            app.confirm_and_execute();
                        } else {
                            // Request empty trash confirmation
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
            }
        }
    }
}

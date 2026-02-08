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

    if let Err(err) = res {
        eprintln!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if crossterm::event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        if app.is_delete_pending() {
                            app.cancel_delete_confirmation();
                        } else if app.show_search {
                            app.show_search = false;
                        } else {
                            return Ok(());
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
                        if app.is_delete_pending() {
                            // Confirmation pending - execute delete
                            if let Some(session) = app.get_selected_session() {
                                let session_clone = session.clone();
                                match commands::delete_session(&session_clone) {
                                    Ok(_) => {
                                        app.move_selected_to_trash();
                                        app.confirm_delete = None;
                                    }
                                    Err(_e) => {
                                        app.status_message =
                                            Some("Delete failed".to_string());
                                        app.confirm_delete = None;
                                    }
                                }
                            }
                        } else {
                            // No confirmation pending - request confirmation
                            app.request_delete_confirmation();
                        }
                    }
                    KeyCode::Char('y') if !app.show_search && app.is_delete_pending() => {
                        // Confirm delete with 'y'
                        if let Some(session) = app.get_selected_session() {
                            let session_clone = session.clone();
                            match commands::delete_session(&session_clone) {
                                Ok(_) => {
                                    app.move_selected_to_trash();
                                    app.confirm_delete = None;
                                }
                                Err(_e) => {
                                    app.status_message =
                                        Some("Delete failed".to_string());
                                    app.confirm_delete = None;
                                }
                            }
                        }
                    }
                    KeyCode::Char('n') if !app.show_search && app.is_delete_pending() => {
                        app.cancel_delete_confirmation();
                    }
                    KeyCode::Char('r') if !app.show_search => {
                        app.restore_selected_from_trash();
                    }
                    KeyCode::Char('e') if !app.show_search => {
                        if let Some(session) = app.get_selected_session() {
                            match commands::export_session(session) {
                                Ok(path) => {
                                    app.status_message =
                                        Some(format!("Exported to {}", path));
                                }
                                Err(_e) => {
                                    app.status_message =
                                        Some("Export failed".to_string());
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

// src/main.rs

mod app;
mod commands;
mod models;
mod store;
mod ui;

use app::App;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::error::Error;
use std::io;

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // Create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
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
                    KeyCode::Char('q') | KeyCode::Esc => {
                        if app.show_search {
                            app.show_search = false;
                        } else {
                            return Ok(());
                        }
                    }
                    KeyCode::Char('f') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        app.toggle_search();
                    }
                    KeyCode::Tab if !app.show_search => app.switch_tab(),
                    KeyCode::Up if !app.show_search => app.select_prev(),
                    KeyCode::Down if !app.show_search => app.select_next(),
                    KeyCode::PageDown if !app.show_search => app.scroll_preview_down(),
                    KeyCode::PageUp if !app.show_search => app.scroll_preview_up(),
                    KeyCode::Char('d') if !app.show_search => {
                        if let Some(session) = app.get_selected_session() {
                            match commands::delete_session(session) {
                                Ok(_) => {
                                    // Success - session would be moved to trash in full implementation
                                }
                                Err(e) => {
                                    eprintln!("Delete error: {}", e);
                                }
                            }
                        }
                    }
                    KeyCode::Char('r') if !app.show_search => println!("Restore pressed"),
                    KeyCode::Char('s') if !app.show_search => println!("Switch pressed"),
                    KeyCode::Char('e') if !app.show_search => println!("Export pressed"),
                    _ if app.show_search => {
                        match key.code {
                            KeyCode::Char(c) => app.add_search_char(c),
                            KeyCode::Backspace => app.pop_search_char(),
                            KeyCode::Esc => app.show_search = false,
                            KeyCode::Enter => app.show_search = false,
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

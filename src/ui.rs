// src/ui.rs

use crate::app::{App, Tab};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
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
        .split(f.area());

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

    // Draw search modal if active
    draw_search_modal(f, app);
}

fn draw_tabs(f: &mut Frame, area: Rect, app: &App) {
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

fn draw_list(f: &mut Frame, area: Rect, app: &App) {
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

fn draw_preview(f: &mut Frame, area: Rect, app: &App) {
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

fn draw_search_modal(f: &mut Frame, app: &App) {
    if !app.show_search {
        return;
    }

    let size = f.area();
    let area = Rect {
        x: size.width / 4,
        y: size.height / 2,
        width: size.width / 2,
        height: 3,
    };

    let search = Paragraph::new(format!("Search: {}_", app.search_query))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().bg(Color::Black).fg(Color::White));

    f.render_widget(search, area);
}

fn draw_commands(f: &mut Frame, area: Rect) {
    let commands = Paragraph::new("[d]elete  [r]estore  [s]witch  [e]xport  | Search: [Ctrl+F]")
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::TOP));

    f.render_widget(commands, area);
}

use crate::app::{App, Tab};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.area());

    draw_tabs(f, chunks[0], app);

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    draw_list(f, content_chunks[0], app);
    draw_preview(f, content_chunks[1], app);

    draw_commands(f, chunks[2], app);

    if app.show_search {
        draw_search_modal(f, app);
    }
}

fn draw_tabs(f: &mut Frame, area: Rect, app: &App) {
    let session_count = app.sessions.len();
    let trash_count = app.trash.len();

    let sessions_style = if app.current_tab == Tab::Sessions {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let trash_style = if app.current_tab == Tab::Trash {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let tabs = vec![
        Span::styled(format!("  Sessions ({})  ", session_count), sessions_style),
        Span::styled(format!("  Trash ({})  ", trash_count), trash_style),
        Span::styled(
            "  [Tab] to switch  ",
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

fn draw_list(f: &mut Frame, area: Rect, app: &App) {
    let filtered = app.filtered_sessions();

    let header = Row::new(vec![
        Cell::from("Project").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("ID").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Msgs").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ])
    .bottom_margin(0);

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(idx, session)| {
            let style = if idx == app.selected_session_idx {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let short_id = if session.id.len() > 8 {
                &session.id[..8]
            } else {
                &session.id
            };

            Row::new(vec![
                Cell::from(session.project_name.as_str()),
                Cell::from(short_id.to_string()),
                Cell::from(format!("{}", session.messages.len())),
            ])
            .style(style)
        })
        .collect();

    let title = match app.current_tab {
        Tab::Sessions => format!(" Sessions ({}) ", filtered.len()),
        Tab::Trash => format!(" Trash ({}) ", filtered.len()),
    };

    let widths = [
        Constraint::Min(12),
        Constraint::Length(10),
        Constraint::Length(5),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray));

    // Create TableState and select current index for automatic scrolling
    let mut state = ratatui::widgets::TableState::default();
    state.select(Some(app.selected_session_idx));

    f.render_stateful_widget(table, area, &mut state);
}

fn draw_preview(f: &mut Frame, area: Rect, app: &App) {
    if let Some(session) = app.get_selected_session() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Project: ", Style::default().fg(Color::Yellow)),
                Span::raw(&session.project_name),
            ]),
            Line::from(vec![
                Span::styled("Session: ", Style::default().fg(Color::Yellow)),
                Span::raw(&session.id),
            ]),
            Line::from(vec![
                Span::styled("Updated: ", Style::default().fg(Color::Yellow)),
                Span::raw(&session.updated_at),
            ]),
            Line::from(vec![
                Span::styled("Size: ", Style::default().fg(Color::Yellow)),
                Span::raw(format_size(session.size)),
            ]),
            Line::from(vec![
                Span::styled("Messages: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", session.messages.len())),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "─── Conversation ───",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
        ];

        for msg in &session.messages {
            let (prefix, style) = if msg.role == "user" {
                (
                    "▶ You: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                (
                    "◀ Agent: ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            };

            lines.push(Line::from(Span::styled(prefix, style)));

            let truncated = if msg.content.len() > 500 {
                let mut end = 500;
                while !msg.content.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &msg.content[..end])
            } else {
                msg.content.clone()
            };

            for text_line in truncated.lines() {
                lines.push(Line::from(format!("  {}", text_line)));
            }
            lines.push(Line::from(""));
        }

        let preview = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Preview ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().bg(Color::Black))
            .wrap(Wrap { trim: false })
            .scroll((app.preview_scroll, 0));

        f.render_widget(preview, area);
    } else {
        let empty = Paragraph::new("No session selected. Use ↑/↓ to navigate.")
            .block(
                Block::default()
                    .title(" Preview ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .style(Style::default().bg(Color::Black));
        f.render_widget(empty, area);
    }
}

fn draw_search_modal(f: &mut Frame, app: &App) {
    let size = f.area();
    let area = Rect {
        x: size.width / 4,
        y: size.height / 2 - 1,
        width: size.width / 2,
        height: 3,
    };

    f.render_widget(Clear, area);

    let search = Paragraph::new(format!("Search: {}_", app.search_query))
        .block(
            Block::default()
                .title(" Search (Esc to close) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Black).fg(Color::White));

    f.render_widget(search, area);
}

fn draw_commands(f: &mut Frame, area: Rect, app: &App) {
    let commands_text = if let Some(ref msg) = app.status_message {
        Line::from(vec![Span::styled(msg, Style::default().fg(Color::Green))])
    } else {
        let cmds = match app.current_tab {
            Tab::Sessions => {
                vec![
                    Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
                    Span::raw(" resume  "),
                    Span::styled("[d]", Style::default().fg(Color::Red)),
                    Span::raw("elete  "),
                    Span::styled("[e]", Style::default().fg(Color::Yellow)),
                    Span::raw("xport  "),
                    Span::styled("[PgUp/PgDn]", Style::default().fg(Color::Cyan)),
                    Span::raw(" scroll  "),
                    Span::styled("[Ctrl+F]", Style::default().fg(Color::Cyan)),
                    Span::raw(" search  "),
                    Span::styled("[q]", Style::default().fg(Color::DarkGray)),
                    Span::raw("uit"),
                ]
            }
            Tab::Trash => {
                vec![
                    Span::styled("[r]", Style::default().fg(Color::Green)),
                    Span::raw("estore  "),
                    Span::styled("[d]", Style::default().fg(Color::Red)),
                    Span::raw("elete  "),
                    Span::styled("[Shift+E]", Style::default().fg(Color::Red)),
                    Span::raw(" empty trash  "),
                    Span::styled("[q]", Style::default().fg(Color::DarkGray)),
                    Span::raw("uit"),
                ]
            }
        };
        Line::from(cmds)
    };

    let bar = Paragraph::new(commands_text).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(bar, area);
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

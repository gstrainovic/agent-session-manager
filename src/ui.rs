use crate::app::{App, FocusPanel, Tab};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn draw_loading(f: &mut Frame, loaded: usize, total: usize) {
    let area = f.area();
    let percent = if total > 0 { (loaded * 100) / total } else { 0 };

    let gauge = ratatui::widgets::Gauge::default()
        .block(
            Block::default()
                .title(" Loading Sessions ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
        .percent(percent as u16)
        .label(format!("{}/{}", loaded, total));

    let centered = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(3),
            Constraint::Percentage(45),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(centered[1]);

    f.render_widget(gauge, horizontal[1]);
}

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

    if app.show_help {
        draw_help_modal(f, app);
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
        Span::styled("  [Tab] to switch  ", Style::default().fg(Color::DarkGray)),
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

    let sort_arrow = match app.sort_direction {
        crate::app::SortDirection::Ascending => "▲",
        crate::app::SortDirection::Descending => "▼",
    };

    let project_header = if app.sort_field == crate::app::SortField::Project {
        format!("Project {}", sort_arrow)
    } else {
        "Project".to_string()
    };

    let date_header = if app.sort_field == crate::app::SortField::Date {
        format!("Date {}", sort_arrow)
    } else {
        "Date".to_string()
    };

    let msgs_header = if app.sort_field == crate::app::SortField::Messages {
        format!("Msgs {}", sort_arrow)
    } else {
        "Msgs".to_string()
    };

    let header = Row::new(vec![
        Cell::from(project_header).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from(date_header).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from(msgs_header).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])
    .bottom_margin(0);

    let rows: Vec<Row> = filtered
        .iter()
        .map(|session| {
            // Format date as yyyy-mm-dd hh:mm
            let formatted_date = format_datetime(&session.updated_at);

            Row::new(vec![
                Cell::from(session.project_name.as_str()),
                Cell::from(formatted_date),
                Cell::from(format!("{}", session.messages.len())),
            ])
            .style(Style::default().fg(Color::White))
        })
        .collect();

    let title = match app.current_tab {
        Tab::Sessions => format!(" Sessions ({}) ", filtered.len()),
        Tab::Trash => format!(" Trash ({}) ", filtered.len()),
    };

    let widths = [
        Constraint::Min(12),
        Constraint::Length(16),
        Constraint::Length(7),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if app.focus == FocusPanel::List {
                    Color::Green
                } else {
                    Color::DarkGray
                })),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    // Create TableState and select current index for automatic scrolling
    let mut state = ratatui::widgets::TableState::default();
    state.select(Some(app.selected_session_idx));

    f.render_stateful_widget(table, area, &mut state);
}

fn format_datetime(iso_string: &str) -> String {
    if iso_string.len() >= 16 {
        let date_part = &iso_string[0..10];
        let time_part = &iso_string[11..16];
        format!("{} {}", date_part, time_part)
    } else {
        iso_string.to_string()
    }
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
            Line::from(vec![
                Span::styled("Entries: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}", session.total_entries)),
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

            let sanitized = sanitize_for_display(&msg.content);
            let truncated = if sanitized.len() > 500 {
                let mut end = 500;
                while !sanitized.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &sanitized[..end])
            } else {
                sanitized
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
                    .border_style(Style::default().fg(if app.focus == FocusPanel::Preview {
                        Color::Yellow
                    } else {
                        Color::DarkGray
                    })),
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
                    Span::styled("[s]", Style::default().fg(Color::Magenta)),
                    Span::raw(" sort  "),
                    Span::styled("[S]", Style::default().fg(Color::Magenta)),
                    Span::raw(" dir  "),
                    Span::styled("[←/→]", Style::default().fg(Color::Cyan)),
                    Span::raw(" focus  "),
                    Span::styled("[Ctrl+F]", Style::default().fg(Color::Cyan)),
                    Span::raw(" search  "),
                    Span::styled("[h]", Style::default().fg(Color::DarkGray)),
                    Span::raw("elp  "),
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
                    Span::styled("[t]", Style::default().fg(Color::Red)),
                    Span::raw(" empty  "),
                    Span::styled("[s]", Style::default().fg(Color::Magenta)),
                    Span::raw(" sort  "),
                    Span::styled("[S]", Style::default().fg(Color::Magenta)),
                    Span::raw(" dir  "),
                    Span::styled("[←/→]", Style::default().fg(Color::Cyan)),
                    Span::raw(" focus  "),
                    Span::styled("[h]", Style::default().fg(Color::DarkGray)),
                    Span::raw("elp  "),
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

/// Replaces Unicode characters with known terminal width mismatches.
/// Characters in Miscellaneous Symbols (U+2600-U+26FF), Dingbats (U+2700-U+27BF),
/// and emoji ranges have ambiguous widths that cause ratatui rendering artifacts.
fn sanitize_for_display(text: &str) -> String {
    text.chars()
        .map(|c| match c as u32 {
            0x2600..=0x26FF => ' ',   // Miscellaneous Symbols (⛁, ⛀, ⛶, etc.)
            0x2700..=0x27BF => ' ',   // Dingbats
            0x1F300..=0x1F9FF => ' ', // Miscellaneous Symbols and Pictographs, Emoticons, etc.
            0x1FA00..=0x1FAFF => ' ', // Supplemental Symbols
            _ => c,
        })
        .collect()
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

fn draw_help_modal(f: &mut Frame, app: &App) {
    let area = f.area();
    let width = (area.width as f32 * 0.8).min(100.0) as u16;
    let height = (area.height as f32 * 0.8).min(40.0) as u16;

    let popup_area = Rect {
        x: (area.width - width) / 2,
        y: (area.height - height) / 2,
        width,
        height,
    };

    f.render_widget(Clear, popup_area);

    let help_text = std::fs::read_to_string("README.md")
        .or_else(|_| std::fs::read_to_string("/home/g/workspace/agent-session-manager/README.md"))
        .unwrap_or_else(|_| "README.md not found".to_string());
    let lines: Vec<String> = help_text.lines().map(|s| s.to_string()).collect();

    let visible_lines: Vec<Line> = lines
        .iter()
        .skip(app.help_scroll as usize)
        .take(height as usize - 2)
        .map(|line| {
            let styled_line = if line.starts_with("# ") {
                Line::from(vec![Span::styled(
                    line.clone(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )])
            } else if line.starts_with("## ") {
                Line::from(vec![Span::styled(
                    line.clone(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )])
            } else if line.starts_with("### ") {
                Line::from(vec![Span::styled(
                    line.clone(),
                    Style::default().fg(Color::Green),
                )])
            } else if line.starts_with("| ") || line.starts_with("|-") {
                Line::from(vec![Span::styled(
                    line.clone(),
                    Style::default().fg(Color::White),
                )])
            } else if line.starts_with("```") {
                Line::from(vec![Span::styled(
                    line.clone(),
                    Style::default().fg(Color::DarkGray),
                )])
            } else {
                Line::from(vec![Span::styled(
                    sanitize_for_display(line),
                    Style::default().fg(Color::White),
                )])
            };
            styled_line
        })
        .collect();

    let help_widget = Paragraph::new(visible_lines)
        .block(
            Block::default()
                .title(" Help (h to close, ↑/↓ to scroll) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(help_widget, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::models::{Message, Session};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn make_session(id: &str, project: &str, messages: Vec<Message>) -> Session {
        Session {
            id: id.to_string(),
            project_path: format!("/home/g/{}", project),
            project_name: project.to_string(),
            created_at: "2026-01-15T10:00:00+01:00".to_string(),
            updated_at: "2026-01-15T12:00:00+01:00".to_string(),
            size: 1024,
            total_entries: messages.len() + 3,
            messages,
            original_content: None,
        }
    }

    fn make_msg(role: &str, content: &str) -> Message {
        Message {
            role: role.to_string(),
            content: content.to_string(),
        }
    }

    fn render_to_string(app: &App, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &app)).unwrap();
        terminal.backend().to_string()
    }

    #[test]
    fn test_renders_session_list_header() {
        let app = App::with_sessions(vec![make_session("abc12345-6789", "my-project", vec![])]);
        let output = render_to_string(&app, 100, 20);
        assert!(output.contains("Sessions"), "Should show Sessions tab");
        assert!(output.contains("my-project"), "Should show project name");
    }

    #[test]
    fn test_renders_message_count() {
        let app = App::with_sessions(vec![make_session(
            "abc12345-6789",
            "test-proj",
            vec![make_msg("user", "Hello"), make_msg("assistant", "Hi there")],
        )]);
        let output = render_to_string(&app, 100, 20);
        assert!(output.contains("2"), "Should show message count of 2");
    }

    #[test]
    fn test_renders_preview_for_selected_session() {
        let app = App::with_sessions(vec![make_session(
            "abc12345-6789",
            "my-project",
            vec![
                make_msg("user", "How do I test TUIs?"),
                make_msg("assistant", "Use TestBackend from ratatui"),
            ],
        )]);
        let output = render_to_string(&app, 100, 20);
        assert!(output.contains("Preview"), "Should show Preview panel");
        assert!(
            output.contains("How do I test TUIs"),
            "Should show user message in preview"
        );
    }

    #[test]
    fn test_renders_empty_state() {
        let app = App::with_sessions(vec![]);
        let output = render_to_string(&app, 100, 20);
        assert!(output.contains("Sessions (0)"), "Should show 0 sessions");
        assert!(
            output.contains("No session selected"),
            "Should show empty state message"
        );
    }

    #[test]
    fn test_selection_moves_preview() {
        let mut app = App::with_sessions(vec![
            make_session(
                "aaa11111-0000",
                "first-project",
                vec![make_msg("user", "First message")],
            ),
            make_session(
                "bbb22222-0000",
                "second-project",
                vec![make_msg("user", "Second message")],
            ),
        ]);

        // Initially first session selected
        let output = render_to_string(&app, 100, 20);
        assert!(
            output.contains("First message"),
            "Should show first session preview"
        );

        // Move selection down
        app.select_next();
        let output = render_to_string(&app, 100, 20);
        assert!(
            output.contains("Second message"),
            "Should show second session preview"
        );
    }

    #[test]
    fn test_search_modal_renders() {
        let mut app = App::with_sessions(vec![make_session("abc12345-6789", "my-project", vec![])]);
        app.show_search = true;
        app.search_query = "test".to_string();

        let output = render_to_string(&app, 100, 20);
        assert!(output.contains("Search"), "Should show search modal");
        assert!(output.contains("test"), "Should show search query");
    }

    #[test]
    fn test_truncated_session_id_in_list() {
        let app = App::with_sessions(vec![make_session(
            "abcdef12-3456-7890-abcd-ef1234567890",
            "proj",
            vec![],
        )]);
        let output = render_to_string(&app, 100, 20);
        assert!(
            output.contains("abcdef12"),
            "List should show truncated ID (first 8 chars)"
        );
        // Full ID is shown in Preview panel - that's correct
        assert!(
            output.contains("abcdef12-3456-7890"),
            "Preview should show full ID"
        );
    }

    #[test]
    fn test_commands_bar_shows_keybindings() {
        let app = App::with_sessions(vec![make_session("abc12345-6789", "my-project", vec![])]);
        let output = render_to_string(&app, 100, 20);
        assert!(output.contains("resume"), "Should show resume command");
        assert!(output.contains("elete"), "Should show delete command");
        assert!(output.contains("search"), "Should show search command");
    }

    #[test]
    fn test_sanitize_replaces_problematic_unicode() {
        // These chars (Miscellaneous Symbols) cause width mismatches
        let input = "⛁ Active files ⛀ board ⛶ custom";
        let result = sanitize_for_display(input);
        assert!(!result.contains('⛁'), "Should replace ⛁");
        assert!(!result.contains('⛀'), "Should replace ⛀");
        assert!(!result.contains('⛶'), "Should replace ⛶");
        assert!(result.contains("Active files"), "Should keep regular text");
    }

    #[test]
    fn test_sanitize_keeps_normal_text() {
        let input = "Hello, Welt! Ärger mit Ümlauten.";
        let result = sanitize_for_display(input);
        assert_eq!(
            result, input,
            "Normal text including umlauts should be unchanged"
        );
    }

    #[test]
    fn test_sanitize_keeps_box_drawing() {
        let input = "─── Conversation ───";
        let result = sanitize_for_display(input);
        assert_eq!(result, input, "Box drawing chars should be unchanged");
    }

    #[test]
    fn test_preview_with_problematic_unicode_renders_clean() {
        let app = App::with_sessions(vec![make_session(
            "abc12345-6789",
            "project",
            vec![make_msg("user", "⛁ Active 30+ ⛁ files ⛶ custom stack")],
        )]);
        let output = render_to_string(&app, 100, 20);
        assert!(!output.contains('⛁'), "Preview should not contain ⛁");
        assert!(output.contains("Active 30+"), "Should keep normal text");
    }

    #[test]
    fn test_preview_shows_entries_and_messages() {
        let app = App::with_sessions(vec![make_session(
            "abc12345-6789",
            "my-project",
            vec![make_msg("user", "Hello")],
        )]);
        let output = render_to_string(&app, 100, 20);
        // Preview should show both messages count and total entries
        assert!(output.contains("Messages:"), "Should show Messages label");
        assert!(output.contains("Entries:"), "Should show Entries label");
    }

    #[test]
    fn test_snapshot_initial_render() {
        let app = App::with_sessions(vec![
            make_session(
                "abc12345-6789",
                "my-project",
                vec![
                    make_msg("user", "Hello, how are you?"),
                    make_msg("assistant", "I am doing well, thank you!"),
                ],
            ),
            make_session(
                "def98765-4321",
                "other-project",
                vec![make_msg("user", "What is Rust?")],
            ),
        ]);
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &app)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }
}

use crate::app::{App, ClickAction, FocusPanel, Tab};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, Wrap},
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

pub fn draw(f: &mut Frame, app: &mut App) {
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

    // Click-Regionen jedes Frame neu aufbauen (Single Source of Truth)
    let area = f.area();
    app.terminal_size = (area.width, area.height);
    app.click_regions.clear();

    draw_tabs(f, chunks[0], app);

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Min(0)])
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

    if app.show_settings {
        draw_settings_modal(f, app);
    }

    if app.show_rename {
        draw_rename_modal(f, app);
    }
}

fn draw_tabs(f: &mut Frame, area: Rect, app: &mut App) {
    let session_count = app.sessions.len();
    let trash_count = app.trash.len();

    let tab_indicator = |tab: Tab| {
        if app.current_tab == tab {
            ("●", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        } else {
            ("○", Style::default().fg(Color::DarkGray))
        }
    };
    let (sessions_marker, sessions_style) = tab_indicator(Tab::Sessions);
    let (trash_marker, trash_style) = tab_indicator(Tab::Trash);

    let sessions_text = format!("  {} 1 Sessions ({})  ", sessions_marker, session_count);
    let trash_text = format!("  {} 2 Trash ({})  ", trash_marker, trash_count);
    let help_text = "│  h help  ";

    let tabs = vec![
        Span::styled(&sessions_text, sessions_style),
        Span::styled(&trash_text, trash_style),
        Span::styled(help_text, Style::default().fg(Color::DarkGray)),
    ];

    let tabs_line = Line::from(tabs);
    let tabs_widget = Paragraph::new(tabs_line).block(
        Block::default()
            .title(" Agent Session Manager ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(tabs_widget, area);

    // Click-Regionen direkt beim Rendering registrieren (Single Source of Truth)
    // Block hat 1px Border links → Content startet bei area.x + 1
    let content_x = area.x + 1;
    let sw = sessions_text.chars().count() as u16;
    let tw = trash_text.chars().count() as u16;
    let hw = help_text.chars().count() as u16;
    app.click_regions.push((
        Rect { x: content_x, y: area.y, width: sw, height: area.height },
        ClickAction::SwitchTab(Tab::Sessions),
    ));
    app.click_regions.push((
        Rect { x: content_x + sw, y: area.y, width: tw, height: area.height },
        ClickAction::SwitchTab(Tab::Trash),
    ));
    app.click_regions.push((
        Rect { x: content_x + sw + tw, y: area.y, width: hw, height: area.height },
        ClickAction::ToggleHelp,
    ));
}

fn draw_list(f: &mut Frame, area: Rect, app: &mut App) {
    // Clone to release borrow on app before we need &mut app.list_table_state
    let filtered: Vec<_> = app.filtered_sessions().into_iter().cloned().collect();

    let sort_arrow = match app.sort_direction {
        crate::app::SortDirection::Ascending => "▲",
        crate::app::SortDirection::Descending => "▼",
    };

    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let make_header = |label: &str, field: crate::app::SortField| -> Cell {
        let text = if app.sort_field == field {
            format!("{} {}", label, sort_arrow)
        } else {
            label.to_string()
        };
        Cell::from(text).style(header_style)
    };

    let header = Row::new(vec![
        make_header("Project", crate::app::SortField::Project),
        make_header("Name", crate::app::SortField::Name),
        make_header("Date", crate::app::SortField::Date),
        make_header("Msgs", crate::app::SortField::Messages),
    ])
    .bottom_margin(0);

    let rows: Vec<Row> = filtered
        .iter()
        .map(|session| {
            let formatted_date = format_datetime(&session.updated_at);
            let name = session.slug.as_deref().unwrap_or("");

            Row::new(vec![
                Cell::from(session.project_name.as_str()),
                Cell::from(name),
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
        Constraint::Min(10),
        Constraint::Min(8),
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

    let selected = app.selected_session_idx;
    let total = filtered.len();
    app.list_table_state.select(Some(selected));
    f.render_stateful_widget(table, area, &mut app.list_table_state);

    // Scrollbar rechts neben der Liste
    let mut scrollbar_state = ScrollbarState::new(total).position(selected);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .style(Style::default().fg(if app.focus == FocusPanel::List {
                Color::Green
            } else {
                Color::DarkGray
            })),
        area,
        &mut scrollbar_state,
    );
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
                Span::styled("Label:   ", Style::default().fg(Color::Cyan)),
                Span::raw(session.slug.as_deref().unwrap_or("—")),
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

        let total_lines = lines.len();

        let border_color = if app.focus == FocusPanel::Preview {
            Color::Yellow
        } else {
            Color::DarkGray
        };

        let preview = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Preview ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .style(Style::default().bg(Color::Black))
            .wrap(Wrap { trim: false })
            .scroll((app.preview_scroll, 0));

        f.render_widget(preview, area);

        // Scrollbar über dem Preview rendern
        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .position(app.preview_scroll as usize);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .style(Style::default().fg(border_color)),
            area,
            &mut scrollbar_state,
        );
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

fn draw_search_modal(f: &mut Frame, app: &mut App) {
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

    // Search-Modal: Keine Click-Regionen → click-outside in main.rs schließt das Modal
    app.click_regions.clear();
}

fn draw_commands(f: &mut Frame, area: Rect, app: &mut App) {
    let sep = Span::styled("  │  ", Style::default().fg(Color::DarkGray));

    // Confirmation mit [y]/[n] Buttons
    if app.is_confirmation_pending() {
        if let Some(ref msg) = app.status_message {
            let question = if let Some(pos) = msg.find(" Press ") {
                &msg[..pos]
            } else {
                msg.as_str()
            };
            let question_with_pad = format!("{}  ", question);
            let yes_text = " [y] yes ";
            let gap = "  ";
            let no_text = " [n] no  ";
            let bar = Paragraph::new(Line::from(vec![
                Span::styled(&question_with_pad, Style::default().fg(Color::Yellow)),
                Span::styled(yes_text, Style::default().fg(Color::Black).bg(Color::Green)),
                Span::raw(gap),
                Span::styled(no_text, Style::default().fg(Color::Black).bg(Color::Red)),
            ]))
            .block(Block::default().borders(Borders::TOP).border_style(
                Style::default().fg(Color::DarkGray),
            ));
            f.render_widget(bar, area);

            // Click-Regionen: Content startet bei area.x + 0 (TOP border nimmt nur y)
            // Block mit Borders::TOP hat content bei area.y + 1
            let content_y = area.y + 1;
            let q_width = question_with_pad.chars().count() as u16;
            let yes_width = yes_text.chars().count() as u16;
            let gap_width = gap.chars().count() as u16;
            let no_width = no_text.chars().count() as u16;
            // Modale Regionen: erst alle normalen Regionen löschen
            app.click_regions.clear();
            app.click_regions.push((
                Rect { x: area.x + q_width, y: content_y, width: yes_width, height: 1 },
                ClickAction::ConfirmYes,
            ));
            app.click_regions.push((
                Rect { x: area.x + q_width + yes_width + gap_width, y: content_y, width: no_width, height: 1 },
                ClickAction::ConfirmNo,
            ));
            return;
        }
    }

    // Status-Nachricht: volle Breite
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

    let k = |s: &'static str| Span::styled(s, Style::default().fg(Color::Cyan));
    let d = |s: &'static str| Span::styled(s, Style::default().fg(Color::Red));
    let t = |s: &'static str| Span::raw(s);

    let nav = vec![
        k("↑↓"), t(" nav  "), k("←→"), t(" focus"),
    ];

    let action_defs: Vec<(&str, &str, ClickAction)> = match app.current_tab {
        Tab::Sessions => vec![
            ("Enter", " run  ", ClickAction::ResumeSession),
            ("r", " rename  ", ClickAction::RenameSession),
            ("d", " delete  ", ClickAction::DeleteSession),
            ("e", " export  ", ClickAction::ExportSession),
            ("c", " clear  ", ClickAction::CleanZeroMessages),
            ("f", " find  ", ClickAction::ToggleSearch),
            ("s", " sort  ", ClickAction::ToggleSort),
            ("p", " preferences  ", ClickAction::OpenSettings),
            ("h", " help  ", ClickAction::ToggleHelp),
            ("q", " quit", ClickAction::Quit),
        ],
        Tab::Trash => vec![
            ("u", " undo  ", ClickAction::RestoreFromTrash),
            ("r", " rename  ", ClickAction::RenameSession),
            ("e", " empty trash ", ClickAction::EmptyTrash),
            ("f", " find  ", ClickAction::ToggleSearch),
            ("s", " sort  ", ClickAction::ToggleSort),
            ("h", " help  ", ClickAction::ToggleHelp),
            ("q", " quit", ClickAction::Quit),
        ],
    };

    // Build spans for rendering and click regions simultaneously
    let mut actions: Vec<Span> = Vec::new();
    let content_y = area.y + 1; // Borders::TOP nimmt erste Zeile
    // nav spans: "↑↓" + " nav  " + "←→" + " focus" + sep "  │  "
    let nav_width: u16 = nav.iter().map(|s| s.content.chars().count() as u16).sum();
    let sep_width = 5u16; // "  │  "
    let mut x = area.x + nav_width + sep_width;

    for (key, desc, click_action) in &action_defs {
        let key_style = if *key == "d" || *key == "c" { d(key) } else { k(key) };
        actions.push(key_style);
        actions.push(t(desc));
        let kw = key.chars().count() as u16;
        let dw = desc.chars().count() as u16;
        app.click_regions.push((
            Rect { x, y: content_y, width: kw + dw, height: area.height.saturating_sub(1) },
            click_action.clone(),
        ));
        x += kw + dw;
    }

    let mut spans = nav;
    spans.push(sep);
    spans.extend(actions);

    let bar = Paragraph::new(Line::from(spans)).block(
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

fn draw_help_modal(f: &mut Frame, app: &mut App) {
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

    let help_text = std::fs::read_to_string("help.md")
        .or_else(|_| std::fs::read_to_string("/home/g/workspace/agent-session-manager/help.md"))
        .unwrap_or_else(|_| "README.md not found".to_string());
    let lines: Vec<String> = help_text.lines().map(|s| s.to_string()).collect();

    let visible_lines: Vec<Line> = lines
        .iter()
        .skip(app.help_scroll as usize)
        .take(height as usize - 2)
        .map(|line| parse_markdown_line(line))
        .collect();

    let help_widget = Paragraph::new(visible_lines)
        .block(
            Block::default()
                .title(" Help (Esc/h to close, ↑/↓ to scroll) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(help_widget, popup_area);

    // Help-Modal: Keine Click-Regionen → click-outside in main.rs schließt das Modal
    app.click_regions.clear();
}

fn parse_markdown_line(line: &str) -> Line<'_> {
    let line = sanitize_for_display(line);

    if line.starts_with("# ") {
        Line::from(vec![Span::styled(
            line[2..].to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )])
    } else if line.starts_with("## ") {
        Line::from(vec![Span::styled(
            line[3..].to_string(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )])
    } else if line.starts_with("### ") {
        Line::from(vec![Span::styled(
            line[4..].to_string(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )])
    } else if line.starts_with("```") {
        Line::from(vec![Span::styled(
            line.to_string(),
            Style::default().fg(Color::DarkGray),
        )])
    } else if line.starts_with("| ") || line.starts_with("|-") {
        Line::from(vec![Span::styled(
            line.to_string(),
            Style::default().fg(Color::White),
        )])
    } else if line.starts_with("- ") || line.starts_with("* ") {
        let content = line[2..].to_string();
        let spans = parse_inline_formatting(content, Style::default().fg(Color::White));
        let mut result = vec![Span::styled(
            "• ".to_string(),
            Style::default().fg(Color::Cyan),
        )];
        result.extend(spans);
        Line::from(result)
    } else if line.starts_with("  ")
        && (line.trim().starts_with("- ") || line.trim().starts_with("* "))
    {
        let content = line.trim()[2..].to_string();
        let spans = parse_inline_formatting(content, Style::default().fg(Color::White));
        let mut result = vec![Span::styled(
            "  ◦ ".to_string(),
            Style::default().fg(Color::DarkGray),
        )];
        result.extend(spans);
        Line::from(result)
    } else {
        let spans = parse_inline_formatting(line.clone(), Style::default().fg(Color::White));
        Line::from(spans)
    }
}

fn parse_inline_formatting(text: String, base_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut chars = text.chars().peekable();
    let mut current_text = String::new();

    while let Some(ch) = chars.next() {
        if ch == '`' {
            if !current_text.is_empty() {
                spans.push(Span::styled(current_text.clone(), base_style));
                current_text.clear();
            }
            let mut code_content = String::new();
            while let Some(next_ch) = chars.next() {
                if next_ch == '`' {
                    break;
                }
                code_content.push(next_ch);
            }
            spans.push(Span::styled(
                code_content,
                Style::default().fg(Color::Yellow).bg(Color::DarkGray),
            ));
        } else if ch == '*' || ch == '_' {
            if chars.peek() == Some(&ch) {
                chars.next();
                if !current_text.is_empty() {
                    spans.push(Span::styled(current_text.clone(), base_style));
                    current_text.clear();
                }
                let mut bold_content = String::new();
                let mut found_end = false;
                while let Some(next_ch) = chars.next() {
                    if next_ch == ch && chars.peek() == Some(&ch) {
                        chars.next();
                        found_end = true;
                        break;
                    }
                    bold_content.push(next_ch);
                }
                spans.push(Span::styled(
                    bold_content.clone(),
                    base_style.add_modifier(Modifier::BOLD),
                ));
                if !found_end {
                    current_text.push(ch);
                    current_text.push(ch);
                    current_text.push_str(&bold_content);
                }
            } else {
                current_text.push(ch);
            }
        } else if ch == '[' {
            let mut link_text = String::new();
            let mut found_end = false;
            while let Some(next_ch) = chars.next() {
                if next_ch == ']' {
                    found_end = true;
                    break;
                }
                link_text.push(next_ch);
            }
            if found_end && chars.peek() == Some(&'(') {
                chars.next();
                let mut url = String::new();
                while let Some(next_ch) = chars.next() {
                    if next_ch == ')' {
                        break;
                    }
                    url.push(next_ch);
                }
                spans.push(Span::styled(
                    link_text,
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::UNDERLINED),
                ));
            } else {
                current_text.push(ch);
                current_text.push_str(&link_text);
                if found_end {
                    current_text.push(']');
                }
            }
        } else {
            current_text.push(ch);
        }
    }

    if !current_text.is_empty() {
        spans.push(Span::styled(current_text, base_style));
    }

    spans
}

fn draw_settings_modal(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let width = (area.width as f32 * 0.6) as u16;
    let height = 7u16;

    let popup_area = Rect {
        x: (area.width.saturating_sub(width)) / 2,
        y: (area.height.saturating_sub(height)) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    };

    let block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(popup_area);

    let save_text = "  [Enter]";
    let save_desc = " save  ";
    let cancel_text = "[Esc]";
    let cancel_desc = " cancel";

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Export Path: ", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  {}_", app.settings_input),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(save_text, Style::default().fg(Color::Green)),
            Span::raw(save_desc),
            Span::styled(cancel_text, Style::default().fg(Color::Red)),
            Span::raw(cancel_desc),
        ]),
    ];

    let paragraph = Paragraph::new(text);

    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);
    f.render_widget(paragraph, inner);

    // Click-Regionen: Settings-Modal überschreibt alle anderen
    app.click_regions.clear();
    let btn_y = inner.y + 4; // 5. Zeile im Modal (index 4)
    let save_width = (save_text.chars().count() + save_desc.chars().count()) as u16;
    let cancel_width = (cancel_text.chars().count() + cancel_desc.chars().count()) as u16;
    app.click_regions.push((
        Rect { x: inner.x, y: btn_y, width: save_width, height: 1 },
        ClickAction::SaveSettings,
    ));
    app.click_regions.push((
        Rect { x: inner.x + save_width, y: btn_y, width: cancel_width, height: 1 },
        ClickAction::CancelSettings,
    ));
}

fn draw_rename_modal(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let width = (area.width as f32 * 0.6) as u16;
    let height = 7u16;

    let popup_area = Rect {
        x: (area.width.saturating_sub(width)) / 2,
        y: (area.height.saturating_sub(height)) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    };

    let block = Block::default()
        .title(" Rename Session ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  New name: ", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  {}_", app.rename_input),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [Enter]", Style::default().fg(Color::Green)),
            Span::raw(" save  "),
            Span::styled("[Esc]", Style::default().fg(Color::Red)),
            Span::raw(" cancel"),
        ]),
    ];

    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);
    f.render_widget(Paragraph::new(text), inner);
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
            jsonl_path: std::path::PathBuf::new(),
            slug: None,
        }
    }

    fn make_msg(role: &str, content: &str) -> Message {
        Message {
            role: role.to_string(),
            content: content.to_string(),
        }
    }

    fn render_to_string(app: &mut App, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        terminal.backend().to_string()
    }

    #[test]
    fn test_renders_session_list_header() {
        let mut app = App::with_sessions(vec![make_session("abc12345-6789", "my-project", vec![])]);
        let output = render_to_string(&mut app, 100, 20);
        assert!(output.contains("Sessions"), "Should show Sessions tab");
        assert!(output.contains("my-project"), "Should show project name");
    }

    #[test]
    fn test_renders_message_count() {
        let mut app = App::with_sessions(vec![make_session(
            "abc12345-6789",
            "test-proj",
            vec![make_msg("user", "Hello"), make_msg("assistant", "Hi there")],
        )]);
        let output = render_to_string(&mut app, 100, 20);
        assert!(output.contains("2"), "Should show message count of 2");
    }

    #[test]
    fn test_renders_preview_for_selected_session() {
        let mut app = App::with_sessions(vec![make_session(
            "abc12345-6789",
            "my-project",
            vec![
                make_msg("user", "How do I test TUIs?"),
                make_msg("assistant", "Use TestBackend from ratatui"),
            ],
        )]);
        let output = render_to_string(&mut app, 100, 20);
        assert!(output.contains("Preview"), "Should show Preview panel");
        assert!(
            output.contains("How do I test TUIs"),
            "Should show user message in preview"
        );
    }

    #[test]
    fn test_renders_empty_state() {
        let mut app = App::with_sessions(vec![]);
        let output = render_to_string(&mut app, 100, 20);
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
        let output = render_to_string(&mut app, 100, 20);
        assert!(
            output.contains("First message"),
            "Should show first session preview"
        );

        // Move selection down
        app.select_next();
        let output = render_to_string(&mut app, 100, 20);
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

        let output = render_to_string(&mut app, 100, 20);
        assert!(output.contains("Search"), "Should show search modal");
        assert!(output.contains("test"), "Should show search query");
    }

    #[test]
    fn test_truncated_session_id_in_list() {
        let mut app = App::with_sessions(vec![make_session(
            "abcdef12-3456-7890-abcd-ef1234567890",
            "proj",
            vec![],
        )]);
        let output = render_to_string(&mut app, 100, 20);
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
        let mut app = App::with_sessions(vec![make_session("abc12345-6789", "my-project", vec![])]);
        let output = render_to_string(&mut app, 100, 20);
        assert!(output.contains("run"), "Should show run command");
        assert!(output.contains("rename"), "Should show rename command");
        assert!(output.contains("elete"), "Should show delete command");
        assert!(output.contains("find"), "Should show find command");
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
        let mut app = App::with_sessions(vec![make_session(
            "abc12345-6789",
            "project",
            vec![make_msg("user", "⛁ Active 30+ ⛁ files ⛶ custom stack")],
        )]);
        let output = render_to_string(&mut app, 100, 20);
        assert!(!output.contains('⛁'), "Preview should not contain ⛁");
        assert!(output.contains("Active 30+"), "Should keep normal text");
    }

    #[test]
    fn test_preview_shows_entries_and_messages() {
        let mut app = App::with_sessions(vec![make_session(
            "abc12345-6789",
            "my-project",
            vec![make_msg("user", "Hello")],
        )]);
        let output = render_to_string(&mut app, 100, 20);
        // Preview should show both messages count and total entries
        assert!(output.contains("Messages:"), "Should show Messages label");
        assert!(output.contains("Entries:"), "Should show Entries label");
    }

    #[test]
    fn test_name_column_shows_custom_title() {
        let mut s = make_session("abc12345-6789", "my-project", vec![make_msg("user", "hi")]);
        s.slug = Some("my-label".to_string());
        let mut app = App::with_sessions(vec![s]);
        let output = render_to_string(&mut app, 120, 20);
        assert!(output.contains("my-label"), "Name column should show custom title");
        assert!(output.contains("Name"), "Header should have Name column");
    }

    #[test]
    fn test_preview_shows_custom_title_label() {
        let mut s = make_session("abc12345-6789", "my-project", vec![make_msg("user", "hi")]);
        s.slug = Some("renamed-session".to_string());
        let mut app = App::with_sessions(vec![s]);
        let output = render_to_string(&mut app, 120, 20);
        assert!(output.contains("renamed-session"), "Preview should show custom title as label");
    }

    #[test]
    fn test_snapshot_initial_render() {
        let mut app = App::with_sessions(vec![
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
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_snapshot_settings_modal() {
        let mut app = App::with_sessions(vec![make_session(
            "abc12345-6789",
            "my-project",
            vec![make_msg("user", "Hello")],
        )]);
        app.open_settings();
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_snapshot_help_modal() {
        let mut app = App::with_sessions(vec![make_session(
            "abc12345-6789",
            "my-project",
            vec![make_msg("user", "Hello")],
        )]);
        app.toggle_help();
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_snapshot_delete_confirm() {
        let mut app = App::with_sessions(vec![make_session(
            "abc12345-6789",
            "my-project",
            vec![make_msg("user", "Hello")],
        )]);
        app.request_delete_confirmation();
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }

    // --- format_size ---

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(2 * 1024 * 1024 + 512 * 1024), "2.5 MB");
    }

    // --- format_datetime ---

    #[test]
    fn test_format_datetime_iso() {
        assert_eq!(format_datetime("2026-01-15T10:30:00+01:00"), "2026-01-15 10:30");
    }

    #[test]
    fn test_format_datetime_short_string() {
        assert_eq!(format_datetime("short"), "short");
    }

    // --- parse_markdown_line ---

    #[test]
    fn test_parse_markdown_h1() {
        let line = parse_markdown_line("# Title");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "Title");
    }

    #[test]
    fn test_parse_markdown_bullet() {
        let line = parse_markdown_line("- Item text");
        assert_eq!(line.spans[0].content.as_ref(), "• ");
    }

    // --- parse_inline_formatting ---

    #[test]
    fn test_parse_inline_code() {
        let spans = parse_inline_formatting("use `cargo test`".to_string(), Style::default());
        assert!(spans.iter().any(|s| s.content.as_ref() == "cargo test"));
    }

    #[test]
    fn test_parse_inline_bold() {
        let spans =
            parse_inline_formatting("this is **bold** text".to_string(), Style::default());
        assert!(spans.iter().any(|s| s.content.as_ref() == "bold"));
    }

    #[test]
    fn test_parse_inline_link() {
        let spans = parse_inline_formatting(
            "see [docs](http://example.com)".to_string(),
            Style::default(),
        );
        assert!(spans.iter().any(|s| s.content.as_ref() == "docs"));
    }

    // --- draw_loading ---

    #[test]
    fn test_snapshot_loading_progress() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw_loading(f, 42, 100)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }

    // --- parse_markdown_line: H2, H3, code blocks, tables, nested lists ---

    #[test]
    fn test_parse_markdown_h2() {
        let line = parse_markdown_line("## Subtitle");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "Subtitle");
    }

    #[test]
    fn test_parse_markdown_h3() {
        let line = parse_markdown_line("### Section");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "Section");
    }

    #[test]
    fn test_parse_markdown_code_block() {
        let line = parse_markdown_line("```rust");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "```rust");
    }

    #[test]
    fn test_parse_markdown_table_line() {
        let line = parse_markdown_line("| Key | Action |");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "| Key | Action |");
    }

    #[test]
    fn test_parse_markdown_table_separator() {
        let line = parse_markdown_line("|---|---|");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content.as_ref(), "|---|---|");
    }

    #[test]
    fn test_parse_markdown_nested_bullet() {
        let line = parse_markdown_line("  - Nested item");
        assert_eq!(line.spans[0].content.as_ref(), "  ◦ ");
    }

    #[test]
    fn test_parse_markdown_asterisk_bullet() {
        let line = parse_markdown_line("* Asterisk item");
        assert_eq!(line.spans[0].content.as_ref(), "• ");
    }

    #[test]
    fn test_parse_markdown_nested_asterisk() {
        let line = parse_markdown_line("  * Nested asterisk");
        assert_eq!(line.spans[0].content.as_ref(), "  ◦ ");
    }

    #[test]
    fn test_parse_markdown_plain_text() {
        let line = parse_markdown_line("Just plain text");
        assert!(line.spans.iter().any(|s| s.content.as_ref() == "Just plain text"));
    }

    // --- parse_inline_formatting: edge cases ---

    #[test]
    fn test_parse_inline_unclosed_backtick() {
        let spans = parse_inline_formatting("use `unclosed code".to_string(), Style::default());
        // Should not panic; unclosed backtick produces a span with content up to end
        assert!(!spans.is_empty());
        assert!(spans.iter().any(|s| s.content.as_ref() == "unclosed code"));
    }

    #[test]
    fn test_parse_inline_unclosed_bold() {
        let spans =
            parse_inline_formatting("this is **unclosed bold".to_string(), Style::default());
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_parse_inline_unclosed_link() {
        let spans =
            parse_inline_formatting("see [broken link".to_string(), Style::default());
        assert!(!spans.is_empty());
        // Should include the bracket as plain text
        assert!(spans
            .iter()
            .any(|s| s.content.as_ref().contains("[") || s.content.as_ref().contains("broken")));
    }

    #[test]
    fn test_parse_inline_link_without_url() {
        let spans =
            parse_inline_formatting("see [text] no url".to_string(), Style::default());
        assert!(!spans.is_empty());
        // [text] without (url) should be treated as plain text
        assert!(spans.iter().any(|s| s.content.as_ref().contains("text")));
    }

    #[test]
    fn test_parse_inline_underscore_bold() {
        let spans =
            parse_inline_formatting("this is __bold__ text".to_string(), Style::default());
        assert!(spans.iter().any(|s| s.content.as_ref() == "bold"));
    }

    #[test]
    fn test_parse_inline_single_asterisk_not_bold() {
        let spans =
            parse_inline_formatting("a * b * c".to_string(), Style::default());
        // Single asterisks should be kept as plain text
        assert!(spans.iter().any(|s| s.content.as_ref().contains("*")));
    }

    #[test]
    fn test_parse_inline_empty_string() {
        let spans = parse_inline_formatting(String::new(), Style::default());
        assert!(spans.is_empty());
    }

    // --- draw_loading edge case ---

    #[test]
    fn test_draw_loading_zero_total() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw_loading(f, 0, 0)).unwrap();
        // Should not panic with total=0
        let output = terminal.backend().to_string();
        assert!(output.contains("0/0"));
    }

    // --- sanitize_for_display: additional ranges ---

    #[test]
    fn test_sanitize_replaces_emoji() {
        let input = "Hello \u{1F600} World"; // 😀
        let result = sanitize_for_display(input);
        assert!(!result.contains('\u{1F600}'));
        assert!(result.contains("Hello"));
        assert!(result.contains("World"));
    }

    #[test]
    fn test_sanitize_replaces_supplemental_symbols() {
        let input = "Test \u{1FA80} end"; // 🪀
        let result = sanitize_for_display(input);
        assert!(!result.contains('\u{1FA80}'));
    }

    // --- format_datetime edge cases ---

    #[test]
    fn test_format_datetime_empty_string() {
        assert_eq!(format_datetime(""), "");
    }

    #[test]
    fn test_format_datetime_exact_16_chars() {
        assert_eq!(format_datetime("2026-01-15T10:30"), "2026-01-15 10:30");
    }

    // --- Snapshot: Trash tab render ---

    #[test]
    fn test_snapshot_trash_tab() {
        let mut app = App::with_sessions(vec![]);
        app.trash = vec![make_session(
            "trash-session",
            "deleted-project",
            vec![make_msg("user", "old message")],
        )];
        app.current_tab = crate::app::Tab::Trash;
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }

    // --- Snapshot: Search modal with results ---

    #[test]
    fn test_snapshot_search_with_filter() {
        let mut app = App::with_sessions(vec![
            make_session(
                "abc12345",
                "alpha-project",
                vec![make_msg("user", "alpha content")],
            ),
            make_session(
                "def67890",
                "beta-project",
                vec![make_msg("user", "beta content")],
            ),
        ]);
        app.show_search = true;
        app.search_query = "alpha".to_string();
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }
}

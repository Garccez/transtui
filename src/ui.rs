use std::io;
use tui::{
    Frame,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap},
};

use crate::app::{App, AppState};

pub fn render(frame: &mut Frame<CrosstermBackend<io::Stdout>>, app: &App) {
    match app.state {
        AppState::FileSelection => render_file_selection(frame, app),
        AppState::Editing => {
            if let Some(editing) = &app.editing {
                render_editing(frame, editing, app)
            }
        }
        AppState::SaveConfirmation => {
            if let Some(confirmation) = &app.save_confirmation {
                render_save_confirmation(frame, confirmation, app)
            }
        }
        AppState::Exiting => (),
    }
}

pub fn render_file_selection(frame: &mut Frame<CrosstermBackend<io::Stdout>>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
        .split(frame.size());

    let items: Vec<ListItem> = app
        .file_selection
        .files
        .iter()
        .map(|f| {
            ListItem::new(
                f.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            )
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.locale.get("file_selection_title")),
        )
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black));

    frame.render_stateful_widget(list, chunks[0], &mut app.file_selection.list_state.clone());

    let help = Paragraph::new(vec![Spans::from(vec![
        Span::raw(app.locale.get("help_navigation")),
        Span::styled(
            app.locale.get("up_down_keys"),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(app.locale.get("select_help")),
        Span::styled(
            app.locale.get("language_key"),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(app.locale.get("language_help")),
        Span::styled(
            app.locale.get("enter_key"),
            Style::default().fg(Color::Green),
        ),
        Span::raw(app.locale.get("open_help")),
        Span::styled(app.locale.get("quit_key"), Style::default().fg(Color::Red)),
        Span::raw(app.locale.get("quit_help")),
    ])])
    .block(Block::default().borders(Borders::TOP))
    .wrap(Wrap { trim: true });

    frame.render_widget(help, chunks[1]);
}

pub fn render_editing(
    frame: &mut Frame<CrosstermBackend<io::Stdout>>,
    state: &crate::app::EditingState,
    app: &App,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(frame.size());

    let title = app.locale.get_with_params(
        "translation_title",
        &[
            ("translated", &state.translated_keys.to_string()),
            ("total", &state.total_keys.to_string()),
        ],
    );

    let rows: Vec<Row> = state
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let key_style = if entry.is_translated {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            let style = if state.table_state.selected() == Some(i) {
                Style::default().bg(Color::Blue)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(Span::styled(entry.key.clone(), key_style)),
                Cell::from(format_json_value(&entry.original)),
                Cell::from(format_json_value(&entry.translated)),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(rows)
        .header(Row::new(vec![
            app.locale.get("header_key"),
            app.locale.get("header_original"),
            app.locale.get("header_translated"),
        ]))
        .block(Block::default().borders(Borders::ALL).title(title))
        .widths(&[
            Constraint::Percentage(25),
            Constraint::Percentage(35),
            Constraint::Percentage(40),
        ]);

    frame.render_stateful_widget(table, chunks[0], &mut state.table_state.clone());

    let cursor_byte_pos = state
        .input
        .chars()
        .take(state.cursor_pos)
        .map(|c| c.len_utf8())
        .sum::<usize>();

    let (left, right) = state.input.split_at(cursor_byte_pos);
    let input_display = format!("{}█{}", left, right);

    let input = Paragraph::new(input_display).block(
        Block::default()
            .borders(Borders::ALL)
            .title(app.locale.get("edit_value_title")),
    );
    frame.render_widget(input, chunks[1]);

    let help_text = if state.editing.is_some() {
        vec![Spans::from(vec![
            Span::styled(
                app.locale.get("cursor_key"),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(app.locale.get("cursor_help")),
            Span::styled(
                app.locale.get("enter_key"),
                Style::default().fg(Color::Green),
            ),
            Span::raw(app.locale.get("confirm_help")),
            Span::styled(app.locale.get("esc_key"), Style::default().fg(Color::Red)),
            Span::raw(app.locale.get("cancel_help")),
        ])]
    } else if state.search_mode {
        vec![Spans::from(vec![
            Span::styled(
                app.locale.get("up_down_keys"),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(app.locale.get("search_navigate_help")),
            Span::styled(
                app.locale.get("enter_key"),
                Style::default().fg(Color::Green),
            ),
            Span::raw(app.locale.get("select_help")),
            Span::styled(app.locale.get("esc_key"), Style::default().fg(Color::Red)),
            Span::raw(app.locale.get("cancel_help")),
        ])]
    } else {
        vec![Spans::from(vec![
            Span::raw(app.locale.get("navigation_help")),
            Span::styled(
                app.locale.get("up_down_keys"),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(app.locale.get("select_help")),
            Span::styled(
                app.locale.get("language_key"),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(app.locale.get("language_help")),
            Span::styled(
                app.locale.get("enter_key"),
                Style::default().fg(Color::Green),
            ),
            Span::raw(app.locale.get("edit_help")),
            Span::styled("T", Style::default().fg(Color::Magenta)),
            Span::raw(app.locale.get("mark_translated_help")),
            Span::styled("B", Style::default().fg(Color::LightGreen)),
            Span::raw(app.locale.get("save_help")),
            Span::styled(app.locale.get("esc_key"), Style::default().fg(Color::Blue)),
            Span::raw(app.locale.get("save_return_help")),
            Span::styled("Q", Style::default().fg(Color::Red)),
            Span::raw(app.locale.get("save_quit_help")),
            Span::styled("S", Style::default().fg(Color::Cyan)),
            Span::raw(app.locale.get("search_help")),
        ])]
    };

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::TOP))
        .wrap(Wrap { trim: true });

    frame.render_widget(help, chunks[2]);

    if state.search_mode {
        let search_text = app.locale.get_with_params(
            "search_results",
            &[
                ("query", &state.search_query),
                ("count", &state.search_results.len().to_string()),
            ],
        );
        let search_bar = Paragraph::new(search_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.locale.get("search_title")),
        );
        frame.render_widget(search_bar, chunks[3]);
    } else if state.save_notification.is_some() {
        let notification = Paragraph::new(app.locale.get("save_success"))
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(notification, chunks[3]);
    }
}

pub fn render_save_confirmation(
    frame: &mut Frame<CrosstermBackend<io::Stdout>>,
    state: &crate::app::SaveConfirmationState,
    app: &App,
) {
    let area = frame.size();
    let popup_area = centered_rect(60, 20, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(app.locale.get("warning_title"))
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(block, popup_area);

    let inner_area = centered_rect(50, 10, popup_area);

    let text = Paragraph::new(state.message.clone())
        .style(Style::default().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    let button_text = format!("[ {} ]", app.locale.get("confirm_button"));
    let button = Paragraph::new(button_text)
        .style(Style::default().bg(Color::Green).fg(Color::White))
        .alignment(Alignment::Center);

    let inner_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(inner_area);

    frame.render_widget(text, inner_layout[0]);
    frame.render_widget(button, inner_layout[1]);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

pub fn format_json_value(value: &serde_json::Value) -> String {
    value.to_string().replace('"', "")
}

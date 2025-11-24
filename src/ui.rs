use std::io;
use tui::{
    Frame,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, TableState, Wrap},
};

use crate::app::{App, AppState};

pub fn render(frame: &mut Frame<CrosstermBackend<io::Stdout>>, app: &mut App) {
    match app.state {
        AppState::FileSelection => render_file_selection(frame, app),
        AppState::Editing => {
            if let Some(editing) = &mut app.editing {
                render_editing(frame, editing, app.locale.get("translation_title"), &app.locale)
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

pub fn render_file_selection(frame: &mut Frame<CrosstermBackend<io::Stdout>>, app: &mut App) {
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

    frame.render_stateful_widget(list, chunks[0], &mut app.file_selection.list_state);

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
    state: &mut crate::app::EditingState,
    title_template: &str,
    locale: &crate::localization::Locale,
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

    let mut title = title_template.to_string();
    let params = [
        ("translated", state.translated_keys.to_string()),
        ("total", state.total_keys.to_string()),
    ];
    for (k, v) in &params {
        title = title.replace(&format!("{{{}}}", k), v);
    }

    if state.search_mode && !state.search_query.is_empty() {
        let rows: Vec<Row> = state
            .search_results
            .iter()
            .enumerate()
            .map(|(view_index, &entry_index)| {
                let entry = &state.entries[entry_index];
                
                let key_style = if entry.is_translated {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };

                let style = if state.search_selection == Some(view_index) {
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
                locale.get("header_key"),
                locale.get("header_original"),
                locale.get("header_translated"),
            ]))
            .block(Block::default().borders(Borders::ALL).title(title))
            .widths(&[
                Constraint::Percentage(25),
                Constraint::Percentage(35),
                Constraint::Percentage(40),
            ]);

        let mut temp_state = TableState::default();
        temp_state.select(state.search_selection);
        frame.render_stateful_widget(table, chunks[0], &mut temp_state);

    } else {
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
                locale.get("header_key"),
                locale.get("header_original"),
                locale.get("header_translated"),
            ]))
            .block(Block::default().borders(Borders::ALL).title(title))
            .widths(&[
                Constraint::Percentage(25),
                Constraint::Percentage(35),
                Constraint::Percentage(40),
            ]);

        frame.render_stateful_widget(table, chunks[0], &mut state.table_state);
    };

    // --- CÁLCULO DO SCROLL HORIZONTAL ---
    
    // 1. Calculamos a largura interna da caixa (largura total - 2 caracteres das bordas)
    let inner_width = (chunks[1].width.saturating_sub(2)) as usize;

    // 2. Convertemos o input para Vec<char> para facilitar o manuseio de índices
    let chars: Vec<char> = state.input.chars().collect();
    let total_len = chars.len();

    // 3. Calculamos o offset (quantos caracteres pular do início) para manter o cursor visível
    let scroll_offset = if total_len <= inner_width {
        0
    } else {
        // Tenta manter o cursor centralizado na caixa quando o texto é longo
        let center_pos = inner_width / 2;
        let mut start = state.cursor_pos.saturating_sub(center_pos);
        
        // Se a janela passar do final do texto, alinha com o final
        if start + inner_width > total_len {
            start = total_len.saturating_sub(inner_width);
        }
        start
    };

    // 4. Criamos a string visível baseada no offset calculado
    let visible_input: String = chars
        .iter()
        .skip(scroll_offset)
        .take(inner_width)
        .collect();

    // 5. Renderiza apenas o texto visível
    let input = Paragraph::new(visible_input).block(
        Block::default()
            .borders(Borders::ALL)
            .title(locale.get("edit_value_title")),
    );
    frame.render_widget(input, chunks[1]);

    // 6. Posiciona o cursor visualmente
    // A posição X será: (início da caixa + 1 da borda) + (posição real - offset do scroll)
    let visual_cursor_x = state.cursor_pos.saturating_sub(scroll_offset);
    
    frame.set_cursor(
        chunks[1].x + 1 + visual_cursor_x as u16,
        chunks[1].y + 1,
    );

    // ------------------------------------

    let help_text = if state.editing.is_some() {
        vec![Spans::from(vec![
            Span::styled(
                locale.get("cursor_key"),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(locale.get("cursor_help")),
            Span::styled(
                locale.get("enter_key"),
                Style::default().fg(Color::Green),
            ),
            Span::raw(locale.get("confirm_help")),
            Span::styled(locale.get("esc_key"), Style::default().fg(Color::Red)),
            Span::raw(locale.get("cancel_help")),
        ])]
    } else if state.search_mode {
        vec![Spans::from(vec![
            Span::styled(
                locale.get("up_down_keys"),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(locale.get("search_navigate_help")),
            Span::styled(
                locale.get("enter_key"),
                Style::default().fg(Color::Green),
            ),
            Span::raw(locale.get("select_help")),
            Span::styled(locale.get("esc_key"), Style::default().fg(Color::Red)),
            Span::raw(locale.get("cancel_help")),
        ])]
    } else {
        vec![Spans::from(vec![
            Span::raw(locale.get("navigation_help")),
            Span::styled(
                locale.get("up_down_keys"),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(locale.get("select_help")),
            Span::styled(
                locale.get("language_key"),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(locale.get("language_help")),
            Span::styled(
                locale.get("enter_key"),
                Style::default().fg(Color::Green),
            ),
            Span::raw(locale.get("edit_help")),
            Span::styled("T", Style::default().fg(Color::Magenta)),
            Span::raw(locale.get("mark_translated_help")),
            Span::styled("B", Style::default().fg(Color::LightGreen)),
            Span::raw(locale.get("save_help")),
            Span::styled(locale.get("esc_key"), Style::default().fg(Color::Blue)),
            Span::raw(locale.get("save_return_help")),
            Span::styled("Q", Style::default().fg(Color::Red)),
            Span::raw(locale.get("save_quit_help")),
            Span::styled("S", Style::default().fg(Color::Cyan)),
            Span::raw(locale.get("search_help")),
        ])]
    };

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::TOP))
        .wrap(Wrap { trim: true });

    frame.render_widget(help, chunks[2]);

    if state.search_mode {
        let mut search_text = locale.get("search_results").to_string();
        search_text = search_text.replace("{query}", &state.search_query);
        search_text = search_text.replace("{count}", &state.search_results.len().to_string());
        
        let search_bar = Paragraph::new(search_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(locale.get("search_title")),
        );
        frame.render_widget(search_bar, chunks[3]);
    } else if state.save_notification.is_some() {
        let notification = Paragraph::new(locale.get("save_success"))
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

    frame.render_widget(
        Block::default().style(Style::default().bg(Color::DarkGray)),
        area,
    );

    let popup_area = centered_rect(50, 30, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(app.locale.get("warning_title"))
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    frame.render_widget(block, popup_area);

    let inner_area = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(popup_area);

    let text = Paragraph::new(state.message.clone())
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    frame.render_widget(text, inner_area[0]);

    let button_width = 20;
    let button_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - button_width) / 2),
            Constraint::Percentage(button_width),
            Constraint::Percentage((100 - button_width) / 2),
        ])
        .split(inner_area[2])[1];

    let button_text = format!("[ {} ]", app.locale.get("confirm_button"));
    let button = Paragraph::new(button_text)
        .style(Style::default().fg(Color::Black).bg(Color::Green))
        .alignment(Alignment::Center);

    frame.render_widget(button, button_area);
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

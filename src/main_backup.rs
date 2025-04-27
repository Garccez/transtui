use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
};
use serde_json::{Map, Value};
use std::{
    fs,
    io,
    path::{Path, PathBuf},
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{
        Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState, Wrap,
    },
    Frame, Terminal,
};

enum AppState {
    FileSelection,
    Editing,
    Exiting,
}

struct FileSelectionState {
    files: Vec<PathBuf>,
    list_state: ListState,
}

struct Entry {
    key: String,
    original: Value,
    translated: Value,
    is_translated: bool,
}

struct EditingState {
    entries: Vec<Entry>,
    table_state: TableState,
    original_path: PathBuf,
    editing: Option<usize>,
    input: String,
    cursor_pos: usize,
    search_query: String,
    search_mode: bool,
    search_results: Vec<usize>,
    search_selection: Option<usize>,
    total_keys: usize,
    translated_keys: usize,
}

fn list_json_files() -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path.extension().unwrap_or_default() == "json"
            && !path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with("_traduzido.json")
        {
            files.push(path);
        }
    }
    Ok(files)
}

fn format_json_value(value: &Value) -> String {
    value.to_string().replace('"', "")
}

fn load_translated_keys(path: &Path) -> Result<Vec<String>> {
    if path.exists() {
        let content = fs::read_to_string(path)?;
        Ok(content
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    } else {
        Ok(Vec::new())
    }
}

fn save_translated_keys(path: &Path, entries: &[Entry]) -> Result<()> {
    let translated: Vec<String> = entries
        .iter()
        .filter(|e| e.is_translated)
        .map(|e| e.key.clone())
        .collect();
    
    let content = translated.join("; ");
    fs::write(path, content)?;
    Ok(())
}

fn save_translated_json(state: &EditingState) -> Result<()> {
    let mut translated_map = Map::new();
    for entry in &state.entries {
        translated_map.insert(entry.key.clone(), entry.translated.clone());
    }

    let new_filename = format!(
        "{}_traduzido.json",
        state.original_path.file_stem().unwrap().to_str().unwrap()
    );
    let mut new_path = state.original_path.clone();
    new_path.set_file_name(new_filename);

    let json = serde_json::to_string_pretty(&translated_map)?;
    fs::write(new_path, json)?;
    Ok(())
}

fn load_existing_translations(original_path: &Path) -> Result<Map<String, Value>> {
    let translated_filename = format!(
        "{}_traduzido.json",
        original_path.file_stem().unwrap().to_str().unwrap()
    );
    let mut translated_path = original_path.to_path_buf();
    translated_path.set_file_name(translated_filename);

    if translated_path.exists() {
        let content = fs::read_to_string(&translated_path)?;
        if let Ok(Value::Object(map)) = serde_json::from_str(&content) {
            return Ok(map);
        }
    }
    
    // Return empty map if no translation exists or there was an error
    Ok(Map::new())
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_state = AppState::FileSelection;
    let mut file_state = FileSelectionState {
        files: list_json_files()?,
        list_state: ListState::default(),
    };
    file_state.list_state.select(Some(0));

    let mut editing_state: Option<EditingState> = None;

    loop {
        terminal.draw(|f| {
            match app_state {
                AppState::FileSelection => render_file_selection(f, &file_state),
                AppState::Editing => render_editing(f, editing_state.as_mut().unwrap()),
                AppState::Exiting => (),
            }
        })?;

        if let Event::Key(key) = event::read()? {
            match app_state {
                AppState::FileSelection => handle_file_selection(
                    key,
                    &mut file_state,
                    &mut app_state,
                    &mut editing_state,
                )?,
                AppState::Editing => {
                    handle_editing(key, editing_state.as_mut().unwrap(), &mut app_state)?
                }
                AppState::Exiting => break,
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        Clear(ClearType::All)
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn render_file_selection(frame: &mut Frame<CrosstermBackend<io::Stdout>>, state: &FileSelectionState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
        .split(frame.size());

    let items: Vec<ListItem> = state
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
        .block(Block::default().borders(Borders::ALL).title("Selecione um arquivo JSON"))
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black));

    frame.render_stateful_widget(list, chunks[0], &mut state.list_state.clone());

    let help = Paragraph::new(vec![
        Spans::from(vec![
            Span::raw("Navegação: "),
            Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Selecionar | "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(" Abrir | "),
            Span::styled("Q", Style::default().fg(Color::Red)),
            Span::raw(" Sair"),
        ])
    ])
    .block(Block::default().borders(Borders::TOP))
    .wrap(Wrap { trim: true });
    
    frame.render_widget(help, chunks[1]);
}

fn render_editing(frame: &mut Frame<CrosstermBackend<io::Stdout>>, state: &mut EditingState) {
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

    let title = format!("Dados (Traduzidas: {}/{})", state.translated_keys, state.total_keys);
    
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
        .header(Row::new(vec!["Chave", "Original", "Traduzido"]))
        .block(Block::default().borders(Borders::ALL).title(title))
        .widths(&[
            Constraint::Percentage(25),
            Constraint::Percentage(35),
            Constraint::Percentage(40),
        ]);

    frame.render_stateful_widget(table, chunks[0], &mut state.table_state);

    let cursor_byte_pos = state.input
        .chars()
        .take(state.cursor_pos)
        .map(|c| c.len_utf8())
        .sum::<usize>();
        
    let (left, right) = state.input.split_at(cursor_byte_pos);
    let input_display = format!("{}█{}", left, right);

    let input = Paragraph::new(input_display)
        .block(Block::default().borders(Borders::ALL).title("Editar valor traduzido (Enter para confirmar)"));
    frame.render_widget(input, chunks[1]);

    let help_text = if state.editing.is_some() {
        vec![
            Spans::from(vec![
                Span::styled("←/→", Style::default().fg(Color::Yellow)),
                Span::raw(" Mover cursor | "),
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(" Confirmar | "),
                Span::styled("Esc", Style::default().fg(Color::Red)),
                Span::raw(" Cancelar"),
            ])
        ]
    } else if state.search_mode {
        vec![
            Spans::from(vec![
                Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
                Span::raw(" Navegar | "),
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(" Selecionar | "),
                Span::styled("Esc", Style::default().fg(Color::Red)),
                Span::raw(" Cancelar"),
            ])
        ]
    } else {
        vec![
            Spans::from(vec![
                Span::raw("Navegação: "),
                Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
                Span::raw(" Selecionar | "),
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(" Editar | "),
                Span::styled("T", Style::default().fg(Color::Magenta)),
                Span::raw(" Marcar tradução | "),
                Span::styled("Esc", Style::default().fg(Color::Blue)),
                Span::raw(" Salvar e Voltar | "),
                Span::styled("Q", Style::default().fg(Color::Red)),
                Span::raw(" Salvar e Sair | "),
                Span::styled("S", Style::default().fg(Color::Cyan)),
                Span::raw(" Pesquisar"),
            ])
        ]
    };

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::TOP))
        .wrap(Wrap { trim: true });
    
    frame.render_widget(help, chunks[2]);

    if state.search_mode {
        let search_display = format!("🔍 {} ({} resultados)", state.search_query, state.search_results.len());
        let search_bar = Paragraph::new(search_display)
            .block(Block::default().borders(Borders::ALL).title("Pesquisar chaves"));
        frame.render_widget(search_bar, chunks[3]);
    }
}

fn handle_file_selection(
    key: event::KeyEvent,
    state: &mut FileSelectionState,
    app_state: &mut AppState,
    editing_state: &mut Option<EditingState>,
) -> Result<()> {
    match key.code {
        KeyCode::Char('q') => *app_state = AppState::Exiting,
        KeyCode::Up => {
            if let Some(selected) = state.list_state.selected() {
                let new_selected = selected.saturating_sub(1);
                state.list_state.select(Some(new_selected));
            }
        }
        KeyCode::Down => {
            if let Some(selected) = state.list_state.selected() {
                let new_selected = selected + 1;
                if new_selected < state.files.len() {
                    state.list_state.select(Some(new_selected));
                }
            }
        }
        KeyCode::Enter => {
            if let Some(selected) = state.list_state.selected() {
                let file_path = &state.files[selected];
                let content = fs::read_to_string(file_path)?;
                let data: Value = serde_json::from_str(&content)?;

                if let Value::Object(original_map) = data {
                    // Carregar traduções existentes
                    let existing_translations = load_existing_translations(file_path)?;
                    
                    let txt_path = file_path.with_extension("txt");
                    let translated_keys = load_translated_keys(&txt_path)?;
                    
                    let mut translated_count = 0;
                    let entries: Vec<_> = original_map
                        .into_iter()
                        .map(|(key, original_value)| {
                            let is_translated = translated_keys.contains(&key);
                            if is_translated {
                                translated_count += 1;
                            }
                            
                            // Usar a tradução existente se disponível, caso contrário usar o valor original
                            let translated = if let Some(trans) = existing_translations.get(&key) {
                                trans.clone()
                            } else {
                                original_value.clone()
                            };
                            
                            Entry {
                                key: key.clone(),
                                original: original_value,
                                translated,
                                is_translated,
                            }
                        })
                        .collect();

                    let total_keys = entries.len();
                    
                    let mut table_state = TableState::default();
                    table_state.select(Some(0));

                    *editing_state = Some(EditingState {
                        entries,
                        table_state,
                        original_path: file_path.clone(),
                        editing: None,
                        input: String::new(),
                        cursor_pos: 0,
                        search_query: String::new(),
                        search_mode: false,
                        search_results: Vec::new(),
                        search_selection: None,
                        total_keys,
                        translated_keys: translated_count,
                    });
                    *app_state = AppState::Editing;
                }
            }
        }
        KeyCode::Esc => *app_state = AppState::Exiting,
        _ => {}
    }
    Ok(())
}

fn handle_editing(
    key: event::KeyEvent,
    state: &mut EditingState,
    app_state: &mut AppState,
) -> Result<()> {
    if state.search_mode {
        match key.code {
            KeyCode::Enter => {
                if let Some(selected) = state.search_selection {
                    if let Some(&entry_index) = state.search_results.get(selected) {
                        state.table_state.select(Some(entry_index));
                    }
                }
                state.search_mode = false;
                state.search_query.clear();
                state.search_results.clear();
                state.search_selection = None;
            }
            KeyCode::Esc => {
                state.search_mode = false;
                state.search_query.clear();
                state.search_results.clear();
                state.search_selection = None;
            }
            KeyCode::Up => {
                if !state.search_results.is_empty() {
                    let new_selection = match state.search_selection {
                        Some(current) if current > 0 => Some(current - 1),
                        None => Some(state.search_results.len() - 1),
                        _ => None,
                    };
                    state.search_selection = new_selection;
                }
            }
            KeyCode::Down => {
                if !state.search_results.is_empty() {
                    let new_selection = match state.search_selection {
                        Some(current) if current < state.search_results.len() - 1 => Some(current + 1),
                        None => Some(0),
                        _ => None,
                    };
                    state.search_selection = new_selection;
                }
            }
            KeyCode::Char(c) => {
                state.search_query.push(c);
                update_search_results(state);
            }
            KeyCode::Backspace => {
                state.search_query.pop();
                update_search_results(state);
            }
            _ => {}
        }
        return Ok(());
    }

    if let Some(editing_index) = state.editing {
        match key.code {
            KeyCode::Enter => {
                if let Some(entry) = state.entries.get_mut(editing_index) {
                    let value = if state.input.is_empty() {
                        Value::String("".to_string())
                    } else {
                        Value::String(state.input.clone())
                    };
                    entry.translated = value;
                }
                state.editing = None;
                state.input.clear();
                state.cursor_pos = 0;
            }
            KeyCode::Esc => {
                state.editing = None;
                state.input.clear();
                state.cursor_pos = 0;
            }
            KeyCode::Left => {
                if state.cursor_pos > 0 {
                    state.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if state.cursor_pos < state.input.chars().count() {
                    state.cursor_pos += 1;
                }
            }
            KeyCode::Char(c) => {
                let byte_pos: usize = state.input
                    .chars()
                    .take(state.cursor_pos)
                    .map(|c| c.len_utf8())
                    .sum();
                state.input.insert(byte_pos, c);
                state.cursor_pos += 1;
            }
            KeyCode::Backspace => {
                if state.cursor_pos > 0 {
                    let byte_start: usize = state.input
                        .chars()
                        .take(state.cursor_pos - 1)
                        .map(|c| c.len_utf8())
                        .sum();
                    let byte_end: usize = byte_start + state.input
                        .chars()
                        .nth(state.cursor_pos - 1)
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    
                    state.input.drain(byte_start..byte_end);
                    state.cursor_pos -= 1;
                }
            }
            KeyCode::Delete => {
                if state.cursor_pos < state.input.chars().count() {
                    let byte_start: usize = state.input
                        .chars()
                        .take(state.cursor_pos)
                        .map(|c| c.len_utf8())
                        .sum();
                    let byte_end: usize = byte_start + state.input
                        .chars()
                        .nth(state.cursor_pos)
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    
                    state.input.drain(byte_start..byte_end);
                }
            }
            KeyCode::Home => state.cursor_pos = 0,
            KeyCode::End => state.cursor_pos = state.input.chars().count(),
            KeyCode::Char('q') => {
                save_translated_json(state)?;
                *app_state = AppState::Exiting;
            }
            _ => {}
        }
    } else {
        match key.code {
            KeyCode::Char('t') => {
                if let Some(selected) = state.table_state.selected() {
                    if let Some(entry) = state.entries.get_mut(selected) {
                        entry.is_translated = !entry.is_translated;
                        if entry.is_translated {
                            state.translated_keys += 1;
                        } else {
                            state.translated_keys -= 1;
                        }
                        
                        let txt_path = state.original_path.with_extension("txt");
                        save_translated_keys(&txt_path, &state.entries)?;
                    }
                }
            }
            KeyCode::Char('s') => {
                state.search_mode = true;
                state.search_query.clear();
                update_search_results(state);
            }
            KeyCode::Char('q') => {
                save_translated_json(state)?;
                *app_state = AppState::Exiting;
            }
            KeyCode::Up => {
                let selected = state.table_state.selected().unwrap_or(0);
                let new_selected = selected.saturating_sub(1);
                state.table_state.select(Some(new_selected));
            }
            KeyCode::Down => {
                let selected = state.table_state.selected().unwrap_or(0);
                let new_selected = selected + 1;
                if new_selected < state.entries.len() {
                    state.table_state.select(Some(new_selected));
                }
            }
            KeyCode::Enter => {
                if let Some(selected) = state.table_state.selected() {
                    state.editing = Some(selected);
                    state.input = format_json_value(&state.entries[selected].translated);
                    state.cursor_pos = state.input.chars().count();
                }
            }
            KeyCode::Esc => {
                save_translated_json(state)?;
                *app_state = AppState::FileSelection;
            }
            _ => {}
        }
    }
    Ok(())
}

fn update_search_results(state: &mut EditingState) {
    let search_lower = state.search_query.to_lowercase();
    state.search_results = state.entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.key.to_lowercase().contains(&search_lower))
        .map(|(i, _)| i)
        .collect();
    
    state.search_selection = if !state.search_results.is_empty() {
        Some(0)
    } else {
        None
    };
}

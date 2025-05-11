use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use serde_json::Value;
use std::fs;

use crate::app::{App, AppState};
use crate::file_operations;
use crate::ui::format_json_value;

pub fn handle_events(app: &mut App, key: KeyEvent) -> Result<()> {
    match app.state {
        AppState::FileSelection => handle_file_selection(app, key),
        AppState::Editing => handle_editing(app, key),
        AppState::SaveConfirmation => handle_save_confirmation(app, key),
        AppState::Exiting => Ok(()),
    }
}

fn handle_file_selection(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => app.state = AppState::Exiting,
        KeyCode::Up => {
            if let Some(selected) = app.file_selection.list_state.selected() {
                let new_selected = selected.saturating_sub(1);
                app.file_selection.list_state.select(Some(new_selected));
            }
        }
        KeyCode::Down => {
            if let Some(selected) = app.file_selection.list_state.selected() {
                let new_selected = selected + 1;
                if new_selected < app.file_selection.files.len() {
                    app.file_selection.list_state.select(Some(new_selected));
                }
            }
        }
        KeyCode::Enter => {
            if let Some(file_path) = app.get_selected_file_path() {
                let content = fs::read_to_string(file_path)?;
                let data: Value = serde_json::from_str(&content)?;

                if let Value::Object(original_map) = data {
                    let existing_translations = file_operations::load_existing_translations(
                        file_path,
                        app.locale.get("translations_folder"),
                        app.locale.get("translation_suffix"),
                    )?;

                    let toml_path = file_path.with_extension("toml");
                    let translated_keys = file_operations::load_translated_keys(&toml_path)?;

                    let mut translated_count = 0;
                    let entries = original_map
                        .clone()
                        .into_iter()
                        .map(|(key, original_value)| {
                            let is_translated = translated_keys.contains(&key);
                            if is_translated {
                                translated_count += 1;
                            }

                            let translated = if let Some(trans) = existing_translations.get(&key) {
                                trans.clone()
                            } else {
                                original_value.clone()
                            };

                            crate::app::Entry {
                                key: key.clone(),
                                original: original_value,
                                translated,
                                is_translated,
                            }
                        })
                        .collect();

                    let total_keys = original_map.len();

                    let mut table_state = tui::widgets::TableState::default();
                    table_state.select(Some(0));

                    app.editing = Some(crate::app::EditingState {
                        entries,
                        table_state,
                        original_path: file_path.to_path_buf(),
                        editing: None,
                        input: String::new(),
                        cursor_pos: 0,
                        search_query: String::new(),
                        search_mode: false,
                        search_results: Vec::new(),
                        search_selection: None,
                        total_keys,
                        translated_keys: translated_count,
                        save_notification: None,
                    });
                    app.state = AppState::Editing;
                }
            }
        }
        KeyCode::F(2) => {
            app.switch_language()?;
        }
        KeyCode::Esc => app.state = AppState::Exiting,
        _ => {}
    }
    Ok(())
}

fn handle_editing(app: &mut App, key: KeyEvent) -> Result<()> {
    if let Some(state) = &mut app.editing {
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
                            Some(current) if current < state.search_results.len() - 1 => {
                                Some(current + 1)
                            }
                            None => Some(0),
                            _ => None,
                        };
                        state.search_selection = new_selection;
                    }
                }
                KeyCode::Char(c) => {
                    state.search_query.push(c);
                    app.update_search_results();
                }
                KeyCode::Backspace => {
                    state.search_query.pop();
                    app.update_search_results();
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
                    let byte_pos: usize = state
                        .input
                        .chars()
                        .take(state.cursor_pos)
                        .map(|c| c.len_utf8())
                        .sum();
                    state.input.insert(byte_pos, c);
                    state.cursor_pos += 1;
                }
                KeyCode::Backspace => {
                    if state.cursor_pos > 0 {
                        let byte_start: usize = state
                            .input
                            .chars()
                            .take(state.cursor_pos - 1)
                            .map(|c| c.len_utf8())
                            .sum();
                        let byte_end: usize = byte_start
                            + state
                                .input
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
                        let byte_start: usize = state
                            .input
                            .chars()
                            .take(state.cursor_pos)
                            .map(|c| c.len_utf8())
                            .sum();
                        let byte_end: usize = byte_start
                            + state
                                .input
                                .chars()
                                .nth(state.cursor_pos)
                                .map(|c| c.len_utf8())
                                .unwrap_or(0);

                        state.input.drain(byte_start..byte_end);
                    }
                }
                KeyCode::Home => state.cursor_pos = 0,
                KeyCode::End => state.cursor_pos = state.input.chars().count(),
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    app.toggle_translation()?;
                }
                KeyCode::Char('b') | KeyCode::Char('B') => {
                    app.save_current_file()?;
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    state.search_mode = true;
                    state.search_query.clear();
                    app.update_search_results();
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    app.save_confirmation = Some(crate::app::SaveConfirmationState {
                        message: app.locale.get("save_exit_confirmation").to_string(),
                        return_to: AppState::Editing,
                    });
                    app.state = AppState::SaveConfirmation;
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
                KeyCode::F(2) => {
                    app.switch_language()?;
                }
                KeyCode::Esc => {
                    app.save_confirmation = Some(crate::app::SaveConfirmationState {
                        message: app.locale.get("save_return_confirmation").to_string(),
                        return_to: AppState::Editing,
                    });
                    app.state = AppState::SaveConfirmation;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn handle_save_confirmation(app: &mut App, key: KeyEvent) -> Result<()> {
    let should_exit;
    let return_to;

    if let Some(confirmation) = &app.save_confirmation {
        should_exit = confirmation.message == app.locale.get("save_exit_confirmation");
        return_to = confirmation.return_to.clone();
    } else {
        return Ok(());
    }

    match key.code {
        KeyCode::Enter | KeyCode::Char(' ') => {
            if return_to == AppState::Editing {
                app.save_current_file()?;

                if should_exit {
                    app.state = AppState::Exiting;
                } else {
                    app.state = AppState::FileSelection;
                }
            } else {
                app.state = return_to;
            }
        }
        KeyCode::Esc => {
            app.state = return_to;
        }
        _ => {}
    }

    Ok(())
}

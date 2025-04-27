use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tui::widgets::{ListState, TableState};

use crate::file_operations;

#[derive(Clone, PartialEq)]
pub enum AppState {
    FileSelection,
    Editing,
	SaveConfirmation,
    Exiting,
}

pub struct FileSelectionState {
    pub files: Vec<PathBuf>,
    pub list_state: ListState,
}

pub struct Entry {
    pub key: String,
    pub original: Value,
    pub translated: Value,
    pub is_translated: bool,
}

pub struct EditingState {
    pub entries: Vec<Entry>,
    pub table_state: TableState,
    pub original_path: PathBuf,
    pub editing: Option<usize>,
    pub input: String,
    pub cursor_pos: usize,
    pub search_query: String,
    pub search_mode: bool,
    pub search_results: Vec<usize>,
    pub search_selection: Option<usize>,
    pub total_keys: usize,
    pub translated_keys: usize,
    pub save_notification: Option<Instant>,
}

pub struct SaveConfirmationState {
    pub message: String,
    pub return_to: AppState,
}

#[derive(Serialize, Deserialize)]
pub struct TranslatedKeysData {
    pub keys: Vec<String>,
    pub last_updated: String,
}

pub struct App {
    pub state: AppState,
    pub file_selection: FileSelectionState,
    pub editing: Option<EditingState>,
    pub save_confirmation: Option<SaveConfirmationState>,
}

impl App {
    pub fn new() -> Result<Self> {
        let files = file_operations::list_json_files()?;
        let mut list_state = ListState::default();
        if !files.is_empty() {
            list_state.select(Some(0));
        }

        Ok(Self {
            state: AppState::FileSelection,
            file_selection: FileSelectionState {
                files,
                list_state,
            },
            editing: None,
            save_confirmation: None,
        })
    }

    pub fn check_notification_timeout(&mut self) {
        if let Some(editing) = &mut self.editing {
            if let Some(time) = editing.save_notification {
                if time.elapsed() > Duration::from_secs(2) {
                    editing.save_notification = None;
                }
            }
        }
    }

    pub fn update_search_results(&mut self) {
        if let Some(state) = &mut self.editing {
            let search_lower = state.search_query.to_lowercase();
            state.search_results = state
                .entries
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
    }

    pub fn toggle_translation(&mut self) -> Result<()> {
        if let Some(state) = &mut self.editing {
            if let Some(selected) = state.table_state.selected() {
                if let Some(entry) = state.entries.get_mut(selected) {
                    entry.is_translated = !entry.is_translated;
                    if entry.is_translated {
                        state.translated_keys += 1;
                    } else {
                        state.translated_keys -= 1;
                    }

                    let toml_path = state.original_path.with_extension("toml");
                    file_operations::save_translated_keys(&toml_path, &state.entries)?;
                }
            }
        }
        Ok(())
    }

    pub fn save_current_file(&mut self) -> Result<()> {
        if let Some(state) = &mut self.editing {
            file_operations::save_translated_json(state)?;
            state.save_notification = Some(Instant::now());
        }
        Ok(())
    }

    pub fn get_selected_file_path(&self) -> Option<&Path> {
        self.file_selection
            .list_state
            .selected()
            .map(|selected| &self.file_selection.files[selected]).map(|v| &**v)
    }
}

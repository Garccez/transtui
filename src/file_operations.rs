use anyhow::Result;
use chrono::Local;
use serde_json::{Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};
use toml;

use crate::app::{EditingState, Entry, TranslatedKeysData};

pub fn list_json_files() -> Result<Vec<PathBuf>> {
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

pub fn load_translated_keys(path: &Path) -> Result<Vec<String>> {
    if path.exists() {
        let content = fs::read_to_string(path)?;
        
        // Verificar o formato do arquivo
        if path.extension().unwrap_or_default() == "toml" {
            // Parse TOML
            let data: TranslatedKeysData = toml::from_str(&content)?;
            Ok(data.keys)
        } else {
            // Formato antigo (TXT)
            Ok(content
                .split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect())
        }
    } else {
        Ok(Vec::new())
    }
}

pub fn save_translated_keys(path: &Path, entries: &[Entry]) -> Result<()> {
    let translated: Vec<String> = entries
        .iter()
        .filter(|e| e.is_translated)
        .map(|e| e.key.clone())
        .collect();
    
    // Criar estrutura de dados para salvar
    let data = TranslatedKeysData {
        keys: translated,
        last_updated: Local::now().to_rfc3339(),
    };
    
    // Serializar para TOML
    let content = toml::to_string(&data)?;
    fs::write(path, content)?;
    
    // Se estiver migrando de TXT para TOML, remover o arquivo TXT antigo
    let txt_path = path.with_extension("txt");
    if txt_path.exists() {
        fs::remove_file(txt_path)?;
    }
    
    Ok(())
}

pub fn save_translated_json(state: &EditingState) -> Result<()> {
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
    
    // Atualizar também o arquivo de chaves traduzidas
    let toml_path = state.original_path.with_extension("toml");
    save_translated_keys(&toml_path, &state.entries)?;
    
    Ok(())
}

pub fn load_existing_translations(original_path: &Path) -> Result<Map<String, Value>> {
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
    
    // Retornar mapa vazio se não existir tradução ou houver erro
    Ok(Map::new())
}

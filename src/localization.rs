use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

// Arquivos de tradução embutidos no binário
const EN_TRANSLATIONS: &str = include_str!("../locales/en/app.toml");
const PT_TRANSLATIONS: &str = include_str!("../locales/pt/app.toml");

#[derive(Debug, Deserialize)]
pub struct Locale {
    pub ui: HashMap<String, String>,
}

impl Locale {
    pub fn from_language(lang: crate::app::Language) -> Result<Self> {
        let content = match lang {
            crate::app::Language::EN => EN_TRANSLATIONS,
            crate::app::Language::PT => PT_TRANSLATIONS,
        };
        
        Ok(toml::from_str(content)?)
    }
	
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.ui.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    pub fn get_with_params(&self, key: &str, params: &[(&str, &str)]) -> String {
        let mut text = self.get(key).to_string();
        for (k, v) in params {
            text = text.replace(&format!("{{{}}}", k), v);
        }
        text
    }
}
